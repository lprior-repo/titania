use std::{collections::BTreeSet, path::Path};

use titania_lanes::{Finding, LaneReport, helpers::line_no_from_idx};

use super::{COLD_TOKENS, HotpathRules, TOKENS, allow::AllowEntry};

fn is_cold_path(rel: &str) -> bool {
    let normalized = rel.replace(['/', '.', '_', '-'], " ");
    COLD_TOKENS.iter().any(|token| has_cold_token(&normalized, token))
}

fn has_cold_token(normalized: &str, token: &str) -> bool {
    normalized.split_whitespace().any(|word| word == token)
}

/// Recursively scan a hot source directory for forbidden hotpath tokens.
pub fn scan_dir(
    dir: &Path,
    root: &Path,
    allow: &BTreeSet<AllowEntry>,
    rules: &HotpathRules,
    report: &mut LaneReport,
) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        scan_path(&entry.path(), root, allow, rules, report);
    }
}

fn scan_path(
    path: &Path,
    root: &Path,
    allow: &BTreeSet<AllowEntry>,
    rules: &HotpathRules,
    report: &mut LaneReport,
) {
    if path.is_dir() {
        scan_dir(path, root, allow, rules, report);
        return;
    }
    if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
        return;
    }
    let rel = rel_str(root, path);
    if is_cold_path(&rel) {
        return;
    }
    scan_file(path, &rel, allow, rules, report);
}

fn rel_str(root: &Path, p: &Path) -> String {
    p.strip_prefix(root).map_or_else(
        |_| p.to_string_lossy().into_owned(),
        |rel| rel.to_string_lossy().replace('\\', "/"),
    )
}

fn scan_file(
    file: &Path,
    rel: &str,
    allow: &BTreeSet<AllowEntry>,
    rules: &HotpathRules,
    report: &mut LaneReport,
) {
    let Ok(text) = std::fs::read_to_string(file) else {
        return;
    };
    let mut scan = HotpathLineScan { rel, allow, rules, report };
    text.lines().enumerate().for_each(|(idx, raw)| scan.scan(idx, raw));
}

struct HotpathLineScan<'a> {
    rel: &'a str,
    allow: &'a BTreeSet<AllowEntry>,
    rules: &'a HotpathRules,
    report: &'a mut LaneReport,
}

impl HotpathLineScan<'_> {
    fn scan(&mut self, idx: usize, raw: &str) {
        let line_no = line_no_from_idx(idx);
        let no_comment = strip_comment(raw);
        TOKENS
            .iter()
            .copied()
            .filter(|token| should_report_token(no_comment, self.rel, self.allow, token))
            .for_each(|token| self.push_finding(line_no, token));
    }

    fn push_finding(&mut self, line_no: u32, token: &str) {
        push_hotpath_finding(self.report, self.rules, self.rel, line_no, token);
    }
}

fn push_hotpath_finding(
    report: &mut LaneReport,
    rules: &HotpathRules,
    rel: &str,
    line_no: u32,
    token: &str,
) {
    report.push(Finding::new(
        rules.hotpath.clone(),
        rel,
        line_no,
        format!("token {token} on hot path"),
    ));
}

fn should_report_token(line: &str, rel: &str, allow: &BTreeSet<AllowEntry>, token: &str) -> bool {
    line.contains(token) && !allow.iter().any(|(path, allowed)| path == rel && allowed == token)
}

fn strip_comment(line: &str) -> &str {
    line.find("//").and_then(|idx| line.get(..idx)).map_or(line, |prefix| prefix)
}
