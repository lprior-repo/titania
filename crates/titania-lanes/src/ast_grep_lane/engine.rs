use ast_grep_core::{AstGrep, Node, Pattern, tree_sitter::StrDoc};
use ast_grep_language::Rust;

use super::span::{MatchSite, line_offsets};

const MAX_NESTING_DEPTH: usize = 2;

const UNBOUNDED_ITERATOR_SPELLINGS: &[&str] = &[
    "$ITER.cycle()",
    "repeat($$$ARGS)",
    "repeat_with($$$ARGS)",
    "successors($$$ARGS)",
    "from_fn($$$ARGS)",
    "std::iter::repeat($$$ARGS)",
    "std::iter::repeat_with($$$ARGS)",
    "std::iter::successors($$$ARGS)",
    "std::iter::from_fn($$$ARGS)",
    "core::iter::repeat($$$ARGS)",
    "core::iter::repeat_with($$$ARGS)",
    "core::iter::successors($$$ARGS)",
    "core::iter::from_fn($$$ARGS)",
    "::std::iter::repeat($$$ARGS)",
    "::std::iter::repeat_with($$$ARGS)",
    "::std::iter::successors($$$ARGS)",
    "::std::iter::from_fn($$$ARGS)",
    "::core::iter::repeat($$$ARGS)",
    "::core::iter::repeat_with($$$ARGS)",
    "::core::iter::successors($$$ARGS)",
    "::core::iter::from_fn($$$ARGS)",
];

fn unbounded_iterator_patterns() -> Vec<Pattern> {
    UNBOUNDED_ITERATOR_SPELLINGS.iter().map(|text| Pattern::new(text, Rust)).collect()
}

const UNBOUNDED_ITERATOR_FNS: &[&str] = &["repeat", "repeat_with", "successors", "from_fn"];
const ITER_MODULE_ROOTS: &[&str] = &["std", "core"];

pub(super) struct AstEngine {
    grep: AstGrep<StrDoc<Rust>>,
    line_offsets: Vec<usize>,
    test_ranges: Vec<(usize, usize)>,
}

impl AstEngine {
    pub(super) fn new(source: &str) -> Self {
        let grep = AstGrep::new(source, Rust);
        let test_ranges = collect_test_ranges(&grep.root());
        Self { grep, line_offsets: line_offsets(source), test_ranges }
    }

    pub(super) fn line_offsets(&self) -> &[usize] {
        &self.line_offsets
    }

    fn is_in_test_code(&self, byte_offset: usize) -> bool {
        self.test_ranges.iter().any(|(start, end)| byte_offset >= *start && byte_offset < *end)
    }

    fn match_sites(&self, pattern: &Pattern) -> Vec<MatchSite> {
        self.grep
            .root()
            .find_all(pattern)
            .map(|node_match| MatchSite::from_range(node_match.range()))
            .collect()
    }

    fn match_sites_skipping_tests(&self, pattern: &Pattern) -> Vec<MatchSite> {
        self.grep
            .root()
            .find_all(pattern)
            .filter(|node_match| !self.is_in_test_code(node_match.range().start))
            .map(|node_match| MatchSite::from_range(node_match.range()))
            .collect()
    }

    fn match_sites_any_skipping_tests<'p>(
        &self,
        patterns: impl IntoIterator<Item = &'p Pattern>,
    ) -> Vec<MatchSite> {
        let sites =
            patterns.into_iter().flat_map(|pat| self.match_sites_skipping_tests(pat)).collect();
        sorted_sites(sites)
    }

    pub(super) fn detect_unbounded_loop(&self) -> Vec<MatchSite> {
        let loop_pat = Pattern::new("loop { $$$BODY }", Rust);
        let while_pat = Pattern::new("while $COND { $$$BODY }", Rust);
        let loop_sites = self.match_sites_skipping_tests(&loop_pat);
        let while_sites = self
            .grep
            .root()
            .find_all(&while_pat)
            .filter(|node_match| !self.is_in_test_code(node_match.range().start))
            .filter(|node_match| !is_while_let_match(node_match))
            .map(|node_match| MatchSite::from_range(node_match.range()))
            .collect::<Vec<_>>();
        sorted_sites(loop_sites.into_iter().chain(while_sites).collect())
    }

    pub(super) fn detect_unbounded_iterator(&self) -> Vec<MatchSite> {
        let patterns = unbounded_iterator_patterns();
        let pattern_sites = self.match_sites_any_skipping_tests(patterns.iter());
        let resolved_sites = self.resolved_iterator_sites();
        sorted_sites(pattern_sites.into_iter().chain(resolved_sites).collect())
    }

    fn resolved_iterator_sites(&self) -> Vec<MatchSite> {
        let resolution = IteratorResolution::collect(&self.grep.root());
        self.grep
            .root()
            .dfs()
            .filter(|node| node.kind() == "call_expression")
            .filter(|node| !self.is_in_test_code(node.range().start))
            .filter(|node| call_is_unbounded_iterator(node, &resolution))
            .map(|node| MatchSite::from_range(node.range()))
            .collect()
    }

    pub(super) fn detect_print_stdout(&self) -> Vec<MatchSite> {
        let print = Pattern::new("print!($$$ARGS)", Rust);
        let println = Pattern::new("println!($$$ARGS)", Rust);
        self.match_sites_any_skipping_tests([&print, &println])
    }

    pub(super) fn detect_print_stderr(&self) -> Vec<MatchSite> {
        let eprint = Pattern::new("eprint!($$$ARGS)", Rust);
        let eprintln = Pattern::new("eprintln!($$$ARGS)", Rust);
        self.match_sites_any_skipping_tests([&eprint, &eprintln])
    }

    pub(super) fn detect_wildcard_import(&self) -> Vec<MatchSite> {
        let pat = Pattern::new("use $PATH::*;", Rust);
        self.match_sites_skipping_tests(&pat)
    }

    pub(super) fn detect_unwrap_or(&self) -> Vec<MatchSite> {
        let or = Pattern::new("$VALUE.unwrap_or($DEFAULT)", Rust);
        let or_else = Pattern::new("$VALUE.unwrap_or_else($DEFAULT)", Rust);
        let or_default = Pattern::new("$VALUE.unwrap_or_default()", Rust);
        self.match_sites_any_skipping_tests([&or, &or_else, &or_default])
    }

    pub(super) fn detect_result_string(&self) -> Vec<MatchSite> {
        self.grep
            .root()
            .dfs()
            .filter(|node| !self.is_in_test_code(node.range().start))
            .filter(|node| is_result_with_string_error(node).is_some())
            .map(|node| MatchSite::from_range(node.range()))
            .collect()
    }

    pub(super) fn detect_nesting_depth(&self) -> Vec<MatchSite> {
        self.grep
            .root()
            .dfs()
            .filter(|node| node.kind() == "function_item")
            .filter(|node| !self.is_in_test_code(node.range().start))
            .filter_map(|node| function_excess_nesting_site(&node))
            .collect()
    }

    pub(super) fn detect_recursion_direct(&self) -> Vec<MatchSite> {
        self.grep
            .root()
            .dfs()
            .filter(|node| node.kind() == "function_item")
            .filter(|node| !self.is_in_test_code(node.range().start))
            .filter_map(|node| function_recursion_site(&node))
            .collect()
    }

    pub(super) fn detect_allow_attr(&self) -> Vec<MatchSite> {
        let pat = Pattern::new("#[allow($$$LINTS)]", Rust);
        self.match_sites(&pat)
    }

    pub(super) fn detect_expect_attr(&self) -> Vec<MatchSite> {
        let pat = Pattern::new("#[expect($$$LINTS)]", Rust);
        self.match_sites(&pat)
    }

    pub(super) fn detect_cfg_attr_allow(&self) -> Vec<MatchSite> {
        let pat = Pattern::new("#[cfg_attr($COND, allow($$$LINTS))]", Rust);
        self.match_sites(&pat)
    }

    pub(super) fn detect_crate_allow(&self) -> Vec<MatchSite> {
        let pat = Pattern::new("#![allow($$$LINTS)]", Rust);
        self.match_sites(&pat)
    }

    pub(super) fn detect_crate_expect(&self) -> Vec<MatchSite> {
        let pat = Pattern::new("#![expect($$$LINTS)]", Rust);
        self.match_sites(&pat)
    }

    pub(super) fn detect_inline_suppression(&self) -> Vec<MatchSite> {
        self.grep
            .root()
            .dfs()
            .filter(|node| matches!(node.kind().as_ref(), "line_comment" | "block_comment"))
            .filter(|node| contains_inline_suppression_marker(node.text().as_ref()))
            .map(|node| MatchSite::from_range(node.range()))
            .collect()
    }

    pub(super) fn detect_generated_include(&self) -> Vec<MatchSite> {
        let concat_pattern = Pattern::new("concat!($$$PARTS)", Rust);
        let direct_pattern = Pattern::new("include!(concat!(env!($ENV), $PATH))", Rust);
        let message_pattern =
            Pattern::new("include!(concat!(env!($ENV, $$$MESSAGE), $PATH))", Rust);
        let many_pattern = Pattern::new("include!(concat!(env!($ENV), $$$PATHS))", Rust);
        let many_message_pattern =
            Pattern::new("include!(concat!(env!($ENV, $$$MESSAGE), $$$PATHS))", Rust);
        let sites = [
            single_include_sites(&self.grep, &direct_pattern, &concat_pattern),
            single_include_sites(&self.grep, &message_pattern, &concat_pattern),
            many_include_sites(&self.grep, &many_pattern, &concat_pattern),
            many_include_sites(&self.grep, &many_message_pattern, &concat_pattern),
        ]
        .into_iter()
        .flatten()
        .collect();
        sorted_sites(sites)
    }
}

fn sorted_sites(mut sites: Vec<MatchSite>) -> Vec<MatchSite> {
    sites.sort_by_key(|site| (site.start_byte, site.end_byte));
    sites.dedup_by_key(|site| (site.start_byte, site.end_byte));
    sites
}

struct IteratorResolution {
    module_bindings: Vec<String>,
    fn_aliases: Vec<String>,
}

impl IteratorResolution {
    fn collect(root: &Node<'_, StrDoc<Rust>>) -> Self {
        collect_iterator_resolution(root)
    }
}

fn collect_iterator_resolution(root: &Node<'_, StrDoc<Rust>>) -> IteratorResolution {
    root.dfs().filter(|node| node.kind() == "use_declaration").fold(
        IteratorResolution { module_bindings: Vec::new(), fn_aliases: Vec::new() },
        |resolution, statement| absorb_use_statement(resolution, &statement),
    )
}

fn absorb_use_statement(
    mut resolution: IteratorResolution,
    statement: &Node<'_, StrDoc<Rust>>,
) -> IteratorResolution {
    let Some(body) = use_statement_body(statement) else {
        return resolution;
    };
    for path in flattened_use_paths(&body) {
        apply_use_classification(&mut resolution, &path);
    }
    resolution
}

fn apply_use_classification(resolution: &mut IteratorResolution, path: &str) {
    match classify_use_path(path) {
        UseClassification::ModuleBinding(name) => {
            push_binding_once(&mut resolution.module_bindings, &name);
        }
        UseClassification::FnAlias(name) => push_binding_once(&mut resolution.fn_aliases, &name),
        UseClassification::Ignored => {}
    }
}

enum UseClassification {
    ModuleBinding(String),
    FnAlias(String),
    Ignored,
}

fn classify_use_path(path: &str) -> UseClassification {
    let (path_part, alias) = split_use_alias(path);
    let segments: Vec<&str> =
        path_part.split("::").map(str::trim).filter(|seg| !seg.is_empty()).collect();
    match segments.as_slice() {
        [root, "iter"] | [root, "iter", "self"] if is_iter_module_root(root) => {
            UseClassification::ModuleBinding(module_binding_name(alias).to_owned())
        }
        [root, "iter", func] if is_iter_module_root(root) && is_unbounded_iterator_fn(func) => {
            alias.map_or(UseClassification::Ignored, fn_alias_classification)
        }
        _ => UseClassification::Ignored,
    }
}

fn module_binding_name(alias: Option<&str>) -> &str {
    alias.map_or("iter", |name| name)
}

fn fn_alias_classification(name: &str) -> UseClassification {
    UseClassification::FnAlias(name.to_owned())
}

fn push_binding_once(bindings: &mut Vec<String>, name: &str) {
    if !bindings.iter().any(|bound| bound == name) {
        bindings.push(name.to_owned());
    }
}

fn is_iter_module_root(segment: &str) -> bool {
    ITER_MODULE_ROOTS.contains(&segment)
}

fn is_unbounded_iterator_fn(name: &str) -> bool {
    UNBOUNDED_ITERATOR_FNS.contains(&name)
}

fn split_use_alias(path: &str) -> (&str, Option<&str>) {
    match path.split_once(" as ") {
        Some((head, alias)) => (head.trim(), valid_use_alias(alias.trim())),
        None => (path.trim(), None),
    }
}

fn valid_use_alias(alias: &str) -> Option<&str> {
    (!alias.is_empty() && alias.chars().all(|ch| ch.is_alphanumeric() || ch == '_'))
        .then_some(alias)
}

fn use_statement_body(statement: &Node<'_, StrDoc<Rust>>) -> Option<String> {
    let text = statement.text();
    let after_use = text.split_once("use ")?.1;
    let body = after_use.split_once(';')?.0;
    Some(body.trim().to_owned())
}

fn flattened_use_paths(body: &str) -> Vec<String> {
    let Some(open) = body.find('{') else {
        return vec![body.trim().to_owned()];
    };
    let Some((head, inner)) = use_group_head_and_body(body, open) else {
        return vec![body.trim().to_owned()];
    };
    top_level_use_members(inner)
        .iter()
        .flat_map(|member| flattened_use_paths(member))
        .map(|path| format!("{head}::{path}"))
        .collect()
}

fn use_group_head_and_body(member: &str, open: usize) -> Option<(&str, &str)> {
    let (head, from_open) = member.split_at_checked(open)?;
    let inner = from_open.strip_prefix('{')?;
    let close = matching_use_close_brace(inner)?;
    let (body, _) = inner.split_at_checked(close)?;
    Some((head.trim().trim_end_matches("::").trim_end(), body))
}

fn matching_use_close_brace(source: &str) -> Option<usize> {
    let mut depth = 1usize;
    source.char_indices().find_map(|(index, ch)| {
        depth = use_brace_depth_after(depth, ch)?;
        (depth == 0).then_some(index)
    })
}

const fn use_brace_depth_after(depth: usize, ch: char) -> Option<usize> {
    match ch {
        '{' => depth.checked_add(1),
        '}' => depth.checked_sub(1),
        _ => Some(depth),
    }
}

type UseMemberScan = (Vec<(usize, usize)>, usize, usize);

fn top_level_use_members(body: &str) -> Vec<&str> {
    let (spans, _, member_start) =
        body.char_indices().fold((Vec::new(), 0usize, 0usize), advance_use_member_scan);
    spans
        .into_iter()
        .chain((member_start < body.len()).then_some((member_start, body.len())))
        .filter_map(|(start, end)| body.get(start..end))
        .map(str::trim)
        .filter(|member| !member.is_empty())
        .collect()
}

fn advance_use_member_scan(state: UseMemberScan, (index, ch): (usize, char)) -> UseMemberScan {
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

fn call_is_unbounded_iterator(
    call: &Node<'_, StrDoc<Rust>>,
    resolution: &IteratorResolution,
) -> bool {
    let Some(callee) = call.children().find(|child| is_callee_kind(&child.kind())) else {
        return false;
    };
    let peeled = peel_generic_function(&callee);
    match peeled.kind().as_ref() {
        "identifier" => identifier_is_unbounded(&peeled, resolution),
        "scoped_identifier" => scoped_callee_is_unbounded(&peeled, resolution),
        _ => false,
    }
}

fn identifier_is_unbounded(node: &Node<'_, StrDoc<Rust>>, resolution: &IteratorResolution) -> bool {
    let text = node.text();
    let name = text.trim();
    is_unbounded_iterator_fn(name)
        || resolution.fn_aliases.iter().any(|alias| alias.as_str() == name)
}

fn scoped_callee_is_unbounded(
    callee: &Node<'_, StrDoc<Rust>>,
    resolution: &IteratorResolution,
) -> bool {
    let segments = scoped_path_segments(callee);
    match segments.as_slice() {
        [root, module, func] => {
            is_iter_module_root(root) && module.as_str() == "iter" && is_unbounded_iterator_fn(func)
        }
        [binding, func] => {
            resolution.module_bindings.iter().any(|bound| bound == binding)
                && is_unbounded_iterator_fn(func)
        }
        _ => false,
    }
}

fn scoped_path_segments(node: &Node<'_, StrDoc<Rust>>) -> Vec<String> {
    let mut named = node.children().filter(Node::is_named);
    match (node.kind().as_ref(), named.next(), named.next()) {
        ("scoped_identifier", Some(head), Some(leaf)) => segments_with_leaf(&head, &leaf),
        ("scoped_identifier", Some(only), None) => scoped_path_segments(&only),
        _ => vec![node.text().trim().to_owned()],
    }
}

fn segments_with_leaf(head: &Node<'_, StrDoc<Rust>>, leaf: &Node<'_, StrDoc<Rust>>) -> Vec<String> {
    let mut segments = scoped_path_segments(head);
    segments.push(leaf.text().trim().to_owned());
    segments
}

fn contains_inline_suppression_marker(comment: &str) -> bool {
    comment.contains("ast-grep-ignore") || comment.contains("sg-ignore")
}

fn is_out_dir_expression(node: &Node<'_, StrDoc<Rust>>, concat_pattern: &Pattern) -> bool {
    if is_out_dir_node(node) {
        return true;
    }
    decode_concat_expression(node, concat_pattern).is_some_and(|decoded| decoded == "OUT_DIR")
}

fn decode_concat_expression(
    node: &Node<'_, StrDoc<Rust>>,
    concat_pattern: &Pattern,
) -> Option<String> {
    let concat_match = node.find(concat_pattern)?;
    concat_match
        .get_env()
        .get_multiple_matches("PARTS")
        .iter()
        .try_fold(String::new(), append_decoded_part)
}

fn append_decoded_part(mut output: String, part: &Node<'_, StrDoc<Rust>>) -> Option<String> {
    let text = part.text();
    let (literal, tail, raw) = parse_literal(text.as_ref())?;
    if !tail.is_empty() {
        return None;
    }
    let decoded = decode_literal(literal, raw)?;
    output.push_str(&decoded);
    Some(output)
}

fn single_include_sites(
    grep: &AstGrep<StrDoc<Rust>>,
    pattern: &Pattern,
    concat_pattern: &Pattern,
) -> Vec<MatchSite> {
    grep.root()
        .find_all(pattern)
        .filter_map(|node_match| {
            let environment = node_match.get_env().get_match("ENV")?;
            let path = node_match.get_env().get_match("PATH")?;
            (is_out_dir_expression(environment, concat_pattern) && path_non_empty(path))
                .then(|| MatchSite::from_range(node_match.range()))
        })
        .collect()
}

fn many_include_sites(
    grep: &AstGrep<StrDoc<Rust>>,
    pattern: &Pattern,
    concat_pattern: &Pattern,
) -> Vec<MatchSite> {
    grep.root()
        .find_all(pattern)
        .filter_map(|node_match| {
            let environment = node_match.get_env().get_match("ENV")?;
            let paths = node_match.get_env().get_multiple_matches("PATHS");
            (!paths.is_empty()
                && is_out_dir_expression(environment, concat_pattern)
                && paths.iter().any(path_non_empty))
            .then(|| MatchSite::from_range(node_match.range()))
        })
        .collect()
}

fn is_while_let_match(node: &Node<'_, StrDoc<Rust>>) -> bool {
    let text = node.text();
    let trimmed = text.trim_start();
    let after_while = trimmed.strip_prefix("while").map_or(trimmed, |rest| rest);
    after_while.trim_start().starts_with("let")
}

fn path_non_empty(node: &Node<'_, StrDoc<Rust>>) -> bool {
    !node.text().trim().is_empty()
}

fn is_out_dir_node(node: &Node<'_, StrDoc<Rust>>) -> bool {
    let text = node.text();
    let Some((literal, tail, raw)) = parse_literal(text.as_ref()) else {
        return false;
    };
    tail.is_empty() && is_out_dir_literal(literal, raw)
}

type LiteralParts<'a> = (&'a str, &'a str, bool);

#[derive(Debug)]
enum EscapeMode {
    Plain,
    AfterSlash,
    LineContinuation,
    Hex { digits: String },
    Unicode { digits: String, started: bool },
}

fn is_out_dir_literal(literal: &str, raw: bool) -> bool {
    decode_literal(literal, raw).is_some_and(|decoded| decoded == "OUT_DIR")
}

fn decode_literal(literal: &str, raw: bool) -> Option<String> {
    if raw {
        return Some(literal.to_owned());
    }
    let (decoded, mode) = fold_escape_sequence(literal)?;
    matches!(mode, EscapeMode::Plain).then_some(decoded)
}

fn fold_escape_sequence(literal: &str) -> Option<(String, EscapeMode)> {
    literal.chars().try_fold((String::new(), EscapeMode::Plain), |current, character| {
        advance_escape(current, character)
    })
}

fn advance_escape(
    (decoded, mode): (String, EscapeMode),
    character: char,
) -> Option<(String, EscapeMode)> {
    match mode {
        EscapeMode::Plain => Some(plain_state(decoded, character)),
        EscapeMode::AfterSlash => after_slash(decoded, character),
        EscapeMode::LineContinuation => Some(line_continuation_state(decoded, character)),
        EscapeMode::Hex { digits } => hex_digit(decoded, character, digits),
        EscapeMode::Unicode { digits, started } => {
            unicode_digit(decoded, character, digits, started)
        }
    }
}

fn plain_state(decoded: String, character: char) -> (String, EscapeMode) {
    if character == '\\' {
        (decoded, EscapeMode::AfterSlash)
    } else {
        push_decoded(decoded, character)
    }
}

fn after_slash(decoded: String, character: char) -> Option<(String, EscapeMode)> {
    if let Some(next) = control_escape(character) {
        return Some((decoded, next));
    }
    let mapped = simple_escape(character)?;
    Some(push_decoded(decoded, mapped))
}

fn line_continuation_state(decoded: String, character: char) -> (String, EscapeMode) {
    if character.is_whitespace() {
        (decoded, EscapeMode::LineContinuation)
    } else {
        push_decoded(decoded, character)
    }
}

fn hex_digit(decoded: String, character: char, mut digits: String) -> Option<(String, EscapeMode)> {
    digits.push(character);
    match digits.len() {
        0 | 1 => Some((decoded, EscapeMode::Hex { digits })),
        2 => {
            let code = parse_hex_byte(&digits)?;
            Some(push_decoded(decoded, char::from(code)))
        }
        _ => None,
    }
}

fn unicode_digit(
    decoded: String,
    character: char,
    mut digits: String,
    started: bool,
) -> Option<(String, EscapeMode)> {
    if !started && character == '{' {
        return Some((decoded, EscapeMode::Unicode { digits, started: true }));
    }
    if started && character == '}' {
        let code = parse_hex_codepoint(&digits)?;
        let decoded_character = char::from_u32(code)?;
        return Some(push_decoded(decoded, decoded_character));
    }
    if !started {
        return None;
    }
    if character != '_' {
        digits.push(character);
    }
    Some((decoded, EscapeMode::Unicode { digits, started }))
}

const fn control_escape(character: char) -> Option<EscapeMode> {
    match character {
        'x' => Some(EscapeMode::Hex { digits: String::new() }),
        'u' => Some(EscapeMode::Unicode { digits: String::new(), started: false }),
        '\n' | '\r' => Some(EscapeMode::LineContinuation),
        _ => None,
    }
}

const fn simple_escape(character: char) -> Option<char> {
    match character {
        'n' => Some('\n'),
        'r' => Some('\r'),
        't' => Some('\t'),
        '\\' | '"' => Some(character),
        _ => None,
    }
}

const fn parse_hex_byte(digits: &str) -> Option<u8> {
    let Ok(byte) = u8::from_str_radix(digits, 16) else {
        return None;
    };
    Some(byte)
}

const fn parse_hex_codepoint(digits: &str) -> Option<u32> {
    let Ok(codepoint) = u32::from_str_radix(digits, 16) else {
        return None;
    };
    Some(codepoint)
}

fn push_decoded(mut decoded: String, character: char) -> (String, EscapeMode) {
    decoded.push(character);
    (decoded, EscapeMode::Plain)
}

fn parse_literal(source: &str) -> Option<LiteralParts<'_>> {
    if let Some(body) = source.strip_prefix('"') {
        let (content, tail) = body.split_once('"')?;
        return Some((content, tail, false));
    }
    let raw = source.strip_prefix('r')?;
    let hash_count = raw.chars().take_while(|ch| *ch == '#').count();
    let opening = format!("r{}\"", "#".repeat(hash_count));
    let body = source.strip_prefix(&opening)?;
    let closing = format!("\"{}", "#".repeat(hash_count));
    let (content, tail) = body.split_once(&closing)?;
    Some((content, tail, true))
}

const STD_RESULT_SPELLINGS: &[&str] = &["Result", "std::result::Result", "core::result::Result"];
const STD_STRING_SPELLINGS: &[&str] = &["String", "std::string::String", "alloc::string::String"];

fn is_result_with_string_error(node: &Node<'_, StrDoc<Rust>>) -> Option<()> {
    (node.kind() == "generic_type").then_some(())?;
    let mut named = node.children().filter(Node::is_named);
    let head_is_result = named.next().is_some_and(|t| type_path_is_std(&t, STD_RESULT_SPELLINGS));
    let args = named.next()?;
    (args.kind() == "type_arguments").then_some(())?;
    let mut arg_children = args.children().filter(Node::is_named);
    let _first_arg = arg_children.next();
    let second_is_string =
        arg_children.next().is_some_and(|t| type_path_is_std(&t, STD_STRING_SPELLINGS));
    let no_more_args = arg_children.next().is_none();
    (head_is_result && second_is_string && no_more_args).then_some(())
}

fn type_path_is_std(node: &Node<'_, StrDoc<Rust>>, spellings: &[&str]) -> bool {
    let text = node.text();
    let compact: String = text.chars().filter(|ch| !ch.is_whitespace()).collect();
    let normalized = match compact.strip_prefix("::") {
        Some(rest) => rest,
        None => &compact,
    };
    spellings.contains(&normalized)
}

fn function_has_excess_nesting(func: &Node<'_, StrDoc<Rust>>) -> Option<usize> {
    function_nesting_depth(func).filter(|&depth| depth > MAX_NESTING_DEPTH)
}

fn function_excess_nesting_site(node: &Node<'_, StrDoc<Rust>>) -> Option<MatchSite> {
    function_has_excess_nesting(node).map(|_| MatchSite::from_range(node.range()))
}

fn function_nesting_depth(func: &Node<'_, StrDoc<Rust>>) -> Option<usize> {
    let body = function_body_block(func)?;
    let body_id = body.node_id();
    Some(body.dfs().map(|current| control_flow_depth(&current, body_id)).fold(0, usize::max))
}
fn control_flow_depth(node: &Node<'_, StrDoc<Rust>>, body_id: usize) -> usize {
    self_increment(node).saturating_add(ancestor_count(node, body_id))
}

fn self_increment(node: &Node<'_, StrDoc<Rust>>) -> usize {
    usize::from(is_nesting_node(&node.kind()))
}

fn ancestor_count(node: &Node<'_, StrDoc<Rust>>, body_id: usize) -> usize {
    node.ancestors()
        .take_while(|a| a.node_id() != body_id)
        .filter(|a| is_nesting_node(&a.kind()))
        .count()
}

fn function_body_block<'a>(func: &Node<'a, StrDoc<Rust>>) -> Option<Node<'a, StrDoc<Rust>>> {
    func.children().find(|child| child.kind() == "block")
}

fn is_nesting_node(kind: &str) -> bool {
    matches!(
        kind,
        "if_expression"
            | "while_expression"
            | "for_expression"
            | "loop_expression"
            | "match_expression"
    )
}

fn function_calls_itself(func: &Node<'_, StrDoc<Rust>>) -> Option<()> {
    let name = function_name(func)?;
    func.dfs().any(|node| is_call_to(&node, &name)).then_some(())
}

fn function_recursion_site(node: &Node<'_, StrDoc<Rust>>) -> Option<MatchSite> {
    function_calls_itself(node).map(|()| MatchSite::from_range(node.range()))
}

fn function_name(func: &Node<'_, StrDoc<Rust>>) -> Option<String> {
    func.children().find(|child| child.kind() == "identifier").map(|id| id.text().into_owned())
}

fn is_call_to(node: &Node<'_, StrDoc<Rust>>, name: &str) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }
    node.children()
        .find(|child| is_callee_kind(&child.kind()))
        .is_some_and(|callee| callee_resolves_to(&callee, name))
}

fn is_callee_kind(kind: &str) -> bool {
    matches!(kind, "identifier" | "field_expression" | "scoped_identifier" | "generic_function")
}

const MAX_TURBOFISH_PEEL_DEPTH: usize = 32;

fn peel_generic_function<'r>(callee: &Node<'r, StrDoc<Rust>>) -> Node<'r, StrDoc<Rust>> {
    let mut current = callee.clone();
    for _peel in 0..MAX_TURBOFISH_PEEL_DEPTH {
        current = peel_step(current);
    }
    current
}

fn peel_step(current: Node<'_, StrDoc<Rust>>) -> Node<'_, StrDoc<Rust>> {
    generic_child(&current).map_or(current, |inner| inner)
}

fn generic_child<'r>(node: &Node<'r, StrDoc<Rust>>) -> Option<Node<'r, StrDoc<Rust>>> {
    match node.kind().as_ref() {
        "generic_function" => node.children().find(Node::is_named),
        _ => None,
    }
}

fn callee_resolves_to(callee: &Node<'_, StrDoc<Rust>>, name: &str) -> bool {
    let peeled = peel_generic_function(callee);
    match peeled.kind().as_ref() {
        "identifier" => peeled.text().trim() == name,
        "field_expression" => is_self_field_call(&peeled, name),
        "scoped_identifier" => {
            is_scoped_call(&peeled, "Self", name) || is_qualified_self_call(&peeled, name)
        }
        _ => false,
    }
}

fn is_qualified_self_call(callee: &Node<'_, StrDoc<Rust>>, name: &str) -> bool {
    let mut named = callee.children().filter(Node::is_named);
    let head_is_self = named.next().is_some_and(|head| bracketed_self_type(&head));
    let name_matches = named.next().is_some_and(|leaf| leaf.text().trim() == name);
    head_is_self && name_matches
}

fn bracketed_self_type(head: &Node<'_, StrDoc<Rust>>) -> bool {
    (head.kind() == "bracketed_type")
        && head
            .children()
            .find(Node::is_named)
            .is_some_and(|qualified| qualified_self_type(&qualified))
}

fn qualified_self_type(qualified: &Node<'_, StrDoc<Rust>>) -> bool {
    (qualified.kind() == "qualified_type")
        && qualified
            .children()
            .find(Node::is_named)
            .is_some_and(|self_type| self_type.text().trim() == "Self")
}

fn is_self_field_call(callee: &Node<'_, StrDoc<Rust>>, name: &str) -> bool {
    let mut named = callee.children().filter(Node::is_named);
    let object_is_self = named.next().is_some_and(|object| object.text().trim() == "self");
    let field_is_name = named.next().is_some_and(|field| field.text().trim() == name);
    object_is_self && field_is_name
}

fn is_scoped_call(callee: &Node<'_, StrDoc<Rust>>, scope: &str, name: &str) -> bool {
    let mut named = callee.children().filter(Node::is_named);
    let scope_matches = named.next().is_some_and(|head| head.text().trim() == scope);
    let name_matches = named.next().is_some_and(|leaf| leaf.text().trim() == name);
    scope_matches && name_matches
}

fn collect_test_ranges(root: &Node<'_, StrDoc<Rust>>) -> Vec<(usize, usize)> {
    root.dfs()
        .filter(|node| is_cfg_test_attribute(node))
        .filter_map(|attr| decorated_item_range(&attr))
        .collect()
}

fn is_cfg_test_attribute(node: &Node<'_, StrDoc<Rust>>) -> bool {
    if node.kind() != "attribute_item" {
        return false;
    }
    let text = node.text();
    text.contains("cfg(test)") || text.contains("cfg(all(test") || text.contains("cfg(any(test")
}

fn decorated_item_range(attr: &Node<'_, StrDoc<Rust>>) -> Option<(usize, usize)> {
    attr.next_all()
        .find(|sibling| sibling.kind() != "attribute_item")
        .map(|item| (item.range().start, item.range().end))
}
