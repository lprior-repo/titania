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

    /// Convert a [`NodeMatch`] byte range into a 0-based source line index.
    fn line_of_match(&self, node_match: &NodeMatch<'_, StrDoc<Rust>>) -> usize {
        line_at_byte(&self.line_offsets, node_match.range().start)
    }
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

/// Compute the deepest block nesting inside a function body.
///
/// Counts nested control-flow blocks: `block`, `if_expression`,
/// `while_expression`, `for_expression`, `loop_expression`, `match_expression`,
/// and `match_arm`. Returns `None` for a function without a body block.
fn function_nesting_depth(func: &Node<'_, StrDoc<Rust>>) -> Option<usize> {
    let body = function_body_block(func)?;
    Some(max_block_depth(&body, 0))
}

/// Find the function body block (`{ ... }`) under a `function_item` node.
fn function_body_block<'a>(func: &Node<'a, StrDoc<Rust>>) -> Option<Node<'a, StrDoc<Rust>>> {
    func.children().find(|child| child.kind() == "block")
}

/// Maximum nesting depth of control-flow nodes starting at `node`.
fn max_block_depth(node: &Node<'_, StrDoc<Rust>>, depth: usize) -> usize {
    let kind_owned = node.kind().into_owned();
    let nested = if is_nesting_node(&kind_owned) { depth.saturating_add(1) } else { depth };
    match node.children().map(|child| max_block_depth(&child, nested)).max() {
        Some(child_max) if child_max >= nested => child_max,
        _ => nested,
    }
}

/// Kinds that increase control-flow nesting depth.
fn is_nesting_node(kind: &str) -> bool {
    matches!(
        kind,
        "block"
            | "if_expression"
            | "while_expression"
            | "for_expression"
            | "loop_expression"
            | "match_expression"
            | "match_arm"
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
        .find(|child| child.kind() == "identifier" || child.kind() == "field_expression")
        .is_some_and(|callee| callee.text().trim() == name)
}
