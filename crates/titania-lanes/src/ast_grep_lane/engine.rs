use ast_grep_core::{AstGrep, Node, Pattern, tree_sitter::StrDoc};
use ast_grep_language::Rust;

use super::span::{MatchSite, line_offsets};

const MAX_NESTING_DEPTH: usize = 2;

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
        let cycle = Pattern::new("$ITER.cycle()", Rust);
        let repeat = Pattern::new("repeat($$$ARGS)", Rust);
        let repeat_with = Pattern::new("repeat_with($$$ARGS)", Rust);
        let successors = Pattern::new("successors($$$ARGS)", Rust);
        let from_fn = Pattern::new("from_fn($$$ARGS)", Rust);
        self.match_sites_any_skipping_tests([&cycle, &repeat, &repeat_with, &successors, &from_fn])
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
const STD_STRING_SPELLINGS: &[&str] = &["String", "std::string::String", "core::string::String"];

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
    spellings.iter().any(|spelling| compact == *spelling)
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

fn callee_resolves_to(callee: &Node<'_, StrDoc<Rust>>, name: &str) -> bool {
    match callee.kind().as_ref() {
        "generic_function" => callee
            .children()
            .find(Node::is_named)
            .is_some_and(|inner| callee_resolves_to(&inner, name)),
        "identifier" => callee.text().trim() == name,
        "field_expression" => is_self_field_call(callee, name),
        "scoped_identifier" => {
            is_scoped_call(callee, "Self", name) || is_qualified_self_call(callee, name)
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
