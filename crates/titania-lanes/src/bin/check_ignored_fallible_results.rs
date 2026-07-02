//! DISCARD-001..006 scanner for fallible-call ignores across crates/*/src + xtask/src.
//!
//! Rust re-implementation of the bash lane `scripts/check-ignored-fallible-results.sh`. Run via
//! `cargo run --bin check_ignored_fallible_results --` from the repository root or via the
//! matching Moon task in `.moon/tasks/all.yml`.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

/// Allowlist loading and validation for ignored fallible-result findings.
#[path = "check_ignored_fallible_results/allow.rs"]
pub mod allow;
/// Source scanner for ignored fallible-result patterns.
#[path = "check_ignored_fallible_results/scan.rs"]
pub mod scan;
/// Source-line parsing that strips comments and signatures.
#[path = "check_ignored_fallible_results/source.rs"]
pub mod source;

use std::{io::Write as _, path::Path, process::ExitCode};

use titania_lanes::{LaneExit, LaneReport, RuleId, RuleIdError, current_target_project, exit};

fn main() -> ExitCode {
    let target = match current_target_project() {
        Ok(target) => target,
        Err(error) => {
            return exit_after_stderr_line(
                format_args!(
                    "[check-ignored-fallible-results] cannot resolve target project: {error}"
                ),
                LaneExit::Usage,
            );
        }
    };
    let mut report = LaneReport::new();
    if let Err(error) = run(target.as_std_path(), &mut report) {
        return exit_after_stderr_line(
            format_args!("[check-ignored-fallible-results] rule id configuration error: {error}"),
            LaneExit::Failure,
        );
    }
    print_and_exit(&report)
}

/// Run the ignored fallible-result scan.
///
/// # Errors
///
/// Returns a rule-id construction error if one of the configured discard or
/// allowlist rules is invalid.
fn run(root: &Path, report: &mut LaneReport) -> Result<(), RuleIdError> {
    let allow_rule = RuleId::new(allow::ALLOW_RULE)?;
    let discard_rules = scan::DiscardRules::new()?;
    let allow = allow::load_allow(root, &allow_rule, report);
    scan::scan(root, &allow, &discard_rules, report);
    Ok(())
}

fn exit_after_stderr_line(
    args: std::fmt::Arguments<'_>,
    success: LaneExit,
) -> std::process::ExitCode {
    if write_stderr_line(args).is_err() {
        return exit(LaneExit::Failure);
    }
    exit(success)
}

fn print_and_exit(report: &LaneReport) -> ExitCode {
    let rendered = report.render();
    if !rendered.is_empty() && write_stderr(&rendered).is_err() {
        return exit(LaneExit::Failure);
    }
    if report.is_clean() { exit(LaneExit::Clean) } else { exit(LaneExit::Violations) }
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
