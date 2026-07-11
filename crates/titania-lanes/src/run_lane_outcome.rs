//! Artifact and outcome helpers for the public run-lane dispatcher.

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

use thiserror::Error;
use titania_core::{
    CommandEvidence, Digest, Finding as CoreFinding, GateScope, Lane, LaneEvidence, LaneFailure,
    LaneOutcome, Location, OutcomeError, ProcessTermination, TargetProject, WorkspacePath,
};

use crate::{
    CommandOutput, Finding, LaneReport,
    artifact_writer::{ArtifactWriterError, write_lane_artifact},
};

#[derive(Debug, Error)]
pub(super) enum OutcomeBuildError {
    #[error(transparent)]
    Outcome(#[from] OutcomeError),
    #[error(transparent)]
    Artifact(#[from] ArtifactWriterError),
}

/// Convert a legacy lane report into a v1 lane outcome.
///
/// # Errors
/// Returns [`OutcomeBuildError`] when clean command evidence cannot be built.
pub(super) fn outcome_from_report(
    lane: Lane,
    report: &LaneReport,
    tool_version: &'static str,
) -> Result<LaneOutcome, OutcomeBuildError> {
    if report.is_clean() {
        return clean_outcome(lane, tool_version, report.render().as_bytes());
    }
    Ok(findings_outcome(lane, report))
}

pub(super) fn findings_outcome(lane: Lane, report: &LaneReport) -> LaneOutcome {
    let findings = report.findings().iter().map(|finding| core_finding(lane, finding)).collect();
    LaneOutcome::Findings { findings }
}

pub(super) fn clean_outcome_unchecked(
    lane: Lane,
    tool_version: &'static str,
    digest_seed: &[u8],
) -> LaneOutcome {
    match clean_outcome(lane, tool_version, digest_seed) {
        Ok(outcome) => outcome,
        Err(error) => LaneOutcome::Failed {
            failure: LaneFailure::Suspicious {
                tool: String::from("titania-check"),
                evidence: error.to_string(),
            },
        },
    }
}

/// Write one lane outcome to every gate scope that contains the lane.
///
/// # Errors
/// Returns [`OutcomeBuildError`] when any artifact write fails.
pub(super) fn write_artifacts(
    target: &TargetProject,
    lane: Lane,
    outcome: &LaneOutcome,
) -> Result<(), OutcomeBuildError> {
    containing_scopes(lane)
        .iter()
        .try_for_each(|scope| {
            write_lane_artifact(target.as_std_path(), *scope, lane, outcome).map(|_path| ())
        })
        .map_err(Into::into)
}

pub(super) fn process_termination(output: &CommandOutput) -> ProcessTermination {
    let status = output.status();
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

fn core_finding(lane: Lane, finding: &Finding) -> CoreFinding {
    // Repair hint comes from the lane Finding, which is auto-populated
    // by `RepairHint::for_rule(rule.as_str())` at construction time and
    // optionally overridden by `Finding::with_repair` for normalizers
    // with richer context. The legacy `requires_human_review` fallback
    // path is preserved by that catalog lookup (empty / unknown /
    // dynamic rule ids return that class).
    CoreFinding::reject(
        lane,
        finding.rule().clone(),
        legacy_location(finding),
        finding.message().to_owned(),
        finding.repair().clone(),
    )
}

fn legacy_location(finding: &Finding) -> Location {
    if finding.line() == 0 {
        return Location::workspace();
    }
    WorkspacePath::new(finding.path())
        .map_or_else(|_| Location::workspace(), |path| legacy_span(path, finding.line()))
}

fn legacy_span(path: WorkspacePath, line: u32) -> Location {
    let Ok(location) = Location::span(path, line, 0, line, 1) else {
        return Location::workspace();
    };
    location
}

/// Build clean lane evidence for a successful lane run.
///
/// # Errors
/// Returns [`OutcomeBuildError`] when command or lane evidence is invalid.
fn clean_outcome(
    lane: Lane,
    tool_version: &'static str,
    digest_seed: &[u8],
) -> Result<LaneOutcome, OutcomeBuildError> {
    let command = CommandEvidence::new(
        String::from("titania-check"),
        Box::from([
            String::from("titania-check"),
            String::from("run-lane"),
            String::from(lane_cli_name(lane)),
        ]),
    )?;
    let evidence = LaneEvidence::new(
        command,
        String::from(tool_version),
        ProcessTermination::Exited { code: 0 },
        Digest::from_bytes(digest_seed),
    )?;
    Ok(LaneOutcome::Clean { evidence })
}

const fn containing_scopes(lane: Lane) -> &'static [GateScope] {
    match lane {
        Lane::Fmt
        | Lane::Compile
        | Lane::Clippy
        | Lane::AstGrep
        | Lane::Dylint
        | Lane::PanicScan
        | Lane::PolicyScan => EDIT_PREPUSH_RELEASE,
        Lane::Test | Lane::Deny => PREPUSH_RELEASE,
        Lane::Build => RELEASE_ONLY,
    }
}

const EDIT_PREPUSH_RELEASE: &[GateScope] =
    &[GateScope::Edit, GateScope::Prepush, GateScope::Release];
const PREPUSH_RELEASE: &[GateScope] = &[GateScope::Prepush, GateScope::Release];
const RELEASE_ONLY: &[GateScope] = &[GateScope::Release];

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
