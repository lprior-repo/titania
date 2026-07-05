//! String-backed detectors for the embedded v1 ast-grep rule table.
mod code_scan;

use code_scan::{code_only_source, detect_code_line, first_code_line};

pub(super) fn first_matching_line(source: &str, detects: fn(&str) -> bool) -> Option<usize> {
    first_code_line(source, detects).or_else(|| fallback_first_code_line(source, detects))
}

fn fallback_first_code_line(source: &str, detects: fn(&str) -> bool) -> Option<usize> {
    if !detects(source) {
        return None;
    }
    first_code_line(source, has_code).or(Some(0))
}

fn has_code(line: &str) -> bool {
    !line.trim().is_empty()
}

pub(super) fn detect_for_loop(source: &str) -> bool {
    detect_code_line(source, has_for_loop_tokens)
}

pub(super) fn detect_while_loop(source: &str) -> bool {
    detect_code_line(source, |line| line.contains("while "))
}

pub(super) fn detect_loop_block(source: &str) -> bool {
    detect_code_line(source, |line| line.contains("loop {"))
}

fn has_for_loop_tokens(line: &str) -> bool {
    line.contains("for ") && line.contains(" in ")
}

pub(super) fn detect_print_stdout(source: &str) -> bool {
    detect_code_line(source, |line| {
        has_print_macro_boundary(line, "print!(") || has_print_macro_boundary(line, "println!(")
    })
}

/// Check that `needle` appears in `line` with a non-alphanumeric/non-`_` boundary.
/// Also rejects `eprint!`/`eprintln!` when checking for `print!`/`println!`.
fn has_print_macro_boundary(line: &str, needle: &str) -> bool {
    line.match_indices(needle).any(|(start, _)| {
        let before_start = start.saturating_sub(1);
        line.as_bytes()
            .get(before_start)
            .is_none_or(|&b| !b.is_ascii_alphanumeric() && b != b'_' && b != b'e')
    })
}

pub(super) fn detect_print_stderr(source: &str) -> bool {
    detect_code_line(source, |line| {
        has_print_macro_boundary(line, "eprint!(") || has_print_macro_boundary(line, "eprintln!(")
    })
}

pub(super) fn detect_wildcard_import(source: &str) -> bool {
    detect_code_line(source, |line| line.contains("::*;"))
}

pub(super) fn detect_unwrap_or(source: &str) -> bool {
    detect_code_line(source, |line| {
        line.contains(".unwrap_or(")
            || line.contains(".unwrap_or_else(")
            || line.contains(".unwrap_or_default()")
    })
}

pub(super) fn detect_result_string(source: &str) -> bool {
    let code = code_only_source(source);
    code.match_indices("Result")
        .filter_map(|(start, _)| result_generic_tail(&code, start))
        .any(result_tail_error_is_string)
}

fn result_generic_tail(source: &str, start: usize) -> Option<&str> {
    let (before, from_result) = source.split_at_checked(start)?;
    result_name_boundary(before).then_some(())?;
    from_result.strip_prefix("Result")?.trim_start().strip_prefix('<')
}

fn result_name_boundary(before_result: &str) -> bool {
    before_result.chars().next_back().is_none_or(|ch| !ch.is_ascii_alphanumeric() && ch != '_')
}

fn result_tail_error_is_string(tail: &str) -> bool {
    match tail.chars().try_fold(ResultTypeScan::new(), ResultTypeScan::accept) {
        std::ops::ControlFlow::Break(error) => {
            error.trim().trim_end_matches(',').trim() == "String"
        }
        std::ops::ControlFlow::Continue(_) => false,
    }
}

struct ResultTypeScan {
    depth: u8,
    after_top_level_comma: bool,
    error_argument: String,
}

impl ResultTypeScan {
    const fn new() -> Self {
        Self { depth: 0, after_top_level_comma: false, error_argument: String::new() }
    }

    fn accept(self, ch: char) -> std::ops::ControlFlow<String, Self> {
        match ch {
            '<' => self.nested_open(ch),
            '>' => self.nested_close(ch),
            ',' if self.depth == 0 && !self.after_top_level_comma => self.top_level_comma(),
            _ => self.push_error_char(ch),
        }
    }

    fn nested_open(mut self, ch: char) -> std::ops::ControlFlow<String, Self> {
        self.depth = self.depth.saturating_add(1);
        self.push_error_char(ch)
    }

    fn nested_close(self, ch: char) -> std::ops::ControlFlow<String, Self> {
        match self.depth.checked_sub(1) {
            Some(depth) => self.with_depth(depth).push_error_char(ch),
            None => std::ops::ControlFlow::Break(self.error_argument),
        }
    }

    const fn with_depth(mut self, depth: u8) -> Self {
        self.depth = depth;
        self
    }

    fn top_level_comma(mut self) -> std::ops::ControlFlow<String, Self> {
        self.after_top_level_comma = true;
        self.error_argument.clear();
        std::ops::ControlFlow::Continue(self)
    }

    fn push_error_char(mut self, ch: char) -> std::ops::ControlFlow<String, Self> {
        self.push_optional_error_char(self.after_top_level_comma.then_some(ch));
        std::ops::ControlFlow::Continue(self)
    }

    fn push_optional_error_char(&mut self, ch: Option<char>) {
        self.error_argument.extend(ch);
    }
}

pub(super) fn detect_allow_attr(source: &str) -> bool {
    detect_code_line(source, |line| line.contains("#[allow("))
}

pub(super) fn detect_expect_attr(source: &str) -> bool {
    detect_code_line(source, |line| line.contains("#[expect("))
}

pub(super) fn detect_cfg_attr_allow(source: &str) -> bool {
    detect_code_line(source, |line| line.contains("#[cfg_attr(") && line.contains("allow("))
}

pub(super) fn detect_crate_allow(source: &str) -> bool {
    detect_code_line(source, |line| line.contains("#![allow("))
}

pub(super) fn detect_crate_expect(source: &str) -> bool {
    detect_code_line(source, |line| line.contains("#![expect("))
}

pub(super) fn detect_inline_suppression(source: &str) -> bool {
    source.lines().any(line_comment_contains_inline_suppression)
}

fn line_comment_contains_inline_suppression(line: &str) -> bool {
    line.split_once("//")
        .map(|(_, comment)| comment)
        .is_some_and(contains_inline_suppression_marker)
        || line
            .split_once("/*")
            .map(|(_, comment)| comment)
            .is_some_and(contains_inline_suppression_marker)
}

fn contains_inline_suppression_marker(comment: &str) -> bool {
    comment.contains("ast-grep-ignore") || comment.contains("sg-ignore")
}

pub(super) fn detect_core_infra_import(source: &str) -> bool {
    ["use tokio::", "use axum::", "use sqlx::", "use reqwest::"]
        .iter()
        .any(|needle| source.contains(needle))
}

pub(super) fn detect_core_fs_import(source: &str) -> bool {
    direct_import_contains(source, "std::", &["fs", "env", "net"])
        || grouped_std_import_contains(source, &["fs", "env", "net"])
}

pub(super) fn detect_core_time_import(source: &str) -> bool {
    direct_import_contains(source, "std::time::", &["SystemTime", "Instant"])
        || grouped_import_contains(source, "use std::time::", &["SystemTime", "Instant"])
}

pub(super) fn detect_core_random_import(source: &str) -> bool {
    direct_import_contains(source, "rand::", &["thread_rng", "Rng"])
        || grouped_rand_import_contains(source, &["thread_rng", "Rng"])
}

fn direct_import_contains(source: &str, path_prefix: &str, names: &[&str]) -> bool {
    source
        .match_indices("use ")
        .filter_map(|(start, _)| direct_import_member(source, start, path_prefix))
        .any(|member| names.iter().any(|name| grouped_member_matches(member, name)))
}

fn direct_import_member<'a>(source: &'a str, start: usize, path_prefix: &str) -> Option<&'a str> {
    let (_, from_use) = source.split_at_checked(start)?;
    use_item_prefix_allowed(source, start).then_some(())?;
    Some(from_use.strip_prefix("use ")?.strip_prefix(path_prefix)?.split_once(';')?.0.trim())
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
}

fn grouped_std_import_contains(source: &str, names: &[&str]) -> bool {
    grouped_import_contains(source, "use std::", names)
}

fn grouped_rand_import_contains(source: &str, names: &[&str]) -> bool {
    grouped_import_contains(source, "use rand::", names)
}

fn grouped_import_contains(source: &str, prefix: &str, names: &[&str]) -> bool {
    source
        .match_indices(prefix)
        .filter_map(|(start, _)| grouped_import_body(source, start, prefix))
        .any(|body| names.iter().any(|name| grouped_body_contains_name(body, name)))
}

fn grouped_import_body<'a>(source: &'a str, start: usize, prefix: &str) -> Option<&'a str> {
    let (_, from_prefix) = source.split_at_checked(start)?;
    use_item_prefix_allowed(source, start).then_some(())?;
    Some(from_prefix.strip_prefix(prefix)?.trim_start().strip_prefix('{')?.split_once('}')?.0)
}

fn grouped_body_contains_name(body: &str, name: &str) -> bool {
    body.split(',').map(str::trim).any(|member| grouped_member_matches(member, name))
}

fn grouped_member_matches(member: &str, name: &str) -> bool {
    member
        .strip_prefix(name)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with("::") || rest.starts_with(" as "))
}
