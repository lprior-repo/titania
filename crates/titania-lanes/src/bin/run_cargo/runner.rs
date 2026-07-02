use std::env;

use titania_core::{TargetProject, TargetProjectError, discover_target};
use titania_lanes::{
    CommandIn, CommandOutput, Finding, LaneError, LaneReport, RuleId, RuleIdError,
};

use crate::{findings::collect_findings, lane::CargoLane, usage_message};

const COMPILE_ARGS: &[&str] = &["check", "--workspace", "--all-targets", "--frozen"];
const CLIPPY_ARGS: &[&str] = &[
    "clippy",
    "--workspace",
    "--lib",
    "--bins",
    "--examples",
    "--frozen",
    "--message-format=json",
    "--",
    "-D",
    "warnings",
    "-W",
    "clippy::all",
];
const TEST_ARGS: &[&str] =
    &["test", "--workspace", "--all-features", "--frozen", "--", "--test-threads=1"];
const BUILD_ARGS: &[&str] = &["build", "--workspace", "--release", "--frozen"];

#[derive(Debug)]
pub(crate) enum RunCargoError {
    Usage(String),
    Target(TargetProjectError),
    Command(LaneError),
    CurrentDir(std::io::Error),
    RuleId(RuleIdError),
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
    run_lane(&target, lane, &rule, &extra_args).map_err(RunCargoError::Command)
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
/// Returns the lane error reported by command construction, process execution,
/// output decoding, or command-output classification.
fn run_lane(
    target: &TargetProject,
    lane: CargoLane,
    rule: &RuleId,
    extra_args: &[String],
) -> Result<LaneReport, LaneError> {
    let mut report = scanned_report();
    let output = cargo_output(target, lane, extra_args)?;
    classify_output(&output, lane, rule, &mut report)?;
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
    let manifest = target.manifest_path();
    let mut command = CommandIn::new(target, "cargo")?;
    let _ = command.inherit_env();
    append_lane_args(&mut command, lane, manifest.as_str());
    let _ = command.args_strings(extra_args);
    command.run_capture_raw()
}

fn append_lane_args<'a>(command: &mut CommandIn<'a>, lane: CargoLane, manifest: &'a str) {
    match lane {
        CargoLane::Fmt => append_fmt_args(command, manifest),
        CargoLane::Compile => append_compile_args(command),
        CargoLane::Clippy => append_clippy_args(command),
        CargoLane::Test => append_test_args(command),
        CargoLane::Build => append_build_args(command),
    }
}

fn append_fmt_args<'a>(command: &mut CommandIn<'a>, manifest: &'a str) {
    let _ = command.arg("fmt").arg("--check").arg("--manifest-path").arg(manifest);
}

fn append_compile_args(command: &mut CommandIn<'_>) {
    let _ = command.args(COMPILE_ARGS);
}

fn append_clippy_args(command: &mut CommandIn<'_>) {
    let _ = command.args(CLIPPY_ARGS);
}

fn append_test_args(command: &mut CommandIn<'_>) {
    let _ = command.args(TEST_ARGS);
}

fn append_build_args(command: &mut CommandIn<'_>) {
    let _ = command.args(BUILD_ARGS);
}

fn fallback_message(stdout: &str, stderr: &str) -> String {
    let message = stderr
        .lines()
        .chain(stdout.lines())
        .find(|line| !line.trim().is_empty())
        .map_or("cargo command failed without output", |line| line);
    message.to_owned()
}
