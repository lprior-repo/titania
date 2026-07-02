//! Run Cargo-native lanes inside the target project discovered from CWD.

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
        Err(RunCargoError::Usage(message)) => exit_after_stderr_line(&message, LaneExit::Usage),
        Err(RunCargoError::Target(error)) => {
            exit_after_stderr_line(&format!("target discovery failed: {error}"), LaneExit::Usage)
        }
        Err(RunCargoError::Command(error)) => {
            exit_after_stderr_line(&format!("cargo execution failed: {error}"), LaneExit::Failure)
        }
        Err(RunCargoError::CurrentDir(error)) => exit_after_stderr_line(
            &format!("cannot read current directory: {error}"),
            LaneExit::Failure,
        ),
        Err(RunCargoError::RuleId(error)) => exit_after_stderr_line(
            &format!("rule id configuration error: {error}"),
            LaneExit::Failure,
        ),
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
