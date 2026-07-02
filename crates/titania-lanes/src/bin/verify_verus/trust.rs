use std::{fs, path::Path};

use titania_core::TargetProject;
use titania_lanes::{Finding, LaneReport, RuleId, RuleIdError};

use super::walk::{WalkLine, walk_rs_lines};

const EXTERNAL_RULE: &str = "VERUS_EXTERNAL_001";
pub(crate) const TRUSTED_BASE_WAIVER_FILE: &str = "trusted-base-waivers.txt";

const FORBIDDEN_RULE: &str = "FORBIDDEN_ASSUME";

pub(crate) struct TrustRules {
    pub(crate) forbidden: RuleId,
    pub(crate) external: RuleId,
}

impl TrustRules {
    /// Build rule identifiers for trust-boundary findings.
    ///
    /// # Errors
    ///
    /// Returns the invalid rule-id error if one of the configured rule ids
    /// violates the shared rule-id format.
    pub(crate) fn new() -> Result<Self, RuleIdError> {
        Ok(Self { forbidden: RuleId::new(FORBIDDEN_RULE)?, external: RuleId::new(EXTERNAL_RULE)? })
    }
}

#[must_use]
pub(crate) fn trusted_base_waiver_exists(evidence_dir: &Path) -> bool {
    fs::metadata(evidence_dir.join(TRUSTED_BASE_WAIVER_FILE))
        .is_ok_and(|meta| meta.is_file() && meta.len() != 0)
}

#[must_use]
pub(crate) fn scan_forbidden_trust(
    target: &TargetProject,
    rule: &RuleId,
    report: &mut LaneReport,
) -> Vec<String> {
    let hits: Vec<WalkLine> = trust_scan_roots(target)
        .iter()
        .flat_map(|dir| walk_rs_lines(dir, target.as_std_path()))
        .filter(|wl| is_forbidden_trust_line(&wl.text))
        .collect();
    report.extend_finding(hits.iter().map(|wl| {
        Finding::new(
            rule.clone(),
            wl.path.clone(),
            wl.line_no,
            "forbidden `assume(` or `axiom` outside comments",
        )
    }));
    hits.iter().map(|wl| format!("{}:{}: {}", wl.path, wl.line_no, wl.text)).collect()
}

#[must_use]
pub(crate) fn scan_external_markers(target: &TargetProject) -> Vec<String> {
    trust_scan_roots(target)
        .iter()
        .flat_map(|dir| walk_rs_lines(dir, target.as_std_path()))
        .filter(|wl| is_external_marker_line(&wl.text))
        .map(|wl| format!("{}:{}: {}", wl.path, wl.line_no, wl.text))
        .collect()
}

pub(crate) fn report_unwaived_external_markers(
    report: &mut LaneReport,
    lines: &[String],
    rule: &RuleId,
) {
    report.extend_finding(lines.iter().map(|line| {
        let (path, line_no) = parse_finding_location(line);
        Finding::new(
            rule.clone(),
            path,
            line_no,
            "Verus external marker requires explicit trusted-base waiver artifact",
        )
    }));
}

fn trust_scan_roots(target: &TargetProject) -> [std::path::PathBuf; 2] {
    [target.as_std_path().join("verification/verus"), target.as_std_path().join("contracts/verus")]
}

fn is_forbidden_trust_line(line: &str) -> bool {
    !is_rust_comment(line)
        && (line.contains("assume(") || line.contains("axiom ") || line.contains("axiom\t"))
}

fn is_external_marker_line(line: &str) -> bool {
    !is_rust_comment(line)
        && (line.contains("#[verifier::external_body]")
            || line.contains("#[verifier::external]")
            || line.contains("assume_specification["))
}

fn is_rust_comment(line: &str) -> bool {
    line.trim_start().starts_with("//")
}

fn parse_finding_location(line: &str) -> (String, u32) {
    let mut parts = line.splitn(3, ':');
    let path = parts.next().map_or_else(String::new, str::to_owned);
    let line_no = parts.next().and_then(|s| s.parse::<u32>().ok()).map_or(0, |n| n);
    (path, line_no)
}
