//! CLI dispatch shell for `titania-check`.
//!
//! The shell owns argument parsing, exit-code mapping, and typed dispatch.
//! It does not fake doctor output or lane execution.

pub mod aggregate;
pub mod args;
pub mod doctor;
pub mod explain;
pub mod moon;

use std::{env, io, io::Write, process::ExitCode};

use args::{Cli, Command};
use titania_core::{GateScope, Lane, RuleId};
use titania_lanes::LaneExit;

const EXIT_PASS: u8 = 0;
const EXIT_REJECT: u8 = 1;
const EXIT_POLICY_ERROR: u8 = 2;
const EXIT_INPUT_ERROR: u8 = 3;
const EXIT_INTERNAL_ERROR: u8 = 4;

fn main() -> ExitCode {
    exit(&run(env::args_os().skip(1)))
}

fn run<I>(args: I) -> CliDisposition
where
    I: IntoIterator<Item = std::ffi::OsString>,
{
    match Cli::parse_from_os(args) {
        Ok(cli) => dispatch(cli),
        Err(error) if error.is_help() => CliDisposition::report(error.diagnostic(), EXIT_PASS),
        Err(error) => CliDisposition::input_error(error.diagnostic()),
    }
}

fn dispatch(cli: Cli) -> CliDisposition {
    match cli.command {
        Command::Check(options) => check_with_moon(&options),
        Command::Aggregate(options) => {
            aggregate_with_opts(options.scope, options.emit(), options.out())
        }
        Command::RunLane { lane } => run_lane(lane),
        Command::Doctor(options) => doctor_scope(options.scope, options.emit()),
        Command::Explain { rule_id } => explain_rule(&rule_id),
    }
}

/// Drive `Command::Check` through Moon (spec §12, §13).
///
/// Builds the moon task list for the scope, spawns `moon run <tasks...>` with
/// stderr inherited so the user sees lane progress, then runs the in-process
/// aggregate (`aggregate::report_json`) with the user's `--emit`/`--out` flags.
/// Individual lane outcomes are read from the artifacts Moon's tasks wrote. A
/// missing `moon` binary surfaces as `InputError(3)`; an unreadable artifact or
/// spawn failure (other than `NotFound`) surfaces as `InternalError(>=4)`.
fn check_with_moon(options: &args::CheckOptions) -> CliDisposition {
    let scope = options.scope;
    let emit = options.emit();
    let out = options.out();
    let moon_bin = moon::binary_path();
    let tasks = moon::tasks_for_scope(scope);
    match moon::spawn(&moon_bin, &tasks) {
        Ok(()) => aggregate_with_opts(scope, emit, out),
        Err(moon::MoonSpawnError::NotFound) => {
            CliDisposition::input_error(format!("InputError: {}", moon::MISSING_INSTALL_HINT))
        }
        Err(moon::MoonSpawnError::Failed(error)) => {
            CliDisposition::internal_error(format!("InternalError: moon spawn failed: {error}"))
        }
    }
}

fn aggregate_with_opts(
    scope: GateScope,
    emit: args::EmitFormat,
    out: Option<&std::path::PathBuf>,
) -> CliDisposition {
    env::current_dir().map_or_else(
        |error| current_dir_error(&error),
        |target_root| aggregate_from_root(&target_root, scope, emit, out),
    )
}

fn current_dir_error(error: &io::Error) -> CliDisposition {
    CliDisposition::internal_error(format!("InternalError: current directory unavailable: {error}"))
}
fn aggregate_from_root(
    target_root: &std::path::Path,
    scope: GateScope,
    emit: args::EmitFormat,
    out: Option<&std::path::PathBuf>,
) -> CliDisposition {
    match aggregate::report_json(target_root, scope) {
        Ok(report) => report_with_opts(&report, emit, out),
        Err(error) => CliDisposition::input_error(error.diagnostic()),
    }
}
/// Render the report with emit-format selection and optional file output.
fn report_with_opts(
    report: &aggregate::ReportJson,
    emit: args::EmitFormat,
    out: Option<&std::path::PathBuf>,
) -> CliDisposition {
    let stdout = match emit {
        args::EmitFormat::Human => report.render_human(),
        args::EmitFormat::Json => report.json().to_owned(),
    };

    if let Some(path) = out {
        // Write report to file, suppress stdout.
        write_report(path, &stdout, report.status())
    } else {
        CliDisposition::report(stdout, report_code(report.status()))
    }
}

/// Write a report to file and return a silent disposition, or an internal
/// error disposition when the file write fails.
fn write_report(
    path: &std::path::PathBuf,
    content: &str,
    status: aggregate::ReportStatus,
) -> CliDisposition {
    if let Err(e) = std::fs::write(path, content) {
        return CliDisposition::internal_error(format!(
            "InternalError: cannot write report to {}: {e}",
            path.display()
        ));
    }
    CliDisposition::silent(report_code(status))
}

const fn report_code(status: aggregate::ReportStatus) -> u8 {
    match status {
        aggregate::ReportStatus::Pass => EXIT_PASS,
        aggregate::ReportStatus::Reject => EXIT_REJECT,
        aggregate::ReportStatus::PolicyError => EXIT_POLICY_ERROR,
        aggregate::ReportStatus::InputError => EXIT_INPUT_ERROR,
    }
}

fn run_lane(lane: Lane) -> CliDisposition {
    let execution = titania_lanes::run_lane::execute_lane(lane);
    CliDisposition::lane_execution(map_lane_exit(execution.exit()), execution.stderr())
}

/// Map lane exit disposition to a stable process exit code.
const fn map_lane_exit(exit: LaneExit) -> u8 {
    match exit {
        LaneExit::Clean | LaneExit::NotApplicable => EXIT_PASS,
        LaneExit::Violations => EXIT_REJECT,
        LaneExit::Usage => EXIT_INPUT_ERROR,
        LaneExit::Failure => EXIT_INTERNAL_ERROR,
    }
}

fn doctor_scope(scope: GateScope, emit: args::EmitFormat) -> CliDisposition {
    match doctor::render(scope, emit) {
        Ok(disposition) => disposition,
        Err(error) => CliDisposition::internal_error(format!("InternalError: {error}")),
    }
}

fn explain_rule(rule_id: &RuleId) -> CliDisposition {
    explain::render(rule_id).map_or_else(
        |error| CliDisposition::input_error(format!("InputError: {error}")),
        |output| CliDisposition::report(output, EXIT_PASS),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliDisposition {
    code: u8,
    stdout: Option<String>,
    diagnostic: Option<String>,
}

impl CliDisposition {
    const fn input_error(message: String) -> Self {
        Self { code: EXIT_INPUT_ERROR, stdout: None, diagnostic: Some(message) }
    }

    const fn internal_error(message: String) -> Self {
        Self { code: EXIT_INTERNAL_ERROR, stdout: None, diagnostic: Some(message) }
    }

    const fn report(stdout: String, code: u8) -> Self {
        Self { code, stdout: Some(stdout), diagnostic: None }
    }
    const fn silent(code: u8) -> Self {
        Self { code, stdout: None, diagnostic: None }
    }

    fn lane_execution(code: u8, stderr: &str) -> Self {
        let diagnostic = (!stderr.is_empty()).then(|| stderr.trim_end_matches('\n').to_owned());
        Self { code, stdout: None, diagnostic }
    }

    const fn code(&self) -> u8 {
        self.code
    }

    fn stdout(&self) -> Option<&str> {
        self.stdout.as_deref()
    }

    fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }
}

fn exit(disposition: &CliDisposition) -> ExitCode {
    write_disposition(disposition).map_or_else(
        |_| ExitCode::from(EXIT_INTERNAL_ERROR),
        |()| ExitCode::from(disposition.code()),
    )
}

/// Write the diagnostic for a CLI disposition when one exists.
///
/// # Errors
///
/// Returns the underlying stderr I/O error when the diagnostic cannot be
/// written completely.
fn write_disposition(disposition: &CliDisposition) -> io::Result<()> {
    if let Some(stdout) = disposition.stdout() {
        write_stdout_line(stdout)?;
    }
    if let Some(diagnostic) = disposition.diagnostic() {
        return write_stderr_line(diagnostic);
    }
    Ok(())
}

/// Write a single stdout line.
///
/// # Errors
///
/// Returns the underlying stdout I/O error when the line or newline cannot be
/// written completely.
fn write_stdout_line(text: &str) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(text.as_bytes())?;
    stdout.write_all(b"\n")
}

/// Write a single stderr line.
///
/// # Errors
///
/// Returns the underlying stderr I/O error when the line or newline cannot be
/// written completely.
fn write_stderr_line(text: &str) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    stderr.write_all(text.as_bytes())?;
    stderr.write_all(b"\n")
}
