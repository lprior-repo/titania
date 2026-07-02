//! Identifier-diff verifier for `verification/verus/production_inner` mirrors + BINDING LEDGERs.
//!
//! Rust re-implementation of the bash lane in
//! `velvet-ballistics/scripts/check-production-inner-drift.sh`. Run via
//! `cargo run --bin check_production_inner_drift --` from the repository root or via the
//! matching Moon task in `.moon/tasks/all.yml`.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

#[path = "check_production_inner_drift/claims.rs"]
/// Claim parsing for production mirror headers.
pub mod claims;
#[path = "check_production_inner_drift/externs.rs"]
/// EXTERN ledger scanning for production binding drift.
pub mod externs;
#[path = "check_production_inner_drift/identifiers.rs"]
/// Identifier extraction helpers for mirror comparison.
pub mod identifiers;
#[path = "check_production_inner_drift/mirror.rs"]
/// Mirror source comparison pass.
pub mod mirror;

use titania_core::TargetProject;
use titania_lanes::{LaneExit, LaneReport, RuleId, current_target_project, exit};

use std::io::Write as _;

const MIRROR_DIR: &str = "verification/verus/production_inner";
const DRIFT_RULE: &str = "DRIFT_001";

fn main() -> std::process::ExitCode {
    let target = match current_target_project() {
        Ok(target) => target,
        Err(error) => {
            return fail_after_stderr_line(format_args!(
                "[check-production-inner-drift] target discovery failed: {error}"
            ));
        }
    };
    let rule = match RuleId::new(DRIFT_RULE) {
        Ok(rule) => rule,
        Err(error) => {
            return fail_after_stderr_line(format_args!(
                "[check-production-inner-drift] rule id configuration error: {error}"
            ));
        }
    };
    let mut report = LaneReport::new();
    run(&target, &rule, &mut report);
    print_and_exit(&report)
}

fn run(target: &TargetProject, rule: &RuleId, report: &mut LaneReport) {
    let root = target.as_std_path();
    mirror::per_mirror_pass(root, MIRROR_DIR, rule, report);
    externs::per_extern_pass(root, rule, report);
}

fn print_and_exit(report: &LaneReport) -> std::process::ExitCode {
    let rendered = report.render();
    if !rendered.is_empty() && write_stderr(&rendered).is_err() {
        return exit(LaneExit::Failure);
    }
    if report.is_clean() { exit(LaneExit::Clean) } else { exit(LaneExit::Violations) }
}

fn fail_after_stderr_line(args: std::fmt::Arguments<'_>) -> std::process::ExitCode {
    match write_stderr_line(args) {
        Ok(()) | Err(_) => exit(LaneExit::Failure),
    }
}

/// Writes raw text to standard error.
///
/// # Errors
///
/// Returns any I/O error reported while writing to the locked standard-error stream.
fn write_stderr(text: &str) -> std::io::Result<()> {
    let mut stderr = std::io::stderr().lock();
    stderr.write_all(text.as_bytes())
}

/// Writes formatted text and a trailing newline to standard error.
///
/// # Errors
///
/// Returns any formatting or I/O error reported while writing to standard error.
fn write_stderr_line(args: std::fmt::Arguments<'_>) -> std::io::Result<()> {
    let mut stderr = std::io::stderr().lock();
    stderr.write_fmt(args)?;
    stderr.write_all(b"\n")
}
