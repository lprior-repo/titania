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

    fn first_match_site(&self, pattern: &Pattern) -> Option<MatchSite> {
        let node_match = self.grep.root().find_all(pattern).next()?;
        Some(MatchSite::from_range(node_match.range()))
    }

    fn first_match_site_skipping_tests(&self, pattern: &Pattern) -> Option<MatchSite> {
        self.grep
            .root()
            .find_all(pattern)
            .find(|node_match| !self.is_in_test_code(node_match.range().start))
            .map(|node_match| MatchSite::from_range(node_match.range()))
    }

    fn first_match_site_any_skipping_tests<'p>(
        &self,
        patterns: impl IntoIterator<Item = &'p Pattern>,
    ) -> Option<MatchSite> {
        patterns
            .into_iter()
            .filter_map(|pat| self.first_match_site_skipping_tests(pat))
            .min_by_key(|site| site.start_byte)
    }

    pub(super) fn detect_unbounded_loop(&self) -> Option<MatchSite> {
        let loop_pat = Pattern::new("loop { $$$BODY }", Rust);
        let while_pat = Pattern::new("while $COND { $$$BODY }", Rust);
        let loop_site = self.first_match_site_skipping_tests(&loop_pat);
        let while_site = self
            .grep
            .root()
            .find_all(&while_pat)
            .filter(|node_match| !self.is_in_test_code(node_match.range().start))
            .filter(|node_match| !is_while_let_match(node_match))
            .map(|node_match| MatchSite::from_range(node_match.range()))
            .min_by_key(|site| site.start_byte);
        loop_site.into_iter().chain(while_site).min_by_key(|site| site.start_byte)
    }

    pub(super) fn detect_unbounded_iterator(&self) -> Option<MatchSite> {
        let cycle = Pattern::new("$ITER.cycle()", Rust);
        let repeat = Pattern::new("repeat($$$ARGS)", Rust);
        let repeat_with = Pattern::new("repeat_with($$$ARGS)", Rust);
        let successors = Pattern::new("successors($$$ARGS)", Rust);
        let from_fn = Pattern::new("from_fn($$$ARGS)", Rust);
        self.first_match_site_any_skipping_tests([
            &cycle,
            &repeat,
            &repeat_with,
            &successors,
            &from_fn,
        ])
    }

    pub(super) fn detect_print_stdout(&self) -> Option<MatchSite> {
        let print = Pattern::new("print!($$$ARGS)", Rust);
        let println = Pattern::new("println!($$$ARGS)", Rust);
        self.first_match_site_any_skipping_tests([&print, &println])
    }

    pub(super) fn detect_print_stderr(&self) -> Option<MatchSite> {
        let eprint = Pattern::new("eprint!($$$ARGS)", Rust);
        let eprintln = Pattern::new("eprintln!($$$ARGS)", Rust);
        self.first_match_site_any_skipping_tests([&eprint, &eprintln])
    }

    pub(super) fn detect_wildcard_import(&self) -> Option<MatchSite> {
        let pat = Pattern::new("use $PATH::*;", Rust);
        self.first_match_site_skipping_tests(&pat)
    }

    pub(super) fn detect_unwrap_or(&self) -> Option<MatchSite> {
        let or = Pattern::new("$VALUE.unwrap_or($DEFAULT)", Rust);
        let or_else = Pattern::new("$VALUE.unwrap_or_else($DEFAULT)", Rust);
        let or_default = Pattern::new("$VALUE.unwrap_or_default()", Rust);
        self.first_match_site_any_skipping_tests([&or, &or_else, &or_default])
    }

    pub(super) fn detect_result_string(&self) -> Option<MatchSite> {
        self.grep.root().dfs().filter(|node| !self.is_in_test_code(node.range().start)).find_map(
            |node| is_result_with_string_error(&node).map(|()| MatchSite::from_range(node.range())),
        )
    }

    pub(super) fn detect_nesting_depth(&self) -> Option<MatchSite> {
        self.grep
            .root()
            .dfs()
            .filter(|node| node.kind() == "function_item")
            .filter(|node| !self.is_in_test_code(node.range().start))
            .find_map(|node| function_excess_nesting_site(&node))
    }

    pub(super) fn detect_recursion_direct(&self) -> Option<MatchSite> {
        self.grep
            .root()
            .dfs()
            .filter(|node| node.kind() == "function_item")
            .filter(|node| !self.is_in_test_code(node.range().start))
            .find_map(|node| function_recursion_site(&node))
    }

    pub(super) fn detect_allow_attr(&self) -> Option<MatchSite> {
        let pat = Pattern::new("#[allow($$$LINTS)]", Rust);
        self.first_match_site(&pat)
    }

    pub(super) fn detect_expect_attr(&self) -> Option<MatchSite> {
        let pat = Pattern::new("#[expect($$$LINTS)]", Rust);
        self.first_match_site(&pat)
    }

    pub(super) fn detect_cfg_attr_allow(&self) -> Option<MatchSite> {
        let pat = Pattern::new("#[cfg_attr($COND, allow($$$LINTS))]", Rust);
        self.first_match_site(&pat)
    }

    pub(super) fn detect_crate_allow(&self) -> Option<MatchSite> {
        let pat = Pattern::new("#![allow($$$LINTS)]", Rust);
        self.first_match_site(&pat)
    }

    pub(super) fn detect_crate_expect(&self) -> Option<MatchSite> {
        let pat = Pattern::new("#![expect($$$LINTS)]", Rust);
        self.first_match_site(&pat)
    }

    pub(super) fn detect_generated_include(&self) -> Option<MatchSite> {
        let concat_pattern = Pattern::new("concat!($$$PARTS)", Rust);
        let direct_pattern = Pattern::new("include!(concat!(env!($ENV), $PATH))", Rust);
        let message_pattern =
            Pattern::new("include!(concat!(env!($ENV, $$$MESSAGE), $PATH))", Rust);
        let many_pattern = Pattern::new("include!(concat!(env!($ENV), $$$PATHS))", Rust);
        let many_message_pattern =
            Pattern::new("include!(concat!(env!($ENV, $$$MESSAGE), $$$PATHS))", Rust);
        [
            single_include_site(&self.grep, &direct_pattern, &concat_pattern),
            single_include_site(&self.grep, &message_pattern, &concat_pattern),
            many_include_site(&self.grep, &many_pattern, &concat_pattern),
            many_include_site(&self.grep, &many_message_pattern, &concat_pattern),
        ]
        .into_iter()
        .flatten()
        .min_by_key(|site| site.start_byte)
    }
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

fn single_include_site(
    grep: &AstGrep<StrDoc<Rust>>,
    pattern: &Pattern,
    concat_pattern: &Pattern,
) -> Option<MatchSite> {
    grep.root().find_all(pattern).find_map(|node_match| {
        let environment = node_match.get_env().get_match("ENV")?;
        let path = node_match.get_env().get_match("PATH")?;
        (is_out_dir_expression(environment, concat_pattern) && path_non_empty(path))
            .then(|| MatchSite::from_range(node_match.range()))
    })
}

fn many_include_site(
    grep: &AstGrep<StrDoc<Rust>>,
    pattern: &Pattern,
    concat_pattern: &Pattern,
) -> Option<MatchSite> {
    grep.root().find_all(pattern).find_map(|node_match| {
        let environment = node_match.get_env().get_match("ENV")?;
        let paths = node_match.get_env().get_multiple_matches("PATHS");
        (!paths.is_empty()
            && is_out_dir_expression(environment, concat_pattern)
            && paths.iter().any(path_non_empty))
        .then(|| MatchSite::from_range(node_match.range()))
    })
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

fn parse_hex_byte(digits: &str) -> Option<u8> {
    u8::from_str_radix(digits, 16).ok()
}

fn parse_hex_codepoint(digits: &str) -> Option<u32> {
    u32::from_str_radix(digits, 16).ok()
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

fn is_result_with_string_error(node: &Node<'_, StrDoc<Rust>>) -> Option<()> {
    (node.kind() == "generic_type").then_some(())?;
    let mut named = node.children().filter(Node::is_named);
    let head_is_result = named.next().is_some_and(|t| type_text_is(&t, "Result"));
    let args = named.next()?;
    (args.kind() == "type_arguments").then_some(())?;
    let mut arg_children = args.children().filter(Node::is_named);
    let _first_arg = arg_children.next();
    let second_is_string = arg_children.next().is_some_and(|t| type_text_is(&t, "String"));
    let no_more_args = arg_children.next().is_none();
    (head_is_result && second_is_string && no_more_args).then_some(())
}

fn type_text_is(node: &Node<'_, StrDoc<Rust>>, name: &str) -> bool {
    node.text().trim() == name
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
        .is_some_and(|callee| callee.text().trim() == name)
}

fn is_callee_kind(kind: &str) -> bool {
    matches!(kind, "identifier" | "field_expression")
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
