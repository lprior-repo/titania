//! Compares fuzz oracle function bodies vs production error enum definitions.
//!
//! Rust re-implementation of the bash lane in
//! `velvet-ballistics/scripts/check-error-exhaustiveness.sh`. Run via
//! `cargo run --bin check-error-exhaustiveness --` from the repository root or via the
//! matching Moon task in `.moon/tasks/all.yml`.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

#[path = "check_error_exhaustiveness/check.rs"]
/// Exhaustiveness check runner.
pub mod check;
#[path = "check_error_exhaustiveness/model.rs"]
/// Static check model for error exhaustiveness.
pub mod model;
#[path = "check_error_exhaustiveness/parser.rs"]
/// Lightweight Rust source parsers used by the lane.
pub mod parser;

use std::{
    io::{self, Write},
    process::ExitCode,
};

use titania_lanes::{LaneExit, LaneReport, RuleId, current_target_project, exit};

const EXHAUST_RULE: &str = "EXHAUST_001";

fn main() -> ExitCode {
    let target = match current_target_project() {
        Ok(target) => target,
        Err(error) => {
            return exit_after_stderr_line(
                &format!("[check-error-exhaustiveness] target discovery failed: {error}"),
                LaneExit::Failure,
            );
        }
    };
    let rule = match RuleId::new(EXHAUST_RULE) {
        Ok(rule) => rule,
        Err(error) => {
            return exit_after_stderr_line(
                &format!("[check-error-exhaustiveness] rule id configuration error: {error}"),
                LaneExit::Failure,
            );
        }
    };
    let mut report = LaneReport::new();
    if let Err(error) = check::run(&target, &rule, &mut report) {
        return exit_after_stderr_line(
            &format!("[check-error-exhaustiveness] output failed: {error}"),
            LaneExit::Failure,
        );
    }
    print_and_exit(&report)
}

fn print_and_exit(report: &LaneReport) -> ExitCode {
    let rendered = report.render();
    if !rendered.is_empty() && write_stderr(&rendered).is_err() {
        return exit(LaneExit::Failure);
    }
    if report.is_clean() { exit(LaneExit::Clean) } else { exit(LaneExit::Violations) }
}

/// Writes raw text to stderr.
///
/// # Errors
///
/// Returns the stderr write error when output fails.
fn write_stderr(text: &str) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    stderr.write_all(text.as_bytes())
}

/// Writes a line to stderr.
///
/// # Errors
///
/// Returns the stderr write error when text or newline output fails.
fn write_stderr_line(text: &str) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    stderr.write_all(text.as_bytes())?;
    stderr.write_all(b"\n")
}

fn exit_after_stderr_line(text: &str, code: LaneExit) -> ExitCode {
    match write_stderr_line(text) {
        Ok(()) => exit(code),
        Err(_) => exit(LaneExit::Failure),
    }
}
