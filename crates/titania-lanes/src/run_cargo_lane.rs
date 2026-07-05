//! Library wrapper for Cargo-backed v1 lanes.
//!
//! The public `titania-check run-lane` dispatcher uses this module to execute
//! cargo-native lanes without depending on the compatibility `run-cargo` binary.

use super::run_cargo::{
    CargoLane, RunCargoError, args_for_lane, clean_outcome, findings_outcome, process_termination,
};
use crate::{LaneExit, LaneReport};
use titania_core::{Lane, LaneFailure, LaneOutcome, TargetProject, discover_target};

/// Cargo-backed lane dispatch result.
pub(super) enum CargoRun {
    Unsupported,
    Complete(LaneOutcome),
    Error { exit: LaneExit, message: String },
}

/// Run the Cargo-backed implementation for known Cargo lanes.
pub(super) fn run(lane: Lane) -> CargoRun {
    let Some(lane_name) = cargo_lane_name(lane) else {
        return CargoRun::Unsupported;
    };
    match run_checked(lane_name) {
        Ok(outcome) => CargoRun::Complete(outcome),
        Err(error) => CargoRun::Error { exit: error_exit(&error), message: error_message(error) },
    }
}

/// Map an outer [`Lane`] to its cargo sub-lane name.
const fn cargo_lane_name(lane: Lane) -> Option<&'static str> {
    match lane {
        Lane::Fmt => Some("fmt"),
        Lane::Compile => Some("compile"),
        Lane::Clippy => Some("clippy"),
        Lane::Test => Some("test"),
        Lane::Build => Some("build"),
        Lane::AstGrep | Lane::Dylint | Lane::PanicScan | Lane::PolicyScan | Lane::Deny => None,
    }
}

const fn error_exit(error: &RunCargoError) -> LaneExit {
    match error {
        RunCargoError::Usage(_) | RunCargoError::Target(_) => LaneExit::Usage,
        _other => LaneExit::Failure,
    }
}

fn error_message(error: RunCargoError) -> String {
    match error {
        RunCargoError::Usage(message) | RunCargoError::ToolVersion(message) => message,
        RunCargoError::Target(error) => format!("target discovery failed: {error}"),
        RunCargoError::Command(error) => format!("cargo execution failed: {error}"),
        RunCargoError::CurrentDir(error) => format!("cannot read current directory: {error}"),
        RunCargoError::RuleId(error) => format!("rule id configuration error: {error}"),
        RunCargoError::Outcome(error) => format!("lane outcome construction failed: {error}"),
    }
}

fn usage_message() -> String {
    String::from("usage: run-lane <fmt|compile|clippy|test|build>")
}

/// Resolve target state and run the selected Cargo lane.
///
/// # Errors
/// Returns [`RunCargoError`] when the sub-command is unrecognised, the CWD
/// cannot be read, no Cargo target project is discoverable, the rule ID is
/// invalid, or the lane execution fails.
fn run_checked(lane_name: &str) -> Result<LaneOutcome, RunCargoError> {
    let cargo_lane = CargoLane::parse(lane_name)
        .map_err(|_parse_error| RunCargoError::Usage(usage_message()))?;
    let cwd = std::env::current_dir().map_err(RunCargoError::CurrentDir)?;
    let target = discover_target(&cwd).map_err(RunCargoError::Target)?;
    let rule = crate::RuleId::new(cargo_lane.rule()).map_err(RunCargoError::RuleId)?;
    run_lane(&target, cargo_lane, &rule, &[])
}

/// Execute a single cargo lane and build its [`LaneOutcome`].
///
/// # Errors
/// Returns [`RunCargoError::Command`] on cargo execution failure, or any
/// outcome-construction error from [`lane_outcome`].
fn run_lane(
    target: &TargetProject,
    lane: CargoLane,
    rule: &crate::RuleId,
    extra_args: &[String],
) -> Result<LaneOutcome, RunCargoError> {
    let output = cargo_output(target, lane, extra_args).map_err(RunCargoError::Command)?;
    let mut report = LaneReport::new();
    report.record_scan();
    record_command_result(&output, lane, rule, &mut report);
    lane_outcome(target, lane, extra_args, &output, &report)
}

fn record_command_result(
    output: &crate::CommandOutput,
    lane: CargoLane,
    rule: &crate::RuleId,
    report: &mut LaneReport,
) {
    if output.success() && report.is_clean() {
        report.record_pass();
    }
    if !output.success() {
        report.push(crate::Finding::new(rule.clone(), lane.path(), 0, output_message(output)));
    }
}

fn output_message(output: &crate::CommandOutput) -> String {
    let stdout_result = output.stdout_str();
    let stderr_result = output.stderr_str();
    let stdout = output_text(&stdout_result);
    let stderr = output_text(&stderr_result);
    stderr
        .lines()
        .chain(stdout.lines())
        .find(|line| !line.trim().is_empty())
        .map_or("cargo command failed without output", |line| line)
        .to_owned()
}

const fn output_text<'a>(result: &'a Result<&'a str, crate::LaneError>) -> &'a str {
    match result {
        Ok(text) => text,
        Err(_) => "<non-UTF-8>",
    }
}

/// Dispatch to clean / failure / findings path based on output status.
///
/// # Errors
/// Returns [`RunCargoError::Outcome`] when clean-outcome construction fails.
fn lane_outcome(
    target: &TargetProject,
    lane: CargoLane,
    extra_args: &[String],
    output: &crate::CommandOutput,
    report: &LaneReport,
) -> Result<LaneOutcome, RunCargoError> {
    if output.success() && report.is_clean() {
        return clean_outcome(target, lane, extra_args, output);
    }
    if report.is_clean() {
        return Ok(LaneOutcome::Failed(tool_failure(output)));
    }
    Ok(findings_outcome(lane, report))
}

fn tool_failure(output: &crate::CommandOutput) -> LaneFailure {
    LaneFailure::Tool {
        tool: String::from("cargo"),
        termination: process_termination(output.status()),
    }
}

/// Run cargo for *lane* with the appropriate flags.
///
/// # Errors
/// Returns [`crate::LaneError`] on command construction or execution failure.
fn cargo_output(
    target: &TargetProject,
    lane: CargoLane,
    extra_args: &[String],
) -> Result<crate::CommandOutput, crate::LaneError> {
    let mut command = crate::CommandIn::new(target, "cargo")?;
    let _ = command.inherit_env();
    let _ = command.args(args_for_lane(lane));
    let _ = command.args_strings(extra_args);
    command.run_capture_raw()
}
