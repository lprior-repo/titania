mod code_scan;

use code_scan::code_only_source;

use super::super::span::MatchSite;

pub(super) fn detect_core_infra_import(source: &str) -> Vec<MatchSite> {
    let code = code_only_source(source);
    merge_sites(
        infra_use_sites(&code, &["tokio", "axum", "sqlx", "reqwest"])
            .into_iter()
            .chain(fully_qualified_sites(&code, &["tokio::", "axum::", "sqlx::", "reqwest::"]))
            .chain(crate_alias_sites(&code, &["tokio", "axum", "sqlx", "reqwest"]))
            .collect(),
    )
}

pub(super) fn detect_core_fs_import(source: &str) -> Vec<MatchSite> {
    let code = code_only_source(source);
    merge_sites(
        direct_import_sites(&code, "std::", &["fs", "env", "net"])
            .into_iter()
            .chain(grouped_import_sites(&code, "use std::", &["fs", "env", "net"]))
            .chain(fully_qualified_sites(&code, &["std::fs::", "std::env::", "std::net::"]))
            .collect(),
    )
}

pub(super) fn detect_core_time_import(source: &str) -> Vec<MatchSite> {
    let code = code_only_source(source);
    merge_sites(
        direct_import_sites(&code, "std::time::", &["SystemTime", "Instant"])
            .into_iter()
            .chain(grouped_import_sites(&code, "use std::time::", &["SystemTime", "Instant"]))
            .chain(grouped_import_sites(&code, "use std::", &["time::SystemTime", "time::Instant"]))
            .chain(fully_qualified_sites(&code, &["std::time::SystemTime", "std::time::Instant"]))
            .chain(module_binding_sites(&code, "time", &["SystemTime", "Instant"]))
            .collect(),
    )
}

pub(super) fn detect_core_random_import(source: &str) -> Vec<MatchSite> {
    let code = code_only_source(source);
    merge_sites(
        direct_import_sites(&code, "rand::", &["thread_rng"])
            .into_iter()
            .chain(grouped_import_sites(&code, "use rand::", &["thread_rng"]))
            .chain(fully_qualified_sites(&code, &["rand::thread_rng"]))
            .collect(),
    )
}

pub(super) fn detect_core_process_import(source: &str) -> Vec<MatchSite> {
    let code = code_only_source(source);
    merge_sites(
        direct_import_sites(&code, "std::", &["process"])
            .into_iter()
            .chain(grouped_import_sites(&code, "use std::", &["process"]))
            .chain(fully_qualified_sites(
                &code,
                &[
                    "std::process::",
                    "tokio::process",
                    "std::os::unix::process",
                    "std::os::windows::process",
                ],
            ))
            .collect(),
    )
}

pub(super) fn detect_core_async_effect(source: &str) -> Vec<MatchSite> {
    let code = code_only_source(source);
    merge_sites(
        token_sites(&code, "async fn")
            .into_iter()
            .chain(token_sites(&code, "async unsafe fn"))
            .chain(await_sites(&code))
            .chain(fully_qualified_sites(&code, &["tokio::spawn", "tokio::time"]))
            .chain(grouped_import_sites(&code, "use tokio::", &["task::spawn"]))
            .chain(tokio_task_sites(&code))
            .chain(tokio_alias_effect_sites(&code))
            .collect(),
    )
}

pub(super) fn detect_core_state_effect(source: &str) -> Vec<MatchSite> {
    let code = code_only_source(source);
    merge_sites(
        token_sites(&code, "static mut")
            .into_iter()
            .chain(fully_qualified_sites(
                &code,
                &[
                    "std::sync::Mutex",
                    "std::sync::RwLock",
                    "std::cell::RefCell",
                    "std::thread::spawn",
                ],
            ))
            .chain(direct_import_sites(&code, "std::sync::", &["Mutex", "RwLock"]))
            .chain(grouped_import_sites(&code, "use std::sync::", &["Mutex", "RwLock"]))
            .chain(grouped_import_sites(&code, "use std::", &["sync::Mutex", "sync::RwLock"]))
            .chain(direct_import_sites(&code, "std::cell::", &["RefCell"]))
            .chain(grouped_import_sites(&code, "use std::cell::", &["RefCell"]))
            .chain(grouped_import_sites(&code, "use std::", &["cell::RefCell"]))
            .chain(direct_import_sites(&code, "std::thread::", &["spawn"]))
            .chain(grouped_import_sites(&code, "use std::thread::", &["spawn"]))
            .chain(grouped_import_sites(&code, "use std::", &["thread::spawn"]))
            .chain(module_binding_sites(&code, "sync", &["Mutex", "RwLock"]))
            .chain(module_binding_sites(&code, "cell", &["RefCell"]))
            .chain(module_binding_sites(&code, "thread", &["spawn"]))
            .collect(),
    )
}

fn merge_sites(sites: Vec<MatchSite>) -> Vec<MatchSite> {
    let mut sorted = sites;
    sorted.sort_by_key(|site| (site.start_byte, std::cmp::Reverse(site.end_byte)));
    sorted.into_iter().fold(Vec::new(), push_if_unoverlapped)
}

fn push_if_unoverlapped(mut kept: Vec<MatchSite>, site: MatchSite) -> Vec<MatchSite> {
    if kept.iter().all(|prior| !sites_overlap(*prior, site)) {
        kept.push(site);
    }
    kept
}

const fn sites_overlap(first: MatchSite, second: MatchSite) -> bool {
    first.start_byte < second.end_byte && second.start_byte < first.end_byte
}

fn site_at(start: usize, len: usize) -> Option<MatchSite> {
    start.checked_add(len).map(|end| MatchSite::from_range(start..end))
}

fn token_sites(code: &str, needle: &str) -> Vec<MatchSite> {
    code.match_indices(needle)
        .filter(|(start, _)| {
            leading_boundary_ok(code, *start) && trailing_boundary_ok(code, *start, needle)
        })
        .filter_map(|(start, _)| site_at(start, needle.len()))
        .collect()
}

fn fully_qualified_sites(code: &str, needles: &[&str]) -> Vec<MatchSite> {
    needles
        .iter()
        .flat_map(|needle| {
            let segments = split_path(needle);
            path_sites(code, &segments, needle.ends_with("::"), root_boundary_ok)
        })
        .collect()
}

fn path_sites(
    code: &str,
    segments: &[&str],
    trailing_colons: bool,
    boundary: fn(&str, usize) -> bool,
) -> Vec<MatchSite> {
    let Some((first, rest)) = segments.split_first() else {
        return Vec::new();
    };
    code.match_indices(first)
        .filter(|(start, _)| boundary(code, *start) && trailing_boundary_ok(code, *start, first))
        .filter_map(|(start, _)| extend_path_site(code, start, first, rest, trailing_colons))
        .collect()
}

fn extend_path_site(
    code: &str,
    start: usize,
    first: &str,
    rest: &[&str],
    trailing_colons: bool,
) -> Option<MatchSite> {
    let mut end = start.checked_add(first.len())?;
    for segment in rest {
        end = expect_colons(code, end)?;
        end = expect_segment(code, end, segment)?;
    }
    if trailing_colons {
        end = expect_colons(code, end)?;
    }
    site_at(start, end.checked_sub(start)?)
}

fn expect_colons(code: &str, from: usize) -> Option<usize> {
    let pos = skip_whitespace(code, from);
    let (_, rest) = code.split_at_checked(pos)?;
    if rest.starts_with("::") { pos.checked_add(2) } else { None }
}

fn expect_segment(code: &str, from: usize, segment: &str) -> Option<usize> {
    let pos = skip_whitespace(code, from);
    let (_, rest) = code.split_at_checked(pos)?;
    if rest.starts_with(segment) && trailing_boundary_ok(code, pos, segment) {
        pos.checked_add(segment.len())
    } else {
        None
    }
}

fn skip_whitespace(code: &str, from: usize) -> usize {
    code.split_at_checked(from).map_or(from, |(_, rest)| {
        from.saturating_add(rest.len().saturating_sub(rest.trim_start().len()))
    })
}

fn root_boundary_ok(code: &str, start: usize) -> bool {
    let Some((before, _)) = code.split_at_checked(start) else {
        return false;
    };
    if before.chars().next_back().is_some_and(is_ident_char) {
        return false;
    }
    let mut significant = before.chars().rev().filter(|ch| !ch.is_whitespace());
    match significant.next() {
        Some(':') => colon_prefix_root_ok(&mut significant),
        _ => true,
    }
}

fn colon_prefix_root_ok(significant: &mut impl Iterator<Item = char>) -> bool {
    match significant.next() {
        Some(':') => {
            significant.next().is_none_or(|earlier| !is_ident_char(earlier) && earlier != ':')
        }
        _ => true,
    }
}

fn path_head_boundary_ok(code: &str, start: usize) -> bool {
    let Some((before, _)) = code.split_at_checked(start) else {
        return false;
    };
    if before.chars().next_back().is_some_and(is_ident_char) {
        return false;
    }
    before.chars().rev().find(|ch| !ch.is_whitespace()).is_none_or(|prev| prev != ':')
}

fn leading_boundary_ok(code: &str, start: usize) -> bool {
    code.split_at_checked(start).is_some_and(|(before, _)| {
        before.chars().next_back().is_none_or(|prev| !is_ident_char(prev))
    })
}

fn trailing_boundary_ok(code: &str, start: usize, needle: &str) -> bool {
    needle.ends_with("::")
        || code
            .split_at_checked(start.saturating_add(needle.len()))
            .is_none_or(|(_, after)| after.chars().next().is_none_or(|next| !is_ident_char(next)))
}

fn is_ident_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn split_path(path: &str) -> Vec<&str> {
    path.split("::").filter(|segment| !segment.is_empty()).collect()
}

fn segment_prefix_matches(code: &str, segments: &[Tok], name: &[&str]) -> bool {
    name.len() <= segments.len()
        && segments.iter().zip(name.iter()).all(|(tok, segment)| tok_text(code, *tok) == *segment)
}

fn segments_match(code: &str, segments: &[Tok], expected: &[&str]) -> bool {
    segments.len() == expected.len() && segment_prefix_matches(code, segments, expected)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokKind {
    Ident,
    Colons,
    Open,
    Close,
    Comma,
    Semi,
    Star,
}

#[derive(Debug, Clone, Copy)]
struct Tok {
    kind: TokKind,
    start: usize,
    end: usize,
}

fn lex_tokens(code: &str) -> Vec<Tok> {
    code.char_indices()
        .fold(LexState::default(), |state, (index, ch)| state.accept(index, ch))
        .finish()
}

#[derive(Default)]
struct LexState {
    tokens: Vec<Tok>,
    pending: Option<Tok>,
}

impl LexState {
    fn accept(self, index: usize, ch: char) -> Self {
        let end = index.saturating_add(ch.len_utf8());
        match token_char_kind(ch) {
            Some(kind) if self.continues_pending(kind) => self.extend_pending(end),
            Some(kind) => self.start_pending(kind, index, end),
            None => self.break_pending(),
        }
    }

    fn extend_pending(mut self, end: usize) -> Self {
        self.pending = self.pending.map(|tok| Tok { end, ..tok });
        self
    }

    fn start_pending(mut self, kind: TokKind, start: usize, end: usize) -> Self {
        self.flush();
        self.pending = Some(Tok { kind, start, end });
        self
    }

    fn break_pending(mut self) -> Self {
        self.flush();
        self
    }

    fn continues_pending(&self, kind: TokKind) -> bool {
        self.pending
            .is_some_and(|tok| tok.kind == kind && matches!(kind, TokKind::Ident | TokKind::Colons))
    }

    fn flush(&mut self) {
        self.tokens.extend(self.pending.take());
    }

    fn finish(mut self) -> Vec<Tok> {
        self.flush();
        self.tokens
    }
}

fn token_char_kind(ch: char) -> Option<TokKind> {
    match ch {
        '{' => Some(TokKind::Open),
        '}' => Some(TokKind::Close),
        ',' => Some(TokKind::Comma),
        ';' => Some(TokKind::Semi),
        '*' => Some(TokKind::Star),
        ':' => Some(TokKind::Colons),
        _ if is_ident_char(ch) => Some(TokKind::Ident),
        _ => None,
    }
}

fn tok_text(code: &str, tok: Tok) -> &str {
    statement_text(code, tok.start, tok.end).map_or("", |text| text)
}

fn is_path_colons(tok: Tok) -> bool {
    tok.kind == TokKind::Colons && tok.end.saturating_sub(tok.start) == 2
}

#[derive(Debug, Clone)]
struct UseTree {
    segments: Vec<Tok>,
    alias: Option<Tok>,
    group: Option<Vec<Self>>,
    glob: bool,
}

struct UseStatement {
    tree: UseTree,
}

fn use_statements(code: &str) -> Vec<UseStatement> {
    let tokens = lex_tokens(code);
    tokens
        .iter()
        .enumerate()
        .filter(|(_, tok)| tok.kind == TokKind::Ident && tok_text(code, **tok) == "use")
        .filter(|(_, tok)| use_item_prefix_allowed(code, tok.start))
        .filter_map(|(index, _)| {
            parse_use_statement(code, &tokens, index).map(|tree| UseStatement { tree })
        })
        .collect()
}

fn parse_use_statement(code: &str, tokens: &[Tok], use_index: usize) -> Option<UseTree> {
    let (tree, next) = parse_use_tree(code, tokens, use_index.checked_add(1)?)?;
    let semi = tokens.get(next)?;
    (semi.kind == TokKind::Semi).then_some(tree)
}

fn parse_use_tree(code: &str, tokens: &[Tok], index: usize) -> Option<(UseTree, usize)> {
    let first = tokens.get(index)?;
    (first.kind == TokKind::Ident).then_some(())?;
    let tree = UseTree { segments: vec![*first], alias: None, group: None, glob: false };
    parse_use_tree_tail(code, tokens, index.checked_add(1)?, tree)
}

fn parse_use_tree_tail(
    code: &str,
    tokens: &[Tok],
    index: usize,
    tree: UseTree,
) -> Option<(UseTree, usize)> {
    let Some(tok) = tokens.get(index) else {
        return Some((tree, index));
    };
    match tok.kind {
        TokKind::Colons if is_path_colons(*tok) => parse_after_colons(code, tokens, index, tree),
        TokKind::Ident if tok_text(code, *tok) == "as" => parse_alias(tokens, index, tree),
        _ => Some((tree, index)),
    }
}

fn parse_after_colons(
    code: &str,
    tokens: &[Tok],
    index: usize,
    mut tree: UseTree,
) -> Option<(UseTree, usize)> {
    let next = tokens.get(index.checked_add(1)?)?;
    match next.kind {
        TokKind::Open => parse_use_group(code, tokens, index.checked_add(2)?, tree),
        TokKind::Star => {
            tree.glob = true;
            Some((tree, index.checked_add(2)?))
        }
        TokKind::Ident => {
            tree.segments.push(*next);
            parse_use_tree_tail(code, tokens, index.checked_add(2)?, tree)
        }
        _ => None,
    }
}

fn parse_use_group(
    code: &str,
    tokens: &[Tok],
    index: usize,
    mut tree: UseTree,
) -> Option<(UseTree, usize)> {
    let (members, next) = parse_use_members(code, tokens, index)?;
    tree.group = Some(members);
    Some((tree, next))
}

type ParsedMembers = Option<(Vec<UseTree>, usize)>;

fn parse_use_members(code: &str, tokens: &[Tok], index: usize) -> ParsedMembers {
    match tokens.get(index) {
        Some(tok) if tok.kind == TokKind::Close => Some((Vec::new(), index.checked_add(1)?)),
        _ => parse_use_member_then_rest(code, tokens, index),
    }
}

fn parse_use_member_then_rest(code: &str, tokens: &[Tok], index: usize) -> ParsedMembers {
    let (member, next) = parse_use_tree(code, tokens, index)?;
    match tokens.get(next) {
        Some(tok) if tok.kind == TokKind::Close => Some((vec![member], next.checked_add(1)?)),
        Some(tok) if tok.kind == TokKind::Comma => {
            let (mut rest, after) = parse_use_members(code, tokens, next.checked_add(1)?)?;
            let mut members = vec![member];
            members.append(&mut rest);
            Some((members, after))
        }
        _ => None,
    }
}

fn parse_alias(tokens: &[Tok], index: usize, mut tree: UseTree) -> Option<(UseTree, usize)> {
    let alias = tokens.get(index.checked_add(1)?)?;
    (alias.kind == TokKind::Ident).then_some(())?;
    tree.alias = Some(*alias);
    Some((tree, index.checked_add(2)?))
}

struct FlatPath {
    segments: Vec<Tok>,
    alias: Option<Tok>,
    glob: bool,
}

fn flatten_tree(tree: &UseTree) -> Vec<FlatPath> {
    tree.group.as_ref().map_or_else(
        || vec![FlatPath { segments: tree.segments.clone(), alias: tree.alias, glob: tree.glob }],
        |members| {
            members
                .iter()
                .flat_map(flatten_tree)
                .map(|path| prepend_segments(&tree.segments, path))
                .collect()
        },
    )
}

fn prepend_segments(prefix: &[Tok], mut path: FlatPath) -> FlatPath {
    let mut segments = prefix.to_vec();
    segments.append(&mut path.segments);
    path.segments = segments;
    path
}

fn infra_use_sites(code: &str, crates: &[&str]) -> Vec<MatchSite> {
    lex_tokens(code)
        .windows(3)
        .filter(|window| window_is_use_crate_head(window, code, crates))
        .filter_map(window_site)
        .collect()
}

fn window_is_use_crate_head(window: &[Tok], code: &str, crates: &[&str]) -> bool {
    match window {
        [use_tok, crate_tok, colons] => {
            use_tok.kind == TokKind::Ident
                && tok_text(code, *use_tok) == "use"
                && crate_tok.kind == TokKind::Ident
                && crates.iter().any(|krate| tok_text(code, *crate_tok) == *krate)
                && is_path_colons(*colons)
        }
        _ => false,
    }
}

fn window_site(window: &[Tok]) -> Option<MatchSite> {
    let first = window.first()?;
    let last = window.last()?;
    site_at(first.start, last.end.checked_sub(first.start)?)
}

fn direct_import_sites(code: &str, path_prefix: &str, names: &[&str]) -> Vec<MatchSite> {
    let prefix = split_path(path_prefix);
    let name_paths: Vec<Vec<&str>> = names.iter().map(|name| split_path(name)).collect();
    use_statements(code)
        .iter()
        .filter_map(|statement| direct_statement_site(code, statement, &prefix, &name_paths))
        .collect()
}

fn direct_statement_site(
    code: &str,
    statement: &UseStatement,
    prefix: &[&str],
    name_paths: &[Vec<&str>],
) -> Option<MatchSite> {
    if !segment_prefix_matches(code, &statement.tree.segments, prefix) {
        return None;
    }
    let rest = statement.tree.segments.get(prefix.len()..)?;
    let name = name_paths.iter().find(|name| segment_prefix_matches(code, rest, name))?;
    let first = rest.first()?;
    let last = rest.get(name.len().checked_sub(1)?)?;
    site_at(first.start, last.end.checked_sub(first.start)?)
}

fn grouped_import_sites(code: &str, prefix: &str, names: &[&str]) -> Vec<MatchSite> {
    let prefix_segments = prefixed_path_segments(prefix);
    let name_paths: Vec<Vec<&str>> = names.iter().map(|name| split_path(name)).collect();
    use_statements(code)
        .iter()
        .filter(|statement| segments_match(code, &statement.tree.segments, &prefix_segments))
        .filter_map(|statement| statement.tree.group.as_ref())
        .flat_map(|members| members.iter().flat_map(flatten_tree))
        .flat_map(|path| member_name_sites(code, &path, &name_paths))
        .collect()
}

fn prefixed_path_segments(prefix: &str) -> Vec<&str> {
    split_path(prefix.strip_prefix("use ").map_or(prefix, |rest| rest))
}

fn member_name_sites(code: &str, path: &FlatPath, name_paths: &[Vec<&str>]) -> Vec<MatchSite> {
    if path.glob {
        return Vec::new();
    }
    name_paths
        .iter()
        .filter(|name| segment_prefix_matches(code, &path.segments, name))
        .filter_map(|_| path.segments.last().copied())
        .filter_map(|leaf| site_at(leaf.start, leaf.end.checked_sub(leaf.start)?))
        .collect()
}

fn module_binding_sites(code: &str, module: &str, members: &[&str]) -> Vec<MatchSite> {
    module_bindings(code, module)
        .iter()
        .flat_map(|binding| binding_member_sites(code, binding.as_str(), members))
        .collect()
}

fn binding_member_sites(code: &str, binding: &str, members: &[&str]) -> Vec<MatchSite> {
    members
        .iter()
        .flat_map(|member| path_sites(code, &[binding, *member], false, path_head_boundary_ok))
        .collect()
}

fn module_bindings(code: &str, module: &str) -> Vec<String> {
    use_statements(code)
        .iter()
        .filter(|statement| statement_first_segment_is(code, statement, "std"))
        .flat_map(|statement| flatten_tree(&statement.tree))
        .filter_map(|path| module_binding_from_flat(code, &path, module))
        .collect()
}

fn statement_first_segment_is(code: &str, statement: &UseStatement, text: &str) -> bool {
    statement.tree.segments.first().is_some_and(|seg| tok_text(code, *seg) == text)
}

fn module_binding_from_flat(code: &str, path: &FlatPath, module: &str) -> Option<String> {
    let rest = path.segments.get(1..)?;
    let (head, tail) = rest.split_first()?;
    if tok_text(code, *head) != module {
        return None;
    }
    match tail {
        [] => Some(binding_name(code, path, module)),
        [only] if tok_text(code, *only) == "self" => Some(binding_name(code, path, module)),
        _ => None,
    }
}

fn binding_name(code: &str, path: &FlatPath, module: &str) -> String {
    path.alias.map_or_else(|| module.to_owned(), |alias| tok_text(code, alias).to_owned())
}

fn crate_alias_sites(code: &str, crates: &[&str]) -> Vec<MatchSite> {
    crates
        .iter()
        .flat_map(|krate| crate_root_aliases(code, krate))
        .flat_map(|alias| path_sites(code, &[alias.as_str()], true, leading_boundary_ok))
        .collect()
}

fn tokio_alias_effect_sites(code: &str) -> Vec<MatchSite> {
    crate_root_aliases(code, "tokio")
        .iter()
        .flat_map(|alias| alias_effect_sites(code, alias.as_str()))
        .collect()
}

fn alias_effect_sites(code: &str, alias: &str) -> Vec<MatchSite> {
    ["spawn", "task", "time"]
        .iter()
        .flat_map(|member| path_sites(code, &[alias, *member], false, leading_boundary_ok))
        .collect()
}

fn tokio_task_sites(code: &str) -> Vec<MatchSite> {
    let statements = use_statement_ranges(code);
    path_sites(code, &["tokio", "task"], false, leading_boundary_ok)
        .into_iter()
        .filter(|site| !is_bare_task_import_at(code, &statements, site.start_byte))
        .collect()
}

fn await_sites(code: &str) -> Vec<MatchSite> {
    code.match_indices(".await")
        .filter(|(start, _)| await_boundary_ok(code, *start))
        .filter_map(|(start, _)| site_at(start, ".await".len()))
        .collect()
}

fn await_boundary_ok(code: &str, start: usize) -> bool {
    code.split_at_checked(start.saturating_add(".await".len()))
        .is_none_or(|(_, after)| after.chars().next().is_none_or(|next| !is_ident_char(next)))
}

fn use_statement_ranges(code: &str) -> Vec<(usize, usize)> {
    let tokens = lex_tokens(code);
    tokens
        .iter()
        .enumerate()
        .filter(|(_, tok)| tok.kind == TokKind::Ident && tok_text(code, **tok) == "use")
        .filter(|(_, tok)| use_item_prefix_allowed(code, tok.start))
        .filter_map(|(index, tok)| statement_range_end(&tokens, index).map(|end| (tok.start, end)))
        .collect()
}

fn statement_range_end(tokens: &[Tok], use_index: usize) -> Option<usize> {
    tokens
        .get(use_index.checked_add(1)?..)?
        .iter()
        .find(|tok| tok.kind == TokKind::Semi)
        .map(|semi| semi.end)
}

fn is_bare_task_import_at(code: &str, statements: &[(usize, usize)], start: usize) -> bool {
    statements
        .iter()
        .find(|(stmt_start, stmt_end)| start >= *stmt_start && start < *stmt_end)
        .is_some_and(|(stmt_start, stmt_end)| {
            statement_text(code, *stmt_start, *stmt_end).is_some_and(is_bare_task_statement)
        })
}

fn statement_text(code: &str, start: usize, end: usize) -> Option<&str> {
    let (_, from_start) = code.split_at_checked(start)?;
    let (statement, _) = from_start.split_at_checked(end.checked_sub(start)?)?;
    Some(statement)
}

fn is_bare_task_statement(statement: &str) -> bool {
    is_compact_bare_task_statement(statement) || is_spaced_bare_task_statement(statement)
}

fn is_compact_bare_task_statement(statement: &str) -> bool {
    let compact: String = statement.chars().filter(|ch| !ch.is_whitespace()).collect();
    compact == "usetokio::task;"
        || compact == "usetokio::task::{self};"
        || compact == "usetokio::task::{self,};"
}

fn is_spaced_bare_task_statement(statement: &str) -> bool {
    let spaced = statement.split_whitespace().collect::<Vec<_>>().join(" ");
    spaced == "use tokio::task as task;"
        || spaced == "use tokio::task::{self as task};"
        || spaced == "use tokio::task::{self as task,};"
}

fn use_item_prefix_allowed(source: &str, start: usize) -> bool {
    source
        .split_at_checked(start)
        .map(|(before, _)| before)
        .map(current_line_prefix)
        .is_some_and(visibility_prefix_allows_use)
}

fn current_line_prefix(before_use: &str) -> &str {
    before_use.rsplit_once('\n').map_or(before_use, |(_, line)| line).trim_start()
}

fn visibility_prefix_allows_use(prefix: &str) -> bool {
    let prefix = prefix.trim_end();
    prefix.is_empty()
        || prefix == "pub"
        || prefix.strip_prefix("pub(").is_some_and(|rest| rest.ends_with(')'))
        || prefix.ends_with(';')
        || prefix.ends_with('{')
        || prefix.ends_with('}')
}

pub(super) fn crate_root_aliases(code: &str, krate: &str) -> Vec<String> {
    let definitions = alias_definitions(code);
    let resolved = (0..definitions.len())
        .fold(vec![krate.to_owned()], |current, _| extend_resolved_aliases(&definitions, &current));
    resolved.into_iter().filter(|alias| alias != krate).collect()
}

fn alias_definitions(code: &str) -> Vec<(String, String)> {
    use_statements(code)
        .iter()
        .filter_map(|statement| alias_definition_of(code, &statement.tree))
        .collect()
}

fn alias_definition_of(code: &str, tree: &UseTree) -> Option<(String, String)> {
    let (head, rest) = tree.segments.split_first()?;
    if !rest.is_empty() {
        return None;
    }
    let head = tok_text(code, *head).to_owned();
    if let Some(alias) = tree.alias {
        return Some((head, tok_text(code, alias).to_owned()));
    }
    tree.group
        .as_ref()?
        .iter()
        .find_map(|member| grouped_self_alias(code, member))
        .map(|alias| (head, alias))
}

fn grouped_self_alias(code: &str, member: &UseTree) -> Option<String> {
    if member.segments.len() == 1
        && member.segments.first().is_some_and(|seg| tok_text(code, *seg) == "self")
    {
        member.alias.map(|alias| tok_text(code, alias).to_owned())
    } else {
        None
    }
}

fn extend_resolved_aliases(definitions: &[(String, String)], resolved: &[String]) -> Vec<String> {
    definitions
        .iter()
        .filter(|(head, _)| resolved.iter().any(|name| name == head))
        .fold(resolved.to_vec(), |acc, (_, alias)| push_alias_once(acc, alias))
}

fn push_alias_once(mut acc: Vec<String>, alias: &String) -> Vec<String> {
    if !acc.contains(alias) {
        acc.push(alias.clone());
    }
    acc
}
