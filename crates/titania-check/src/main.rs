//! CLI dispatch shell for `titania-check`.
//!
//! The shell owns argument parsing, exit-code mapping, and explicit typed
//! blockers for downstream work. It does not fake lane execution, aggregate
//! reports, doctor output, or rule explanations.

pub mod aggregate;
pub mod args;

use std::{env, io, io::Write, process::ExitCode};

use args::{Cli, Command};
use titania_core::{GateScope, Lane, RuleId};

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
        Err(error) => CliDisposition::input_error(error.diagnostic()),
    }
}

fn dispatch(cli: Cli) -> CliDisposition {
    match cli.command {
        Command::Check(options) => aggregate_scope(options.scope),
        Command::Aggregate(options) => aggregate_scope(options.scope),
        Command::RunLane { lane } => missing_run_lane(lane),
        Command::Doctor(options) => missing_doctor(options.scope),
        Command::Explain { rule_id } => missing_explain(&rule_id),
    }
}

fn aggregate_scope(scope: GateScope) -> CliDisposition {
    env::current_dir().map_or_else(
        |error| current_dir_error(&error),
        |target_root| aggregate_from_root(&target_root, scope),
    )
}

fn current_dir_error(error: &io::Error) -> CliDisposition {
    CliDisposition::internal_error(format!("InternalError: current directory unavailable: {error}"))
}

fn aggregate_from_root(target_root: &std::path::Path, scope: GateScope) -> CliDisposition {
    match aggregate::report_json(target_root, scope) {
        Ok(report) => report_disposition(&report),
        Err(error) => CliDisposition::input_error(error.diagnostic()),
    }
}

fn report_disposition(report: &aggregate::ReportJson) -> CliDisposition {
    CliDisposition::report(report.json().to_owned(), report_code(report.status()))
}

const fn report_code(status: aggregate::ReportStatus) -> u8 {
    match status {
        aggregate::ReportStatus::Pass => EXIT_PASS,
        aggregate::ReportStatus::Reject => EXIT_REJECT,
        aggregate::ReportStatus::PolicyError => EXIT_POLICY_ERROR,
        aggregate::ReportStatus::InputError => EXIT_INPUT_ERROR,
    }
}

fn missing_run_lane(lane: Lane) -> CliDisposition {
    CliDisposition::input_error(format!(
        "InputError: MissingImplementation command=run-lane lane '{}' bead=tn-uia; lane execution is not yet implemented",
        lane_cli_name(lane)
    ))
}

fn missing_doctor(scope: GateScope) -> CliDisposition {
    CliDisposition::input_error(format!(
        "InputError: MissingImplementation command=doctor scope '{}' bead=tn-4rq.2; doctor output is not yet implemented",
        scope_name(scope)
    ))
}

fn missing_explain(rule_id: &RuleId) -> CliDisposition {
    CliDisposition::input_error(format!(
        "InputError: MissingImplementation command=explain rule '{}' bead=tn-ja8.1; rule explain output is not yet implemented",
        rule_id.as_str()
    ))
}

const fn scope_name(scope: GateScope) -> &'static str {
    match scope {
        GateScope::Edit => "edit",
        GateScope::Prepush => "prepush",
        GateScope::Release => "release",
        _ => "unknown",
    }
}

const fn lane_cli_name(lane: Lane) -> &'static str {
    match lane {
        Lane::Fmt => "fmt",
        Lane::Compile => "compile",
        Lane::Clippy => "clippy",
        Lane::AstGrep => "ast-grep",
        Lane::Dylint => "dylint",
        Lane::PanicScan => "panic-scan",
        Lane::PolicyScan => "policy-scan",
        Lane::Test => "test",
        Lane::Deny => "deny",
        Lane::Build => "build",
    }
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
