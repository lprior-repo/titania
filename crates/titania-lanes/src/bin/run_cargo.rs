//! Run Cargo-native lanes inside the target project discovered from CWD.

#[path = "run_cargo/artifacts.rs"]
/// Typed v1 lane artifact conversion for Cargo lanes.
pub mod artifacts;

#[path = "run_cargo/findings.rs"]
/// Finding extraction from Cargo command output.
pub mod findings;
#[path = "run_cargo/lane.rs"]
/// Cargo lane selector and rule mapping.
pub mod lane;
#[path = "run_cargo/runner.rs"]
/// Run-cargo orchestration and Cargo subprocess execution.
pub mod runner;

use std::{env, io, io::Write, process::ExitCode};

use runner::{RunCargoError, run_checked};
use titania_lanes::{LaneExit, LaneReport, exit};

fn main() -> ExitCode {
    exit(run(env::args().collect()))
}

fn run(args: Vec<String>) -> LaneExit {
    match run_checked(args) {
        Ok(report) => emit_report(&report),
        Err(error) => emit_error(error),
    }
}

fn emit_error(error: RunCargoError) -> LaneExit {
    let code = error_code(&error);
    exit_after_stderr_line(&error_message(error), code)
}

const fn error_code(error: &RunCargoError) -> LaneExit {
    match error {
        RunCargoError::Usage(_) | RunCargoError::Target(_) => LaneExit::Usage,
        _other => LaneExit::Failure,
    }
}

fn error_message(error: RunCargoError) -> String {
    match error {
        RunCargoError::Usage(message) => message,
        RunCargoError::Target(error) => format!("target discovery failed: {error}"),
        RunCargoError::Command(error) => format!("cargo execution failed: {error}"),
        RunCargoError::CurrentDir(error) => format!("cannot read current directory: {error}"),
        RunCargoError::RuleId(error) => format!("rule id configuration error: {error}"),
        RunCargoError::Artifact(error) => format!("artifact write failed: {error}"),
        RunCargoError::Outcome(error) => format!("lane outcome construction failed: {error}"),
        RunCargoError::ToolVersion(error) => format!("tool version capture failed: {error}"),
    }
}

fn emit_report(report: &LaneReport) -> LaneExit {
    write_stderr(&report.render()).map_or(LaneExit::Failure, |()| report_status(report))
}

fn report_status(report: &LaneReport) -> LaneExit {
    if report.is_clean() { LaneExit::Clean } else { LaneExit::Violations }
}

/// Writes `text` to stderr without appending a line terminator.
///
/// # Errors
///
/// Returns the underlying I/O error when stderr cannot be locked or written.
fn write_stderr(text: &str) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    stderr.write_all(text.as_bytes())
}

/// Writes `text` followed by a newline to stderr.
///
/// # Errors
///
/// Returns the underlying I/O error when stderr cannot be locked or written.
fn write_stderr_line(text: &str) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    stderr.write_all(text.as_bytes())?;
    stderr.write_all(b"\n")
}

fn exit_after_stderr_line(text: &str, code: LaneExit) -> LaneExit {
    match write_stderr_line(text) {
        Ok(()) => code,
        Err(_) => LaneExit::Failure,
    }
}

fn usage_message() -> String {
    String::from("usage: run-cargo <fmt|compile|clippy|test|build>")
}
