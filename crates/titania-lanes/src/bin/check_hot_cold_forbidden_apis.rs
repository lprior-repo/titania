//! Walks `crates/*/src` for forbidden API surface in hot paths.
//!
//! Rust re-implementation of the bash lane `scripts/check-hot-cold-forbidden-apis.sh`. Run via
//! `cargo run --bin check-hot-cold-forbidden-apis -- [--self-test]` from
//! the repository root, or via the matching Moon task in
//! `.moon/tasks/all.yml`.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

#[path = "check_hot_cold_forbidden_apis/allow_file.rs"]
/// Allowlist parsing for hot/cold forbidden API exceptions.
pub mod allow_file;
#[path = "check_hot_cold_forbidden_apis/model.rs"]
/// Shared scanner model types and constants.
pub mod model;
#[path = "check_hot_cold_forbidden_apis/scan.rs"]
/// Hot/cold forbidden API scanner implementation.
pub mod scan;
#[path = "check_hot_cold_forbidden_apis/selftest.rs"]
/// Fixture-backed self-test for the scanner.
pub mod selftest;
#[path = "check_hot_cold_forbidden_apis/syntax.rs"]
/// Syntax helpers for stripping non-code text.
pub mod syntax;

use std::{io::Write as _, process::ExitCode};

use model::{FindingData, HOT_CRATES};
use titania_lanes::{
    Finding, LaneExit, LaneReport, RuleId, RuleIdError, current_target_project, exit,
};

const RULE_INVALID_INVOCATION: &str = "HC_INVOCATION_001";
const RULE_VIOLATION: &str = "HC_VIOLATION_001";
const RULE_FIXTURE: &str = "HC_FIXTURE_001";

struct HcRules {
    invalid_invocation: RuleId,
    violation: RuleId,
    fixture: RuleId,
}

impl HcRules {
    /// Build rule identifiers for hot/cold scanner findings.
    ///
    /// # Errors
    ///
    /// Returns the invalid rule-id error if one of the configured rule ids
    /// violates the shared rule-id format.
    fn new() -> Result<Self, RuleIdError> {
        Ok(Self {
            invalid_invocation: RuleId::new(RULE_INVALID_INVOCATION)?,
            violation: RuleId::new(RULE_VIOLATION)?,
            fixture: RuleId::new(RULE_FIXTURE)?,
        })
    }
}

fn finding_from(finding: &FindingData, rule: &RuleId) -> Finding {
    Finding::new(
        rule.clone(),
        finding.rel_path.clone(),
        finding.line_no_as_u32(),
        format!("{}: {}", finding.class_id, finding.text),
    )
}

/// Print command usage.
///
/// # Errors
///
/// Returns the stderr write error if usage cannot be emitted.
fn print_help() -> std::io::Result<()> {
    write_stderr_line(format_args!(
        "usage: check-hot-cold-forbidden-apis [--self-test]\n\
         Scans crates/<boundary>/src for forbidden API surface in hot\n\
         paths. Honors scripts/hot-cold-forbidden-apis.allow."
    ))
}

fn usage_error(error: impl std::fmt::Display, rules: &HcRules) -> ExitCode {
    let mut report = LaneReport::new();
    report.push(Finding::new(
        rules.invalid_invocation.clone(),
        ".",
        0,
        format!("InvalidInvocation: cannot resolve target project: {error}"),
    ));
    if write_stderr(&report.render()).is_err() {
        return exit(LaneExit::Failure);
    }
    exit(LaneExit::Usage)
}

fn fixture_error(error: String, rules: &HcRules) -> ExitCode {
    let mut report = LaneReport::new();
    report.push(Finding::new(rules.fixture.clone(), ".", 0, error));
    if write_stderr(&report.render()).is_err() {
        return exit(LaneExit::Failure);
    }
    exit(LaneExit::Failure)
}

/// Print classified paths and justified exceptions to stdout.
///
/// # Errors
///
/// Returns the stdout write error from the first failed write.
fn print_scan_results(classified: &[String], justified: &[FindingData]) -> std::io::Result<()> {
    if !classified.is_empty() {
        write_stdout_line(format_args!("{}", classified.join("\n")))?;
    }
    let lines: Vec<String> = justified
        .iter()
        .map(|finding| {
            format!(
                "JustifiedException|{}|{}|line={}",
                finding.class_id, finding.rel_path, finding.line_no
            )
        })
        .collect();
    if !lines.is_empty() {
        write_stdout_line(format_args!("{}", lines.join("\n")))?;
    }
    Ok(())
}

/// Print the final scanner summary line.
///
/// # Errors
///
/// Returns the stdout write error if the summary cannot be emitted.
fn print_summary(
    classified: &[String],
    violations: &[FindingData],
    justified: &[FindingData],
) -> std::io::Result<()> {
    write_stdout_line(format_args!(
        "ScanSummary|hot_crates={}|classified={}|violations={}|justified={}",
        HOT_CRATES.join(","),
        classified.len(),
        violations.len(),
        justified.len()
    ))
}

fn finish_scan(
    classified: &[String],
    violations: &[FindingData],
    justified: &[FindingData],
    rules: &HcRules,
) -> ExitCode {
    if print_scan_results(classified, justified).is_err() {
        return exit(LaneExit::Failure);
    }
    let mut report = LaneReport::new();
    report.extend_finding(violations.iter().map(|finding| finding_from(finding, &rules.violation)));
    if write_stderr(&report.render()).is_err() {
        return exit(LaneExit::Failure);
    }
    if print_summary(classified, violations, justified).is_err() {
        return exit(LaneExit::Failure);
    }
    if violations.is_empty() { exit(LaneExit::Clean) } else { exit(LaneExit::Violations) }
}

fn run_lane(rules: &HcRules) -> ExitCode {
    let target = match current_target_project() {
        Ok(target) => target,
        Err(error) => return usage_error(error, rules),
    };
    match scan::scan(target.as_std_path()) {
        Ok((classified, violations, justified)) => {
            finish_scan(&classified, &violations, &justified, rules)
        }
        Err(error) => fixture_error(error, rules),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        return exit_after_io(print_help().is_ok(), LaneExit::Usage);
    }
    if args.iter().any(|arg| arg == "--self-test") {
        return self_test_exit();
    }
    let rules = match HcRules::new() {
        Ok(rules) => rules,
        Err(error) => {
            return exit_after_io(
                write_stderr_line(format_args!(
                    "[check-hot-cold-forbidden-apis] rule id configuration error: {error}"
                ))
                .is_ok(),
                LaneExit::Failure,
            );
        }
    };
    run_lane(&rules)
}

fn exit_after_io(write_succeeded: bool, success: LaneExit) -> ExitCode {
    if !write_succeeded {
        return exit(LaneExit::Failure);
    }
    exit(success)
}

fn self_test_exit() -> ExitCode {
    let outcome = match selftest::self_test() {
        0 => LaneExit::Clean,
        _ => LaneExit::Violations,
    };
    exit(outcome)
}

/// Write one formatted line to stdout.
///
/// # Errors
///
/// Returns the underlying stdout write error.
fn write_stdout_line(args: std::fmt::Arguments<'_>) -> std::io::Result<()> {
    let mut stdout = std::io::stdout().lock();
    stdout.write_fmt(args)?;
    stdout.write_all(b"\n")
}

/// Write raw text to stderr.
///
/// # Errors
///
/// Returns the underlying stderr write error.
fn write_stderr(text: &str) -> std::io::Result<()> {
    let mut stderr = std::io::stderr().lock();
    stderr.write_all(text.as_bytes())
}

/// Write one formatted line to stderr.
///
/// # Errors
///
/// Returns the underlying stderr write error.
fn write_stderr_line(args: std::fmt::Arguments<'_>) -> std::io::Result<()> {
    let mut stderr = std::io::stderr().lock();
    stderr.write_fmt(args)?;
    stderr.write_all(b"\n")
}
