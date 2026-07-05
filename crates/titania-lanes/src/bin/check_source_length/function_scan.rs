use std::path::Path;

use titania_lanes::{
    Finding, LaneReport, RuleId, RuleIdError,
    helpers::{brace_delta, line_no_from_idx, relative_path, saturating_add_usize},
};

use crate::FN_LINE_LIMIT;

const FN_LINE_LIMIT_RULE: &str = "FN_LINE_LIMIT";

#[derive(Clone, Copy)]
enum ScanState {
    Outside,
    Inside { start_line: u32, count: usize, depth: i32 },
}

/// Check one Rust source file for oversized functions.
///
/// # Errors
///
/// Returns a rule-id construction error when the function length rule id is
/// invalid.
pub(super) fn check_file(
    root: &Path,
    file: &Path,
    report: &mut LaneReport,
) -> Result<(), RuleIdError> {
    let Ok(text) = std::fs::read_to_string(file) else {
        return Ok(());
    };
    let rel = relative_path(root, file);
    let rule = RuleId::new(FN_LINE_LIMIT_RULE)?;
    scan_functions(&text, &rel, &rule, report);
    Ok(())
}

fn scan_functions(text: &str, rel: &str, rule: &RuleId, report: &mut LaneReport) {
    let mut state = ScanState::Outside;
    let mut context = ScanContext { rel, rule, report };
    text.lines().enumerate().for_each(|(idx, raw)| {
        state = state.advance(raw, line_no_from_idx(idx), &mut context);
    });
}

struct ScanContext<'a> {
    rel: &'a str,
    rule: &'a RuleId,
    report: &'a mut LaneReport,
}

impl ScanState {
    fn advance(self, raw: &str, line_no: u32, context: &mut ScanContext<'_>) -> Self {
        match self {
            Self::Outside => outside_next(raw, line_no),
            Self::Inside { .. } => inside_next(self, raw, context),
        }
    }
}

fn outside_next(raw: &str, line_no: u32) -> ScanState {
    if is_fn_header(raw) {
        ScanState::Inside { start_line: line_no, count: 0, depth: brace_delta(raw) }
    } else {
        ScanState::Outside
    }
}

fn inside_next(state: ScanState, raw: &str, context: &mut ScanContext<'_>) -> ScanState {
    let ScanState::Inside { start_line, count, depth } = state else {
        return state;
    };
    let count = next_count(count, raw);
    let depth = depth.saturating_add(brace_delta(raw));
    if depth > 0_i32 {
        return ScanState::Inside { start_line, count, depth };
    }
    push_oversized_function(start_line, count, context.rel, context.rule, context.report);
    ScanState::Outside
}

fn next_count(count: usize, raw: &str) -> usize {
    if is_logical_line(raw) { saturating_add_usize(count, 1) } else { count }
}

fn push_oversized_function(
    start_line: u32,
    count: usize,
    rel: &str,
    rule: &RuleId,
    report: &mut LaneReport,
) {
    if count > FN_LINE_LIMIT {
        report.push(Finding::new(
            rule.clone(),
            rel,
            start_line,
            format!("function has {count} logical lines (limit {FN_LINE_LIMIT})"),
        ));
    }
}

fn is_fn_header(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") || !line.contains("fn ") || !line.contains('(') {
        return false;
    }
    line.find("fn ").and_then(|idx| line.get(..idx)).is_some_and(is_fn_boundary)
}

fn is_fn_boundary(before: &str) -> bool {
    before.chars().last().is_none_or(|prev| !(prev.is_alphanumeric() || prev == '_'))
}

fn is_logical_line(line: &str) -> bool {
    let trimmed = line.trim();
    !(trimmed.is_empty() || trimmed.starts_with("//") || trimmed == "{" || trimmed == "}")
}
