use std::env;

use titania_core::{
    FindingError, OutcomeError, TargetProject, TargetProjectError, discover_target,
};
use titania_lanes::{
    CommandIn, CommandOutput, Finding, LaneError, LaneReport, RuleId, RuleIdError,
    artifact_writer::ArtifactWriterError,
};

use crate::{
    artifacts::{write_failure_artifact, write_success_artifact},
    findings::collect_findings,
    lane::CargoLane,
    usage_message,
};

const COMPILE_ARGS: &[&str] = &["check", "--workspace", "--frozen"];
const CLIPPY_ARGS: &[&str] = &[
    "clippy",
    "--workspace",
    "--lib",
    "--bins",
    "--examples",
    "--frozen",
    "--message-format=json",
    "--",
    "-F",
    "clippy::unwrap_used",
    "-F",
    "clippy::expect_used",
    "-F",
    "clippy::panic",
    "-F",
    "clippy::panic_in_result_fn",
    "-F",
    "clippy::todo",
    "-F",
    "clippy::unimplemented",
    "-F",
    "clippy::indexing_slicing",
    "-F",
    "clippy::string_slice",
    "-F",
    "clippy::get_unwrap",
    "-F",
    "clippy::arithmetic_side_effects",
    "-F",
    "clippy::dbg_macro",
    "-F",
    "clippy::as_conversions",
    "-F",
    "clippy::let_underscore_must_use",
    "-F",
    "clippy::await_holding_lock",
    "-D",
    "warnings",
];
const TEST_ARGS: &[&str] = &["test", "--workspace", "--frozen", "--", "--test-threads=1"];
const BUILD_ARGS: &[&str] = &["build", "--workspace", "--release", "--frozen"];

#[derive(Debug)]
pub(crate) enum RunCargoError {
    Usage(String),
    Target(TargetProjectError),
    Command(LaneError),
    CurrentDir(std::io::Error),
    RuleId(RuleIdError),
    Artifact(ArtifactWriterError),
    Outcome(OutcomeError),
    Finding(FindingError),
    ToolVersion(String),
}

/// Runs the selected Cargo lane and returns its lane report.
///
/// # Errors
///
/// Returns a typed error when invocation parsing fails, the target project
/// cannot be discovered, a rule id is invalid, the current directory cannot be
/// read, or Cargo execution fails.
pub(crate) fn run_checked(args: Vec<String>) -> Result<LaneReport, RunCargoError> {
    let mut rest = args.into_iter();
    let _program = rest.next();
    let lane = selected_lane(&mut rest)?;
    let rule = rule_for_lane(lane)?;
    let extra_args: Vec<String> = rest.collect();
    let cwd = env::current_dir().map_err(RunCargoError::CurrentDir)?;
    let target = discover_target(&cwd).map_err(RunCargoError::Target)?;
    run_lane(&target, lane, &rule, &extra_args)
}

/// Select the requested Cargo lane from command-line arguments.
///
/// # Errors
///
/// Returns a usage error when the subcommand is missing or unknown.
fn selected_lane(rest: &mut impl Iterator<Item = String>) -> Result<CargoLane, RunCargoError> {
    let subcommand = rest.next().ok_or_else(|| RunCargoError::Usage(usage_message()))?;
    CargoLane::parse(&subcommand).map_err(RunCargoError::Usage)
}

/// Build the rule id associated with a Cargo lane.
///
/// # Errors
///
/// Returns a rule-id configuration error if the static rule id is invalid.
fn rule_for_lane(lane: CargoLane) -> Result<RuleId, RunCargoError> {
    RuleId::new(lane.rule()).map_err(RunCargoError::RuleId)
}

/// Executes one Cargo lane and converts Cargo output into a `LaneReport`.
///
/// # Errors
///
/// Returns typed errors from command construction/execution, output decoding,
/// lane-outcome construction, or artifact writing.
fn run_lane(
    target: &TargetProject,
    lane: CargoLane,
    rule: &RuleId,
    extra_args: &[String],
) -> Result<LaneReport, RunCargoError> {
    let mut report = scanned_report();
    let output = match cargo_output(target, lane, extra_args) {
        Ok(output) => output,
        Err(error) => {
            write_failure_artifact(target, lane, &error)?;
            return Err(RunCargoError::Command(error));
        }
    };
    if let Err(error) = classify_output(&output, lane, rule, &mut report) {
        write_failure_artifact(target, lane, &error)?;
        return Err(RunCargoError::Command(error));
    }
    write_success_artifact(target, lane, extra_args, &output, &report)?;
    Ok(report)
}

fn scanned_report() -> LaneReport {
    let mut report = LaneReport::new();
    report.record_scan();
    report
}

/// Decode Cargo output and record lane findings.
///
/// # Errors
///
/// Returns a lane error when captured stdout/stderr is not valid UTF-8.
fn classify_output(
    output: &CommandOutput,
    lane: CargoLane,
    rule: &RuleId,
    report: &mut LaneReport,
) -> Result<(), LaneError> {
    let stdout = output.stdout_str()?;
    let stderr = output.stderr_str()?;
    collect_findings(lane, rule, stdout, stderr, report);
    record_outcome(&CargoOutcome { output, lane, rule, stdout, stderr }, report);
    Ok(())
}

struct CargoOutcome<'a> {
    output: &'a CommandOutput,
    lane: CargoLane,
    rule: &'a RuleId,
    stdout: &'a str,
    stderr: &'a str,
}

fn record_outcome(outcome: &CargoOutcome<'_>, report: &mut LaneReport) {
    if outcome.output.success() && report.is_clean() {
        report.record_pass();
    }
    if !outcome.output.success() && report.is_clean() {
        report.push(Finding::new(
            outcome.rule.clone(),
            outcome.lane.path(),
            0,
            fallback_message(outcome.stdout, outcome.stderr),
        ));
    }
}

/// Builds and runs the Cargo command for the selected lane.
///
/// # Errors
///
/// Returns a lane error when the command cannot be constructed or Cargo cannot
/// be executed with captured output.
fn cargo_output(
    target: &TargetProject,
    lane: CargoLane,
    extra_args: &[String],
) -> Result<CommandOutput, LaneError> {
    let mut command = CommandIn::new(target, "cargo")?;
    let _ = command.inherit_env();
    append_lane_args(&mut command, lane);
    let _ = command.args_strings(extra_args);
    command.run_capture_raw()
}

fn append_lane_args(command: &mut CommandIn<'_>, lane: CargoLane) {
    let _ = command.args(args_for_lane(lane));
}

pub(crate) const fn args_for_lane(lane: CargoLane) -> &'static [&'static str] {
    match lane {
        CargoLane::Fmt => FMT_ARGS,
        CargoLane::Compile => COMPILE_ARGS,
        CargoLane::Clippy => CLIPPY_ARGS,
        CargoLane::Test => TEST_ARGS,
        CargoLane::Build => BUILD_ARGS,
    }
}

const FMT_ARGS: &[&str] = &["fmt", "--all", "--check"];

fn fallback_message(stdout: &str, stderr: &str) -> String {
    let message = stderr
        .lines()
        .chain(stdout.lines())
        .find(|line| !line.trim().is_empty())
        .map_or("cargo command failed without output", |line| line);
    message.to_owned()
}
