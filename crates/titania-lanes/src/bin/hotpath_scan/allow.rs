use std::{collections::BTreeSet, path::Path};

use titania_lanes::{Finding, LaneReport, helpers::line_no_from_idx};

use super::{ALLOW_FILE, HotpathRules, TOKENS};

/// A normalized `(path, token)` allowlist entry.
pub type AllowEntry = (String, String);

struct AllowRow<'a> {
    path: &'a str,
    token: &'a str,
    owner: &'a str,
    reviewer: &'a str,
    test: &'a str,
    reason: &'a str,
}

/// Load and validate all hotpath allowlist rows.
#[must_use]
pub fn load_allow(
    root: &Path,
    rules: &HotpathRules,
    report: &mut LaneReport,
) -> BTreeSet<AllowEntry> {
    let path = root.join(ALLOW_FILE);
    let Some(text) = read_allow_file(&path) else {
        return BTreeSet::new();
    };
    text.lines()
        .enumerate()
        .filter_map(|(idx, raw)| allow_entry_from_line(idx, raw, rules, report))
        .collect()
}

fn read_allow_file(path: &Path) -> Option<String> {
    if !path.is_file() {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

fn allow_entry_from_line(
    idx: usize,
    raw: &str,
    rules: &HotpathRules,
    report: &mut LaneReport,
) -> Option<AllowEntry> {
    let line_no = line_no_from_idx(idx);
    let line = raw.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let Some(row) = parse_allow_row(line) else {
        push_allow_finding(report, rules, line_no, "malformed allow row");
        return None;
    };
    if let Some(message) = validate_allow_row(&row) {
        push_allow_finding(report, rules, line_no, message);
        return None;
    }
    Some((row.path.to_string(), row.token.to_string()))
}

fn parse_allow_row(line: &str) -> Option<AllowRow<'_>> {
    let mut parts = line.split('|');
    Some(AllowRow {
        path: parts.next()?,
        token: parts.next()?,
        owner: parts.next()?,
        reviewer: parts.next()?,
        test: parts.next()?,
        reason: parts.next()?,
    })
}

fn validate_allow_row(row: &AllowRow<'_>) -> Option<String> {
    if is_overbroad_allow_path(row.path) {
        return Some("overbroad path".to_string());
    }
    if !TOKENS.contains(&row.token) {
        return Some(format!("unknown token {}", row.token));
    }
    if has_required_allow_metadata(row) { None } else { Some("missing fields".to_string()) }
}

fn is_overbroad_allow_path(path: &str) -> bool {
    path.contains('*')
        || !path.starts_with("crates/")
        || !std::path::Path::new(path).extension().is_some_and(|ext| ext.eq_ignore_ascii_case("rs"))
}

fn has_required_allow_metadata(row: &AllowRow<'_>) -> bool {
    row.owner.starts_with("owner=")
        && row.reviewer.starts_with("reviewed_by=")
        && row.test.starts_with("test=")
        && row.reason.starts_with("reason=")
}

fn push_allow_finding(
    report: &mut LaneReport,
    rules: &HotpathRules,
    line_no: u32,
    message: impl Into<String>,
) {
    report.push(Finding::new(
        rules.allow.clone(),
        format!("{ALLOW_FILE}:{line_no}"),
        line_no,
        message,
    ));
}
