mod code_scan;

use code_scan::code_only_source;

use super::super::span::MatchSite;

pub(super) fn detect_core_infra_import(source: &str) -> Vec<MatchSite> {
    let code = code_only_source(source);
    merge_sites(
        use_line_sites(&code, &["use tokio::", "use axum::", "use sqlx::", "use reqwest::"])
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
        fully_qualified_sites(
            &code,
            &[
                "static mut",
                "std::sync::Mutex",
                "std::sync::RwLock",
                "std::cell::RefCell",
                "std::thread::spawn",
            ],
        )
        .into_iter()
        .chain(direct_import_sites(&code, "std::sync::", &["Mutex", "RwLock"]))
        .chain(grouped_import_sites(&code, "use std::sync::", &["Mutex", "RwLock"]))
        .chain(grouped_import_sites(&code, "use std::", &["sync::Mutex", "sync::RwLock"]))
        .chain(direct_import_sites(&code, "std::cell::", &["RefCell"]))
        .chain(grouped_import_sites(&code, "use std::cell::", &["RefCell"]))
        .chain(grouped_import_sites(&code, "use std::", &["cell::RefCell"]))
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

fn path_head_sites(code: &str, needle: &str) -> Vec<MatchSite> {
    code.match_indices(needle)
        .filter(|(start, _)| {
            path_head_boundary_ok(code, *start) && trailing_boundary_ok(code, *start, needle)
        })
        .filter_map(|(start, _)| site_at(start, needle.len()))
        .collect()
}

fn fully_qualified_sites(code: &str, needles: &[&str]) -> Vec<MatchSite> {
    needles.iter().flat_map(|needle| token_sites(code, needle)).collect()
}

fn crate_alias_sites(code: &str, crates: &[&str]) -> Vec<MatchSite> {
    crates
        .iter()
        .flat_map(|krate| crate_root_aliases(code, krate))
        .flat_map(|alias| token_sites(code, &format!("{alias}::")))
        .collect()
}

fn use_line_sites(code: &str, needles: &[&str]) -> Vec<MatchSite> {
    code.split('\n')
        .fold((Vec::new(), 0usize), |(mut sites, offset), line| {
            sites.extend(line_sites_at(line, offset, needles));
            (sites, offset.saturating_add(line.len()).saturating_add(1))
        })
        .0
}

fn line_sites_at(line: &str, offset: usize, needles: &[&str]) -> Vec<MatchSite> {
    needles
        .iter()
        .flat_map(|needle| {
            line.match_indices(needle)
                .filter_map(|(index, _)| offset.checked_add(index))
                .filter_map(|start| site_at(start, needle.len()))
                .collect::<Vec<_>>()
        })
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

fn tokio_task_sites(code: &str) -> Vec<MatchSite> {
    let statements = use_statement_ranges(code);
    code.match_indices("tokio::task")
        .filter(|(start, _)| {
            leading_boundary_ok(code, *start) && trailing_boundary_ok(code, *start, "tokio::task")
        })
        .filter(|(start, _)| !is_bare_task_import_at(code, &statements, *start))
        .filter_map(|(start, _)| site_at(start, "tokio::task".len()))
        .collect()
}

fn tokio_alias_effect_sites(code: &str) -> Vec<MatchSite> {
    crate_root_aliases(code, "tokio")
        .iter()
        .flat_map(|alias| {
            ["spawn", "task", "time"].iter().map(move |member| format!("{alias}::{member}"))
        })
        .flat_map(|needle| token_sites(code, &needle))
        .collect()
}

fn use_statement_ranges(code: &str) -> Vec<(usize, usize)> {
    code.match_indices("use ")
        .filter(|(start, _)| use_item_prefix_allowed(code, *start))
        .filter_map(|(start, _)| use_statement_end(code, start).map(|end| (start, end)))
        .collect()
}

fn use_statement_end(code: &str, start: usize) -> Option<usize> {
    let (_, from_use) = code.split_at_checked(start)?;
    let semicolon = from_use.find(';')?;
    start.checked_add(semicolon)?.checked_add(1)
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

fn module_binding_sites(code: &str, module: &str, members: &[&str]) -> Vec<MatchSite> {
    module_bindings(code, module)
        .iter()
        .flat_map(|binding| members.iter().map(move |member| format!("{binding}::{member}")))
        .flat_map(|needle| path_head_sites(code, &needle))
        .collect()
}

fn module_bindings(code: &str, module: &str) -> Vec<String> {
    code.match_indices("use ")
        .filter_map(|(start, _)| use_statement_path_text(code, start))
        .flat_map(|statement| statement_module_bindings(&statement, module))
        .collect()
}

fn use_statement_path_text(source: &str, start: usize) -> Option<String> {
    use_item_prefix_allowed(source, start).then_some(())?;
    let statement = source.split_at_checked(start)?.1.strip_prefix("use ")?.split_once(';')?.0;
    Some(statement.trim().to_owned())
}

fn statement_module_bindings(statement: &str, module: &str) -> Vec<String> {
    let Some(rest) = statement.strip_prefix("std::") else {
        return Vec::new();
    };
    std_relative_paths(rest)
        .iter()
        .filter_map(|path| module_binding_from_path(path, module))
        .collect()
}

fn std_relative_paths(rest: &str) -> Vec<String> {
    let rest = rest.trim();
    rest.strip_prefix('{').map_or_else(
        || flatten_member(rest),
        |grouped| grouped.strip_suffix('}').map_or_else(Vec::new, flattened_member_paths),
    )
}

fn module_binding_from_path(path: &str, module: &str) -> Option<String> {
    let rest = path.strip_prefix(module)?;
    match rest {
        "" => Some(module.to_owned()),
        _ => self_binding_from_rest(rest, module),
    }
}

fn self_binding_from_rest(rest: &str, module: &str) -> Option<String> {
    if let Some(alias) = rest.strip_prefix(" as ") {
        return valid_alias(alias.trim());
    }
    let after_self = rest.strip_prefix("::self")?;
    match after_self {
        "" => Some(module.to_owned()),
        _ => after_self.strip_prefix(" as ").and_then(|alias| valid_alias(alias.trim())),
    }
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

fn path_head_boundary_ok(code: &str, start: usize) -> bool {
    code.split_at_checked(start).is_some_and(|(before, _)| {
        before.chars().next_back().is_none_or(|prev| !is_ident_char(prev) && prev != ':')
    })
}

fn is_ident_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn direct_import_sites(code: &str, path_prefix: &str, names: &[&str]) -> Vec<MatchSite> {
    code.match_indices("use ")
        .filter_map(|(start, _)| direct_import_member_span(code, start, path_prefix))
        .filter_map(|(member, member_start)| matched_name_site(member, member_start, names))
        .collect()
}

fn direct_import_member_span<'a>(
    code: &'a str,
    start: usize,
    path_prefix: &str,
) -> Option<(&'a str, usize)> {
    let (_, from_use) = code.split_at_checked(start)?;
    use_item_prefix_allowed(code, start).then_some(())?;
    let untrimmed = from_use.strip_prefix("use ")?.strip_prefix(path_prefix)?.split_once(';')?.0;
    let leading = untrimmed.len().saturating_sub(untrimmed.trim_start().len());
    let member_start = start.checked_add("use ".len())?.checked_add(path_prefix.len())?;
    Some((untrimmed.trim(), member_start.checked_add(leading)?))
}

fn matched_name_site(member: &str, member_start: usize, names: &[&str]) -> Option<MatchSite> {
    names
        .iter()
        .find(|name| grouped_member_matches(member, name))
        .and_then(|name| site_at(member_start, name.len()))
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

fn grouped_import_sites(code: &str, prefix: &str, names: &[&str]) -> Vec<MatchSite> {
    code.match_indices(prefix)
        .filter_map(|(start, _)| grouped_import_body_span(code, start, prefix))
        .flat_map(|(body, body_start)| grouped_body_sites(body, body_start, names))
        .collect()
}

fn grouped_import_body_span<'a>(
    code: &'a str,
    start: usize,
    prefix: &str,
) -> Option<(&'a str, usize)> {
    let (_, from_prefix) = code.split_at_checked(start)?;
    use_item_prefix_allowed(code, start).then_some(())?;
    let after_prefix = from_prefix.strip_prefix(prefix)?;
    let skipped = after_prefix.len().saturating_sub(after_prefix.trim_start().len());
    let after_open = after_prefix.trim_start().strip_prefix('{')?;
    let close = matching_close_brace(after_open)?;
    let (body, _) = after_open.split_at_checked(close)?;
    let body_start = start.checked_add(prefix.len())?.checked_add(skipped)?.checked_add(1)?;
    Some((body, body_start))
}

fn grouped_body_sites(body: &str, body_start: usize, names: &[&str]) -> Vec<MatchSite> {
    top_level_member_spans(body)
        .into_iter()
        .filter_map(|(start, end)| member_slice(body, start, end).map(|text| (text, start)))
        .flat_map(|(text, start)| member_name_sites(text, start, body_start, names))
        .collect()
}

fn member_name_sites(
    member: &str,
    member_start: usize,
    body_start: usize,
    names: &[&str],
) -> Vec<MatchSite> {
    flatten_member(member)
        .iter()
        .filter(|path| names.iter().any(|name| grouped_member_matches(path, name)))
        .filter_map(|path| leaf_site(member, member_start, body_start, path))
        .collect()
}

fn leaf_site(
    member: &str,
    member_start: usize,
    body_start: usize,
    path: &str,
) -> Option<MatchSite> {
    let leaf = path.rsplit("::").next().map_or(path, |leaf| leaf);
    let leaf_offset = member.rfind(leaf)?;
    let start = body_start.checked_add(member_start)?.checked_add(leaf_offset)?;
    site_at(start, leaf.len())
}

fn top_level_member_spans(body: &str) -> Vec<(usize, usize)> {
    let (spans, _, member_start) =
        body.char_indices().fold((Vec::new(), 0usize, 0usize), advance_member_span_scan);
    spans
        .into_iter()
        .chain((member_start < body.len()).then_some((member_start, body.len())))
        .collect()
}

type MemberSpanScan = (Vec<(usize, usize)>, usize, usize);

fn advance_member_span_scan(state: MemberSpanScan, (index, ch): (usize, char)) -> MemberSpanScan {
    let (mut spans, depth, member_start) = state;
    match ch {
        '{' => (spans, depth.saturating_add(1), member_start),
        '}' => (spans, depth.saturating_sub(1), member_start),
        ',' if depth == 0 => {
            spans.push((member_start, index));
            (spans, depth, index.saturating_add(1))
        }
        _ => (spans, depth, member_start),
    }
}

fn member_slice(body: &str, start: usize, end: usize) -> Option<&str> {
    let (_, from_start) = body.split_at_checked(start)?;
    let (member, _) = from_start.split_at_checked(end.checked_sub(start)?)?;
    Some(member)
}

fn flattened_member_paths(body: &str) -> Vec<String> {
    top_level_member_texts(body).iter().flat_map(|member| flatten_member(member)).collect()
}

fn top_level_member_texts(body: &str) -> Vec<&str> {
    top_level_member_spans(body)
        .into_iter()
        .filter_map(|(start, end)| member_slice(body, start, end))
        .map(str::trim)
        .filter(|member| !member.is_empty())
        .collect()
}

fn flatten_member(member: &str) -> Vec<String> {
    let Some(open) = member.find('{') else {
        return vec![member.trim().to_owned()];
    };
    let Some((head, inner)) = member_head_and_group(member, open) else {
        return vec![member.trim().to_owned()];
    };
    flattened_member_paths(inner).iter().map(|path| format!("{head}::{path}")).collect()
}

fn member_head_and_group(member: &str, open: usize) -> Option<(&str, &str)> {
    let (head, from_open) = member.split_at_checked(open)?;
    let inner = from_open.strip_prefix('{')?;
    let close = matching_close_brace(inner)?;
    let (body, _) = inner.split_at_checked(close)?;
    Some((head.trim().trim_end_matches("::").trim_end(), body))
}

fn matching_close_brace(source: &str) -> Option<usize> {
    let mut depth = 1usize;
    source.char_indices().find_map(|(index, ch)| {
        depth = brace_depth_after(depth, ch)?;
        (depth == 0).then_some(index)
    })
}

const fn brace_depth_after(depth: usize, ch: char) -> Option<usize> {
    match ch {
        '{' => depth.checked_add(1),
        '}' => depth.checked_sub(1),
        _ => Some(depth),
    }
}

fn grouped_member_matches(member: &str, name: &str) -> bool {
    member
        .strip_prefix(name)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with("::") || rest.starts_with(" as "))
}

pub(super) fn crate_root_aliases(code: &str, krate: &str) -> Vec<String> {
    let definitions = alias_definitions(code);
    let resolved = (0..definitions.len())
        .fold(vec![krate.to_owned()], |current, _| extend_resolved_aliases(&definitions, &current));
    resolved.into_iter().filter(|alias| alias != krate).collect()
}

fn alias_definitions(code: &str) -> Vec<(String, String)> {
    code.match_indices("use ").filter_map(|(start, _)| alias_definition_at(code, start)).collect()
}

fn alias_definition_at(source: &str, start: usize) -> Option<(String, String)> {
    use_item_prefix_allowed(source, start).then_some(())?;
    let statement =
        source.split_at_checked(start)?.1.strip_prefix("use ")?.split_once(';')?.0.trim();
    plain_alias_definition(statement).or_else(|| grouped_alias_definition(statement))
}

fn plain_alias_definition(statement: &str) -> Option<(String, String)> {
    let (head, alias) = statement.split_once(" as ")?;
    let head = valid_alias(head.trim())?;
    valid_alias(alias.trim()).map(|alias| (head, alias))
}

fn grouped_alias_definition(statement: &str) -> Option<(String, String)> {
    let (head, grouped) = statement.split_once("::{")?;
    let head = valid_alias(head.trim())?;
    let grouped = grouped.trim().strip_suffix('}')?;
    grouped
        .split(',')
        .map(str::trim)
        .find_map(|member| member.strip_prefix("self as "))
        .and_then(|alias| valid_alias(alias.trim()))
        .map(|alias| (head, alias))
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

fn valid_alias(alias: &str) -> Option<String> {
    (!alias.is_empty() && alias.chars().all(is_ident_char)).then(|| alias.to_owned())
}
