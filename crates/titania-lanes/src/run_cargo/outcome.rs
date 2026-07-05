//! Outcome construction for cargo lanes.
//!
//! Holds helpers that turn a raw [`CommandOutput`](crate::CommandOutput) and
//! [`LaneReport`](crate::LaneReport) into a typed [`LaneOutcome`](titania_core::LaneOutcome).

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

use titania_core::{
    CommandEvidence, Digest, Finding as CoreFinding, LaneEvidence, Location, ProcessTermination,
    RepairHint, TargetProject,
};

use super::{
    CargoLane, RunCargoError,
    args::{args_for_lane, core_lane, version_args},
};
use crate::{CommandIn, Finding, LaneReport};

/// Build clean-lane evidence including command argv and tool version.
///
/// # Errors
/// Returns [`RunCargoError::Outcome`] when command or lane evidence
/// construction fails, or [`RunCargoError::Command`] when the tool
/// version command cannot run.
pub(super) fn clean_outcome(
    target: &TargetProject,
    lane: CargoLane,
    extra_args: &[String],
    output: &crate::CommandOutput,
) -> Result<titania_core::LaneOutcome, RunCargoError> {
    let evidence = LaneEvidence::new(
        command_evidence(lane, extra_args)?,
        tool_version(target, lane)?,
        process_termination(output.status()),
        Digest::from_bytes(output.stdout()),
    )
    .map_err(RunCargoError::Outcome)?;
    Ok(titania_core::LaneOutcome::Clean { evidence })
}

/// Build [`CommandEvidence`] recording the underlying cargo argv.
///
/// # Errors
/// Returns [`RunCargoError::Outcome`] on invalid command evidence.
fn command_evidence(
    lane: CargoLane,
    extra_args: &[String],
) -> Result<CommandEvidence, RunCargoError> {
    CommandEvidence::new(String::from("cargo"), argv_for_lane(lane, extra_args))
        .map_err(RunCargoError::Outcome)
}

/// Build the argv box for a cargo invocation including *lane* args.
fn argv_for_lane(lane: CargoLane, extra_args: &[String]) -> Box<[String]> {
    std::iter::once(String::from("cargo"))
        .chain(args_for_lane(lane).iter().map(|arg| (*arg).to_owned()))
        .chain(extra_args.iter().cloned())
        .collect()
}

/// Fetch tool version by running `cargo <version_args>` separately from the
/// lane command whose stdout may legitimately be empty.
///
/// # Errors
/// Returns [`RunCargoError::Command`] on execution or decoding failure,
/// or [`RunCargoError::ToolVersion`] when the version output is empty.
fn tool_version(target: &TargetProject, lane: CargoLane) -> Result<String, RunCargoError> {
    let mut command = CommandIn::new(target, "cargo").map_err(RunCargoError::Command)?;
    let _ = command.inherit_env();
    let _ = command.args(version_args(lane));
    let output = command.run_capture_raw().map_err(RunCargoError::Command)?;
    let stdout = output.stdout_str().map_err(RunCargoError::Command)?;
    stdout.lines().find(|line| !line.trim().is_empty()).map_or_else(
        || Err(RunCargoError::ToolVersion(String::from("version command produced no output"))),
        |version_line| Ok(version_line.to_owned()),
    )
}

/// Convert a raw [`ExitStatus`](std::process::ExitStatus) into a
/// [`ProcessTermination`].
pub(super) fn process_termination(status: std::process::ExitStatus) -> ProcessTermination {
    status
        .code()
        .map_or_else(|| termination_from_signal(status), |code| ProcessTermination::Exited { code })
}

#[cfg(unix)]
fn termination_from_signal(status: std::process::ExitStatus) -> ProcessTermination {
    status
        .signal()
        .and_then(|signal| ProcessTermination::signaled(signal).ok())
        .map_or(ProcessTermination::SpawnFailed, std::convert::identity)
}

#[cfg(not(unix))]
fn termination_from_signal(_status: std::process::ExitStatus) -> ProcessTermination {
    ProcessTermination::SpawnFailed
}

/// Turn a report with findings into a [`LaneOutcome::Findings`](titania_core::LaneOutcome::Findings).
pub(super) fn findings_outcome(lane: CargoLane, report: &LaneReport) -> titania_core::LaneOutcome {
    let findings = report.findings().iter().map(|finding| core_finding(lane, finding)).collect();
    titania_core::LaneOutcome::Findings { findings }
}

/// Convert an inner [`Finding`] into a core [`CoreFinding`].
fn core_finding(lane: CargoLane, finding: &Finding) -> CoreFinding {
    CoreFinding::reject(
        core_lane(lane),
        finding.rule().clone(),
        Location::Workspace,
        finding.message().to_owned(),
        RepairHint::RequiresHumanReview { note: finding.path().to_owned() },
    )
}
