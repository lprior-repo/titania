use std::{collections::HashSet, path::Path};

use titania_lanes::{Finding, LaneReport, RuleId, RuleIdError, helpers::line_no_from_idx};

use crate::{
    LEDGER_PATH,
    paths::{is_excluded_source_path, tracked_set},
};

const SRC_LEN_LEDGER_RULE: &str = "SRC_LEN_LEDGER";

/// Load and validate the source-length exceptions ledger.
///
/// # Errors
///
/// Returns a rule-id construction error when the ledger rule id is invalid.
pub(super) fn load_ledger(
    root: &Path,
    report: &mut LaneReport,
) -> Result<Vec<String>, RuleIdError> {
    let path = root.join(LEDGER_PATH);
    if !path.is_file() {
        let _emitted = crate::write_stderr_line(format_args!(
            "Info: source-length exceptions ledger absent; using empty exceptions"
        ))
        .is_ok();
        return Ok(Vec::new());
    }
    let Ok(text) = std::fs::read_to_string(&path) else {
        let _emitted = crate::write_stderr_line(format_args!(
            "Info: source-length exceptions ledger unreadable; using empty exceptions"
        ))
        .is_ok();
        return Ok(Vec::new());
    };
    let tracked = tracked_set(root);
    let rule = RuleId::new(SRC_LEN_LEDGER_RULE)?;
    Ok(parse_ledger(&text, &tracked, &rule, report))
}

fn parse_ledger(
    text: &str,
    tracked: &HashSet<String>,
    rule: &RuleId,
    report: &mut LaneReport,
) -> Vec<String> {
    let mut entries: Vec<String> = Vec::new();
    let mut context = LedgerParseContext { tracked, rule, report };
    text.lines().enumerate().for_each(|(idx, raw)| {
        push_ledger_line(raw, line_no_from_idx(idx), &mut entries, &mut context);
    });
    entries
}

fn push_ledger_line(
    raw: &str,
    line_no: u32,
    entries: &mut Vec<String>,
    context: &mut LedgerParseContext<'_>,
) {
    if let Some(file) = parse_ledger_line(raw, line_no, entries, context) {
        entries.push(file);
    }
}

struct LedgerParseContext<'a> {
    tracked: &'a HashSet<String>,
    rule: &'a RuleId,
    report: &'a mut LaneReport,
}

fn parse_ledger_line(
    raw: &str,
    line_no: u32,
    entries: &[String],
    context: &mut LedgerParseContext<'_>,
) -> Option<String> {
    let line = raw.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let parts: Vec<&str> = line.split('|').collect();
    let file = ledger_file(&parts, line_no, context.rule, context.report)?;
    validate_ledger_file(file, line_no, entries, context)?;
    Some(file.to_string())
}

fn ledger_file<'a>(
    parts: &'a [&'a str],
    line_no: u32,
    rule: &RuleId,
    report: &mut LaneReport,
) -> Option<&'a str> {
    if parts.len() == 5 {
        return parts.first().copied();
    }
    report.push(Finding::new(
        rule.clone(),
        format!("{LEDGER_PATH}:{line_no}"),
        line_no,
        "malformed row; expected <file>|<owner>|<split_bead>|<removal_plan>|<reason>",
    ));
    None
}

fn validate_ledger_file(
    file: &str,
    line_no: u32,
    entries: &[String],
    context: &mut LedgerParseContext<'_>,
) -> Option<()> {
    if let Some(message) = ledger_file_error(file, context.tracked, entries) {
        context.report.push(Finding::new(
            context.rule.clone(),
            ledger_ref(line_no),
            line_no,
            message,
        ));
        return None;
    }
    Some(())
}

fn ledger_file_error(file: &str, tracked: &HashSet<String>, entries: &[String]) -> Option<String> {
    if invalid_relative_path(file) {
        return Some("invalid path; use a normalized repository-relative path".to_string());
    }
    rust_file_error(file).or_else(|| tracked_error(file, tracked, entries))
}

fn invalid_relative_path(file: &str) -> bool {
    file.starts_with('/') || file.starts_with("../") || file.contains("/../")
}

fn rust_file_error(file: &str) -> Option<String> {
    if !std::path::Path::new(file).extension().is_some_and(|ext| ext.eq_ignore_ascii_case("rs")) {
        return Some(format!("path is not a Rust source file: {file}"));
    }
    if is_excluded_source_path(file) {
        return Some(format!("path is excluded from first-party source-length checks: {file}"));
    }
    None
}

fn tracked_error(file: &str, tracked: &HashSet<String>, entries: &[String]) -> Option<String> {
    if !tracked.contains(file) {
        return Some(format!("path is not a tracked first-party Rust source file: {file}"));
    }
    if entries.iter().any(|known| known == file) {
        return Some(format!("duplicate exception for {file}"));
    }
    None
}

fn ledger_ref(line_no: u32) -> String {
    format!("{LEDGER_PATH}:{line_no}")
}
