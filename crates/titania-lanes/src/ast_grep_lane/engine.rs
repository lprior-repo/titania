//! Real ast-grep engine wrapper for the v1 ast-grep lane.
//!
//! Wraps [`ast_grep_core::AstGrep`] over the Rust tree-sitter grammar and
//! exposes typed detection methods per rule. The engine returns the
//! 0-based line index of the first match (mirroring the legacy
//! `first_matching_line` contract) so the lane can emit typed findings
//! without re-scanning source text.
//!
//! Comment and string literals naturally do not produce AST nodes, so
//! matches inside comments/strings do not occur — the hand-rolled
//! comment/string stripping in `rules/detectors/code_scan` is no longer
//! consulted for rules ported here.

use ast_grep_core::{AstGrep, Node, NodeMatch, Pattern, tree_sitter::StrDoc};
use ast_grep_language::Rust;

/// Maximum block nesting depth permitted inside a function body before
/// `FUNC_NESTING_DEPTH` fires (depth > 2 is rejected). See v1-spec §6.
const MAX_NESTING_DEPTH: usize = 2;

/// Real ast-grep engine bound to a parsed Rust source file.
pub(super) struct AstEngine {
    grep: AstGrep<StrDoc<Rust>>,
    /// Byte offset of the start of each source line (line 0 = offset 0).
    line_offsets: Vec<usize>,
}

impl AstEngine {
    /// Parse `source` as Rust. ast-grep root construction is infallible;
    /// syntax errors surface as error nodes inside the tree, which means
    /// engine detectors simply fail to matches against malformed subtrees.
    pub(super) fn new(source: &str) -> Self {
        let grep = AstGrep::new(source, Rust);
        Self { grep, line_offsets: line_offsets(source) }
    }

    /// 0-based line index of the first pattern match, or `None`.
    fn first_match_line(&self, pattern: &Pattern) -> Option<usize> {
        let node_match = self.grep.root().find_all(pattern).next()?;
        Some(self.line_of_match(&node_match))
    }

    fn line_of_match(&self, node_match: &NodeMatch<'_, StrDoc<Rust>>) -> usize {
        line_at_byte(&self.line_offsets, node_match.range().start)
    }

    /// 0-based line index of the first match across an iterator of patterns.
    ///
    /// Used for rules expressed as `any: [pattern, pattern, ...]` in YAML.
    fn first_match_line_any<'p>(
        &self,
        patterns: impl IntoIterator<Item = &'p Pattern>,
    ) -> Option<usize> {
        patterns.into_iter().filter_map(|pat| self.first_match_line(pat)).min()
    }

    /// Line of the first `for $LOOP in $ITER { $$$BODY }` match (`FUNC_LOOPS_FOR`).
    pub(super) fn detect_for_loop(&self) -> Option<usize> {
        let pat = Pattern::new("for $LOOP in $ITER { $$$BODY }", Rust);
        self.first_match_line(&pat)
    }

    /// Line of the first `while $COND { $$$BODY }` match (`FUNC_LOOPS_WHILE`).
    pub(super) fn detect_while_loop(&self) -> Option<usize> {
        let pat = Pattern::new("while $COND { $$$BODY }", Rust);
        self.first_match_line(&pat)
    }

    /// Line of the first `loop { $$$BODY }` match (`FUNC_LOOPS_LOOP`).
    pub(super) fn detect_loop_block(&self) -> Option<usize> {
        let pat = Pattern::new("loop { $$$BODY }", Rust);
        self.first_match_line(&pat)
    }

    /// Line of the first `print!`/`println!` match (`FUNC_PRINT_STDOUT`).
    pub(super) fn detect_print_stdout(&self) -> Option<usize> {
        let print = Pattern::new("print!($$$ARGS)", Rust);
        let println = Pattern::new("println!($$$ARGS)", Rust);
        self.first_match_line_any([&print, &println])
    }

    /// Line of the first `eprint!`/`eprintln!` match (`FUNC_PRINT_STDERR`).
    pub(super) fn detect_print_stderr(&self) -> Option<usize> {
        let eprint = Pattern::new("eprint!($$$ARGS)", Rust);
        let eprintln = Pattern::new("eprintln!($$$ARGS)", Rust);
        self.first_match_line_any([&eprint, &eprintln])
    }

    /// Line of the first `use $PATH::*;` match (`FUNC_WILDCARD_IMPORT`).
    pub(super) fn detect_wildcard_import(&self) -> Option<usize> {
        let pat = Pattern::new("use $PATH::*;", Rust);
        self.first_match_line(&pat)
    }

    /// Line of the first `.unwrap_or(_)`, `.unwrap_or_else(_)`, or
    /// `.unwrap_or_default()` match (`FUNC_UNWRAP_OR`).
    pub(super) fn detect_unwrap_or(&self) -> Option<usize> {
        let or = Pattern::new("$VALUE.unwrap_or($DEFAULT)", Rust);
        let or_else = Pattern::new("$VALUE.unwrap_or_else($DEFAULT)", Rust);
        let or_default = Pattern::new("$VALUE.unwrap_or_default()", Rust);
        self.first_match_line_any([&or, &or_else, &or_default])
    }

    /// Line of the first `Result<_, String>` `generic_type` node (`FUNC_RESULT_STRING`).
    ///
    /// ast-grep patterns cannot express a partial type-argument match
    /// cleanly across all grammars, so we walk `generic_type` nodes and
    /// inspect the type arguments manually.
    pub(super) fn detect_result_string(&self) -> Option<usize> {
        first_match_node_line(&self.grep.root(), is_result_with_string_error)
    }

    /// Line of the first function whose body nests deeper than
    /// [`MAX_NESTING_DEPTH`] (`FUNC_NESTING_DEPTH`, new in §6).
    pub(super) fn detect_nesting_depth(&self) -> Option<usize> {
        first_function_match(&self.grep.root(), function_has_excess_nesting)
    }

    /// Line of the first function whose body calls itself by name
    /// (`FUNC_RECURSION_DIRECT`, new in §6).
    pub(super) fn detect_recursion_direct(&self) -> Option<usize> {
        first_function_match(&self.grep.root(), function_calls_itself)
    }

    /// Line of the first `#[allow($$$LINTS)]` item attribute (`BYPASS_ALLOW_ATTR`).
    pub(super) fn detect_allow_attr(&self) -> Option<usize> {
        let pat = Pattern::new("#[allow($$$LINTS)]", Rust);
        self.first_match_line(&pat)
    }

    /// Line of the first `#[expect($$$LINTS)]` item attribute (`BYPASS_EXPECT_ATTR`).
    pub(super) fn detect_expect_attr(&self) -> Option<usize> {
        let pat = Pattern::new("#[expect($$$LINTS)]", Rust);
        self.first_match_line(&pat)
    }

    /// Line of the first `#[cfg_attr($COND, allow($$$LINTS))]` (`BYPASS_CFG_ATTR_ALLOW`).
    pub(super) fn detect_cfg_attr_allow(&self) -> Option<usize> {
        let pat = Pattern::new("#[cfg_attr($COND, allow($$$LINTS))]", Rust);
        self.first_match_line(&pat)
    }

    /// Line of the first `#![allow($$$LINTS)]` crate attribute (`BYPASS_CRATE_ALLOW`).
    pub(super) fn detect_crate_allow(&self) -> Option<usize> {
        let pat = Pattern::new("#![allow($$$LINTS)]", Rust);
        self.first_match_line(&pat)
    }

    /// Line of the first `#![expect($$$LINTS)]` crate attribute (`BYPASS_CRATE_EXPECT`).
    pub(super) fn detect_crate_expect(&self) -> Option<usize> {
        let pat = Pattern::new("#![expect($$$LINTS)]", Rust);
        self.first_match_line(&pat)
    }

    /// Line of the first generated-code include using `OUT_DIR`.
    pub(super) fn detect_generated_include(&self) -> Option<usize> {
        let concat_pattern = Pattern::new("concat!($$$PARTS)", Rust);
        let direct_pattern = Pattern::new("include!(concat!(env!($ENV), $PATH))", Rust);
        let message_pattern =
            Pattern::new("include!(concat!(env!($ENV, $$$MESSAGE), $PATH))", Rust);
        let many_pattern = Pattern::new("include!(concat!(env!($ENV), $$$PATHS))", Rust);
        let many_message_pattern =
            Pattern::new("include!(concat!(env!($ENV, $$$MESSAGE), $$$PATHS))", Rust);
        [
            single_include_line(&self.grep, &direct_pattern, &concat_pattern),
            single_include_line(&self.grep, &message_pattern, &concat_pattern),
            many_include_line(&self.grep, &many_pattern, &concat_pattern),
            many_include_line(&self.grep, &many_message_pattern, &concat_pattern),
        ]
        .into_iter()
        .flatten()
        .min()
    }
}

fn is_out_dir_expression(node: &Node<'_, StrDoc<Rust>>, concat_pattern: &Pattern) -> bool {
    if is_out_dir_node(node) {
        return true;
    }
    decode_concat_expression(node, concat_pattern).is_some_and(|decoded| decoded == "OUT_DIR")
}

/// Decode every `concat!` part under `node` into a single string, returning
/// `None` when any part is malformed or contains trailing junk.
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

/// Append a single `concat!` part to `output` after decoding it. Returns
/// `None` if the part is not a recognizable string literal, has trailing
/// content, or fails escape decoding.
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

/// Line of the first `include!(concat!(env!($ENV), $PATH))` match where
/// `env!` resolves to `OUT_DIR` and `$PATH` is non-empty.
fn single_include_line(
    grep: &AstGrep<StrDoc<Rust>>,
    pattern: &Pattern,
    concat_pattern: &Pattern,
) -> Option<usize> {
    grep.root().find_all(pattern).find_map(|node_match| {
        let environment = node_match.get_env().get_match("ENV")?;
        let path = node_match.get_env().get_match("PATH")?;
        (is_out_dir_expression(environment, concat_pattern) && path_non_empty(path))
            .then_some(node_match.start_pos().line())
    })
}

/// Line of the first `include!(concat!(env!($ENV), $$$PATHS))` match where
/// `env!` resolves to `OUT_DIR` and at least one of the `$$$PATHS` is
/// non-empty.
fn many_include_line(
    grep: &AstGrep<StrDoc<Rust>>,
    pattern: &Pattern,
    concat_pattern: &Pattern,
) -> Option<usize> {
    grep.root().find_all(pattern).find_map(|node_match| {
        let environment = node_match.get_env().get_match("ENV")?;
        let paths = node_match.get_env().get_multiple_matches("PATHS");
        (!paths.is_empty()
            && is_out_dir_expression(environment, concat_pattern)
            && paths.iter().any(path_non_empty))
        .then_some(node_match.start_pos().line())
    })
}

/// `true` when `node`'s source text is not pure whitespace.
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

/// Components of a Rust string literal: `(content, trailing, raw)`.
///
/// `content` is the raw text between the opening and closing quotes,
/// `trailing` is whatever follows the closing quote (used to detect
/// literals that have junk after them, e.g. `"foo"bar`), and `raw` is
/// `true` for raw literals like `r"..."` / `r#"..."#`.
type LiteralParts<'a> = (&'a str, &'a str, bool);

/// State machine driving the `decode_literal` fold.
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

/// Decode a Rust string literal into its escaped form. `raw` literals are
/// returned verbatim; cooked literals are processed through the
/// `EscapeMode` state machine below.
fn decode_literal(literal: &str, raw: bool) -> Option<String> {
    if raw {
        return Some(literal.to_owned());
    }
    let (decoded, mode) = fold_escape_sequence(literal)?;
    matches!(mode, EscapeMode::Plain).then_some(decoded)
}

/// Walk every character of `literal` through the escape state machine,
/// short-circuiting on the first malformed sequence. The final state is
/// returned so the caller can reject dangling escape runs.
fn fold_escape_sequence(literal: &str) -> Option<(String, EscapeMode)> {
    literal.chars().fold(Some((String::new(), EscapeMode::Plain)), |state, character| {
        state.and_then(|current| advance_escape(current, character))
    })
}

/// Dispatch a single character through the escape state machine. Each
/// [`EscapeMode`] variant delegates to a per-mode handler so no arm of
/// the dispatcher grows past a handful of lines.
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

/// `Plain` mode: append the character, unless it is `\\` which opens an
/// escape sequence.
fn plain_state(decoded: String, character: char) -> (String, EscapeMode) {
    if character == '\\' {
        (decoded, EscapeMode::AfterSlash)
    } else {
        push_decoded(decoded, character)
    }
}

/// `AfterSlash` mode: classify the escape and either enter a sub-state or
/// emit a single decoded character. The branches that share `push_decoded`
/// are collapsed into a `simple_escape` lookup.
fn after_slash(decoded: String, character: char) -> Option<(String, EscapeMode)> {
    if let Some(next) = control_escape(character) {
        return Some((decoded, next));
    }
    let mapped = simple_escape(character)?;
    Some(push_decoded(decoded, mapped))
}

/// `LineContinuation` mode: skip whitespace until a non-blank character
/// terminates the run and re-enters `Plain` mode.
fn line_continuation_state(decoded: String, character: char) -> (String, EscapeMode) {
    if character.is_whitespace() {
        (decoded, EscapeMode::LineContinuation)
    } else {
        push_decoded(decoded, character)
    }
}

/// `Hex` mode: collect up to two hex digits and emit the byte.
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

/// `Unicode` mode: collect hex digits (and optional `_` separators)
/// between `{` and `}` and emit the codepoint.
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

/// Escape sequence that opens a sub-state (no character is emitted).
const fn control_escape(character: char) -> Option<EscapeMode> {
    match character {
        'x' => Some(EscapeMode::Hex { digits: String::new() }),
        'u' => Some(EscapeMode::Unicode { digits: String::new(), started: false }),
        '\n' | '\r' => Some(EscapeMode::LineContinuation),
        _ => None,
    }
}

/// Single-character escape mapping used by `after_slash`.
const fn simple_escape(character: char) -> Option<char> {
    match character {
        'n' => Some('\n'),
        'r' => Some('\r'),
        't' => Some('\t'),
        '\\' | '"' => Some(character),
        _ => None,
    }
}

/// Parse exactly two hex digits into a byte. Returns `None` on parse
/// failure rather than a discarded `ParseIntError`.
fn parse_hex_byte(digits: &str) -> Option<u8> {
    u8::from_str_radix(digits, 16).ok()
}

/// Parse the hex digits inside `\u{...}` into a Unicode codepoint.
/// Returns `None` on parse failure rather than a discarded
/// `ParseIntError`.
fn parse_hex_codepoint(digits: &str) -> Option<u32> {
    u32::from_str_radix(digits, 16).ok()
}

/// Append a decoded character to the buffer and reset to the
/// `EscapeMode::Plain` state. The character is always accepted, so this
/// helper is infallible.
fn push_decoded(mut decoded: String, character: char) -> (String, EscapeMode) {
    decoded.push(character);
    (decoded, EscapeMode::Plain)
}

/// Split a Rust literal source slice into its `(content, tail, raw)`
/// components. `raw` is `true` for `r"..."` / `r#"..."#` literals.
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

/// Compute byte offsets for the start of every source line.
fn line_offsets(source: &str) -> Vec<usize> {
    let mut offsets: Vec<usize> = vec![0];
    offsets.extend(source.match_indices('\n').map(|(i, _)| i.saturating_add(1)));
    offsets
}

/// 0-based line index containing `byte`, using a binary search over offsets.
fn line_at_byte(offsets: &[usize], byte: usize) -> usize {
    offsets.partition_point(|&offset| offset <= byte).saturating_sub(1)
}

/// Function pointer predicate over an ast-grep node.
type NodePredicate<T> = fn(&Node<'_, StrDoc<Rust>>) -> Option<T>;

/// Walk the tree depth-first; return the line of the first node where `pred`
/// returns `Some(T)`.
fn first_match_node_line<T>(
    root: &Node<'_, StrDoc<Rust>>,
    pred: NodePredicate<T>,
) -> Option<usize> {
    root.dfs().find_map(|node| pred(&node).map(|_| node.start_pos().line()))
}

/// Walk function items; return the line of the first function for which
/// `pred` returns `Some(T)`.
fn first_function_match<T>(root: &Node<'_, StrDoc<Rust>>, pred: NodePredicate<T>) -> Option<usize> {
    root.dfs()
        .filter(|node| node.kind() == "function_item")
        .find_map(|node| pred(&node).map(|_| node.start_pos().line()))
}

/// `Some(())` when `node` is a `generic_type` whose head identifier is
/// `Result` and whose second type argument is exactly the path `String`.
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

/// True when a type node's source text equals `name` (modulo whitespace).
fn type_text_is(node: &Node<'_, StrDoc<Rust>>, name: &str) -> bool {
    node.text().trim() == name
}

/// `Some(depth)` only when the function body's deepest nesting exceeds the limit.
fn function_has_excess_nesting(func: &Node<'_, StrDoc<Rust>>) -> Option<usize> {
    function_nesting_depth(func).filter(|&depth| depth > MAX_NESTING_DEPTH)
}

/// Compute the deepest control-flow nesting inside a function body.
///
/// Counts nested control-flow nodes: `if_expression`, `while_expression`,
/// `for_expression`, `loop_expression`, `match_expression`. Plain `block`
/// nodes (the function body and the bodies of `if`/`for`/etc. arms) do not
/// contribute — only the control-flow constructs themselves do, so a
/// simple `if` in the function body stays at depth 1. Returns `None` for a
/// function without a body block.
fn function_nesting_depth(func: &Node<'_, StrDoc<Rust>>) -> Option<usize> {
    let body = function_body_block(func)?;
    let body_id = body.node_id();
    Some(body.dfs().map(|current| control_flow_depth(&current, body_id)).fold(0, usize::max))
}
/// Depth of a single node: self (1 if control-flow) + control-flow ancestors.
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

/// Find the function body block (`{ ... }`) under a `function_item` node.
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

/// `Some(())` when a `function_item` contains a `call_expression` whose
/// callee is the function's own identifier.
fn function_calls_itself(func: &Node<'_, StrDoc<Rust>>) -> Option<()> {
    let name = function_name(func)?;
    func.dfs().any(|node| is_call_to(&node, &name)).then_some(())
}

/// Extract a function's identifier from its `function_item` node.
fn function_name(func: &Node<'_, StrDoc<Rust>>) -> Option<String> {
    func.children().find(|child| child.kind() == "identifier").map(|id| id.text().into_owned())
}

/// True when `node` is a `call_expression` whose callee path equals `name`.
fn is_call_to(node: &Node<'_, StrDoc<Rust>>, name: &str) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }
    node.children()
        .find(|child| is_callee_kind(&child.kind()))
        .is_some_and(|callee| callee.text().trim() == name)
}

/// AST node kinds that may serve as the callee of a `call_expression`.
fn is_callee_kind(kind: &str) -> bool {
    matches!(kind, "identifier" | "field_expression")
}
