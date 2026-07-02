use std::{fs, path::Path};

use titania_core::TargetProject;
use titania_lanes::{Finding, LaneReport};

use super::walk::walk_rs_lines;

pub const EXTERNAL_RULE: &str = "VERUS-EXTERNAL-001";
pub const TRUSTED_BASE_WAIVER_FILE: &str = "trusted-base-waivers.txt";

const FORBIDDEN_RULE: &str = "FORBIDDEN-ASSUME";

#[must_use]
pub fn trusted_base_waiver_exists(evidence_dir: &Path) -> bool {
    fs::metadata(evidence_dir.join(TRUSTED_BASE_WAIVER_FILE))
        .is_ok_and(|meta| meta.is_file() && meta.len() != 0)
}

#[must_use]
pub fn scan_forbidden_trust(target: &TargetProject, report: &mut LaneReport) -> Vec<String> {
    let mut findings = Vec::new();
    for dir in &trust_scan_roots(target) {
        walk_rs_lines(dir, target.as_std_path(), |line, path, line_no| {
            check_forbidden_line(line, path, line_no, &mut findings, report);
        });
    }
    findings
}

fn check_forbidden_line(
    line: &str,
    path: &str,
    line_no: u32,
    findings: &mut Vec<String>,
    report: &mut LaneReport,
) {
    if is_forbidden_trust_line(line) {
        findings.push(format!("{path}:{line_no}: {line}"));
        report.push(Finding::new(
            FORBIDDEN_RULE,
            path.to_owned(),
            line_no,
            "forbidden `assume(` or `axiom` outside comments",
        ));
    }
}

#[must_use]
pub fn scan_external_markers(target: &TargetProject) -> Vec<String> {
    let mut findings = Vec::new();
    for dir in &trust_scan_roots(target) {
        walk_rs_lines(dir, target.as_std_path(), |line, path, line_no| {
            check_external_line(line, path, line_no, &mut findings);
        });
    }
    findings
}

fn check_external_line(
    line: &str,
    path: &str,
    line_no: u32,
    findings: &mut Vec<String>,
) {
    if is_external_marker_line(line) {
        findings.push(format!("{path}:{line_no}: {line}"));
    }
}

pub fn report_unwaived_external_markers(report: &mut LaneReport, lines: &[String]) {
    for line in lines {
        let (path, line_no) = parse_finding_location(line);
        report.push(Finding::new(
            EXTERNAL_RULE,
            path,
            line_no,
            "Verus external marker requires explicit trusted-base waiver artifact",
        ));
    }
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
