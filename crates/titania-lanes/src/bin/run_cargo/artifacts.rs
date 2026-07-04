use titania_core::{
    CommandEvidence, Digest, Finding as CoreFinding, FindingEffect, GateScope, Lane, LaneEvidence,
    LaneFailure, LaneOutcome, Location, ProcessTermination, RepairHint, TargetProject,
};
use titania_lanes::{
    CommandIn, CommandOutput, Finding, LaneError, LaneReport, artifact_writer::write_lane_artifact,
};

use crate::{
    lane::CargoLane,
    runner::{RunCargoError, args_for_lane},
};

/// Write the artifact for a lane that reached a Cargo process result.
///
/// # Errors
///
/// Returns a typed error when outcome construction or artifact writing fails.
pub(crate) fn write_success_artifact(
    target: &TargetProject,
    lane: CargoLane,
    extra_args: &[String],
    output: &CommandOutput,
    report: &LaneReport,
) -> Result<(), RunCargoError> {
    let outcome = lane_outcome(target, lane, extra_args, output, report)?;
    write_artifact(target, lane, &outcome)
}

/// Write the artifact for a lane infrastructure or decoding failure.
///
/// # Errors
///
/// Returns a typed error when artifact writing fails.
pub(crate) fn write_failure_artifact(
    target: &TargetProject,
    lane: CargoLane,
    error: &LaneError,
) -> Result<(), RunCargoError> {
    write_artifact(target, lane, &LaneOutcome::Failed(failure_for_error(error)))
}

/// Persist one typed lane artifact for the selected Cargo lane.
///
/// # Errors
///
/// Returns a typed error when the atomic artifact writer rejects the target.
fn write_artifact(
    target: &TargetProject,
    lane: CargoLane,
    outcome: &LaneOutcome,
) -> Result<(), RunCargoError> {
    write_lane_artifact(target.as_std_path(), gate_scope(lane), core_lane(lane), outcome)
        .map(|_path| ())
        .map_err(RunCargoError::Artifact)
}

/// Classify Cargo output and legacy findings into one v1 lane outcome.
///
/// # Errors
///
/// Returns a typed error when command evidence or findings cannot be built.
fn lane_outcome(
    target: &TargetProject,
    lane: CargoLane,
    extra_args: &[String],
    output: &CommandOutput,
    report: &LaneReport,
) -> Result<LaneOutcome, RunCargoError> {
    if output.success() && report.is_clean() {
        return clean_outcome(target, lane, extra_args, output);
    }
    if report.is_clean() {
        return Ok(LaneOutcome::Failed(tool_failure(output)));
    }
    findings_outcome(lane, report)
}

/// Build a clean outcome with command evidence.
///
/// # Errors
///
/// Returns a typed error when command evidence or clean evidence is invalid.
fn clean_outcome(
    target: &TargetProject,
    lane: CargoLane,
    extra_args: &[String],
    output: &CommandOutput,
) -> Result<LaneOutcome, RunCargoError> {
    let evidence = LaneEvidence::new(
        command_evidence(lane, extra_args)?,
        tool_version(target, lane)?,
        process_termination(output.status()),
        Digest::from_bytes(output.stdout()),
    )
    .map_err(RunCargoError::Outcome)?;
    Ok(LaneOutcome::Clean { evidence })
}

/// Convert legacy lane findings into a v1 findings outcome.
///
/// # Errors
///
/// Returns a typed error when a finding cannot be represented in core v1.
fn findings_outcome(lane: CargoLane, report: &LaneReport) -> Result<LaneOutcome, RunCargoError> {
    report
        .findings()
        .iter()
        .map(|finding| core_finding(lane, finding))
        .collect::<Result<Box<[_]>, _>>()
        .map(|findings| LaneOutcome::Findings { findings })
}

/// Convert one legacy lane finding into a core v1 finding.
///
/// # Errors
///
/// Returns a typed error when core finding construction rejects the value.
fn core_finding(lane: CargoLane, finding: &Finding) -> Result<CoreFinding, RunCargoError> {
    CoreFinding::new(
        core_lane(lane),
        finding.rule().clone(),
        Location::Workspace,
        finding.message().to_owned(),
        RepairHint::RequiresHumanReview { note: finding.path().to_owned() },
        FindingEffect::Reject,
    )
    .map_err(RunCargoError::Finding)
}

/// Build command evidence for the selected Cargo lane.
///
/// # Errors
///
/// Returns a typed error when the generated argv violates evidence invariants.
fn command_evidence(
    lane: CargoLane,
    extra_args: &[String],
) -> Result<CommandEvidence, RunCargoError> {
    let argv = argv_for_lane(lane, extra_args);
    CommandEvidence::new(String::from("cargo"), argv).map_err(RunCargoError::Outcome)
}

fn argv_for_lane(lane: CargoLane, extra_args: &[String]) -> Box<[String]> {
    std::iter::once("cargo".to_owned())
        .chain(args_for_lane(lane).iter().map(|arg| (*arg).to_owned()))
        .chain(extra_args.iter().cloned())
        .collect()
}

/// Capture the concrete tool version for clean-lane evidence.
///
/// # Errors
///
/// Returns a typed error when the version command fails or prints no version.
fn tool_version(target: &TargetProject, lane: CargoLane) -> Result<String, RunCargoError> {
    let mut command = CommandIn::new(target, "cargo").map_err(RunCargoError::Command)?;
    let _ = command.inherit_env();
    let _ = command.args(version_args(lane));
    let output = command.run_capture().map_err(RunCargoError::Command)?;
    let stdout = output.stdout_str().map_err(RunCargoError::Command)?;
    stdout.lines().find(|line| !line.trim().is_empty()).map(str::to_owned).ok_or_else(|| {
        RunCargoError::ToolVersion(String::from("version command produced no output"))
    })
}

const fn version_args(lane: CargoLane) -> &'static [&'static str] {
    match lane {
        CargoLane::Fmt => &["fmt", "--version"],
        CargoLane::Compile | CargoLane::Clippy | CargoLane::Test | CargoLane::Build => {
            &["--version"]
        }
    }
}

const fn core_lane(lane: CargoLane) -> Lane {
    match lane {
        CargoLane::Fmt => Lane::Fmt,
        CargoLane::Compile => Lane::Compile,
        CargoLane::Clippy => Lane::Clippy,
        CargoLane::Test => Lane::Test,
        CargoLane::Build => Lane::Build,
    }
}

const fn gate_scope(lane: CargoLane) -> GateScope {
    match lane {
        CargoLane::Fmt => GateScope::Edit,
        CargoLane::Compile | CargoLane::Clippy | CargoLane::Test => GateScope::Prepush,
        CargoLane::Build => GateScope::Release,
    }
}

fn tool_failure(output: &CommandOutput) -> LaneFailure {
    LaneFailure::ToolFailure {
        tool: String::from("cargo"),
        termination: process_termination(output.status()),
    }
}

fn process_termination(status: std::process::ExitStatus) -> ProcessTermination {
    status
        .code()
        .map_or(ProcessTermination::SpawnFailed, |code| ProcessTermination::Exited { code })
}

fn failure_for_error(error: &LaneError) -> LaneFailure {
    match error {
        LaneError::Io { source, .. } => {
            LaneFailure::InfraFailure { tool: String::from("cargo"), reason: source.to_string() }
        }
        LaneError::Timeout { .. } => LaneFailure::ToolFailure {
            tool: String::from("cargo"),
            termination: ProcessTermination::TimedOut,
        },
        LaneError::OutputLimitExceeded { limit, .. } => {
            LaneFailure::ResourceFailure { tool: String::from("cargo"), limit: limit.to_string() }
        }
        _other => LaneFailure::SuspiciousFailure {
            tool: String::from("cargo"),
            evidence: error.to_string(),
        },
    }
}
