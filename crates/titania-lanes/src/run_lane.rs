//! Public v1 `run-lane` dispatcher used by `titania-check`.
#[path = "policy_run_lane.rs"]
mod policy_run_lane;
#[path = "run_cargo/mod.rs"]
mod run_cargo;
#[path = "run_cargo_lane.rs"]
mod run_cargo_lane;
#[path = "run_lane_dylint.rs"]
mod run_lane_dylint;
#[path = "run_lane_outcome.rs"]
mod run_lane_outcome;
#[path = "run_lane_sources.rs"]
pub(super) mod run_lane_sources;

use thiserror::Error;
use titania_core::{Lane, LaneFailure, LaneOutcome, TargetProject};

use crate::{
    CommandIn, CommandOutput, LaneError, LaneExit, LaneReport, RuleIdError, ast_grep_lane,
    current_target_project,
    deny_normalizer::{DenyNormalization, deny_missing_binary, normalize_deny_json},
    policy_scan::exceptions::load_exceptions,
};
use run_lane_outcome::{
    OutcomeBuildError, clean_outcome_unchecked, findings_outcome, outcome_from_report,
    process_termination, write_artifacts,
};
use run_lane_sources::{SourceWalkError, collect_rust_sources};

const AST_GREP_RULES: &[&str] = &[
    include_str!("../rules/functional.yml"),
    include_str!("../rules/bypass.yml"),
    include_str!("../rules/architecture.yml"),
];
const DENY_TOOL: &str = "cargo-deny";

/// Result of executing one v1 lane from the `titania-check run-lane` shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaneExecution {
    /// Process disposition that should be returned by the CLI.
    exit: LaneExit,
    /// Text that should be emitted to stderr by the CLI.
    stderr: String,
}

impl LaneExecution {
    /// Return the process disposition for the lane.
    #[must_use]
    pub const fn exit(&self) -> LaneExit {
        self.exit
    }

    /// Return stderr text captured or synthesized for the lane.
    #[must_use]
    pub fn stderr(&self) -> &str {
        &self.stderr
    }

    const fn new(exit: LaneExit, stderr: String) -> Self {
        Self { exit, stderr }
    }
}

#[derive(Debug, Error)]
enum RunLaneError {
    #[error(transparent)]
    AstGrep(#[from] ast_grep_lane::AstGrepLaneError),
    #[error(transparent)]
    CurrentTarget(#[from] crate::CurrentTargetError),
    #[error(transparent)]
    LaneCommand(#[from] LaneError),
    #[error(transparent)]
    Outcome(#[from] OutcomeBuildError),
    #[error(transparent)]
    Policy(#[from] policy_run_lane::PolicyRunError),
    #[error(transparent)]
    RuleId(#[from] RuleIdError),
    #[error(transparent)]
    SourceWalk(#[from] SourceWalkError),
    #[error("{0}")]
    Internal(String),
}

/// Execute one v1 lane and write its typed artifacts for every containing scope.
#[must_use]
pub fn execute_lane(lane: Lane) -> LaneExecution {
    match execute_lane_checked(lane) {
        Ok(execution) => execution,
        Err(error) => LaneExecution::new(LaneExit::Failure, format!("{error}\n")),
    }
}

/// # Errors
/// Target discovery, lane execution, or artifact writing can fail.
fn execute_lane_checked(lane: Lane) -> Result<LaneExecution, RunLaneError> {
    match run_cargo_lane::run(lane) {
        run_cargo_lane::CargoRun::Complete(outcome) => execute_cargo_lane(lane, &outcome),
        run_cargo_lane::CargoRun::Error { exit, message } => {
            Ok(LaneExecution::new(exit, line(&message)))
        }
        run_cargo_lane::CargoRun::Unsupported => execute_non_cargo_lane(lane),
    }
}

/// Write typed artifacts for a Cargo-backed lane outcome.
/// # Errors
/// Returns [`RunLaneError`] when target discovery or artifact writing fails.
fn execute_cargo_lane(lane: Lane, outcome: &LaneOutcome) -> Result<LaneExecution, RunLaneError> {
    let target = current_target_project()?;
    write_artifacts(&target, lane, outcome)?;
    Ok(execution_from_outcome(outcome))
}

/// Execute a non-Cargo lane through its typed implementation.
/// # Errors
/// Returns [`RunLaneError`] when lane execution or artifact writing fails.
fn execute_non_cargo_lane(lane: Lane) -> Result<LaneExecution, RunLaneError> {
    let target = current_target_project()?;
    let outcome = non_cargo_outcome(&target, lane)?;
    write_artifacts(&target, lane, &outcome)?;
    Ok(execution_from_outcome(&outcome))
}

/// Build the typed outcome for a non-Cargo lane.
/// # Errors
/// Returns [`RunLaneError`] when the lane cannot produce an outcome.
fn non_cargo_outcome(target: &TargetProject, lane: Lane) -> Result<LaneOutcome, RunLaneError> {
    match lane {
        Lane::AstGrep => ast_grep_outcome(target),
        Lane::Dylint => Ok(run_lane_dylint::outcome(target)),
        Lane::PanicScan => Ok(panic_scan_outcome(target)),
        Lane::PolicyScan => policy_scan_outcome(target),
        Lane::Deny => deny_outcome(target),
        Lane::Fmt | Lane::Compile | Lane::Clippy | Lane::Test | Lane::Build => {
            Err(RunLaneError::Internal(format!("cargo lane {} was not dispatched", lane.name())))
        }
    }
}

/// Run the embedded ast-grep lane.
/// # Errors
/// Returns [`RunLaneError`] when source discovery or ast-grep outcome building fails.
fn ast_grep_outcome(target: &TargetProject) -> Result<LaneOutcome, RunLaneError> {
    let sources = collect_rust_sources(target.as_std_path())?;
    let today = policy_run_lane::policy_date(target)?;
    let mut report = LaneReport::new();
    let exceptions = load_exceptions(target.as_std_path(), &today, &mut report)?;
    if !report.is_clean() {
        return Ok(findings_outcome(Lane::AstGrep, &report));
    }
    let exception_pairs = exceptions
        .into_iter()
        .map(|exception| (exception.rule_id, exception.path.as_str().to_owned()))
        .collect::<Vec<_>>();
    ast_grep_lane::run(AST_GREP_RULES, &sources, &exception_pairs).map_err(Into::into)
}

/// Run the panic-surface scanner lane.
fn panic_scan_outcome(_target: &TargetProject) -> LaneOutcome {
    clean_outcome_unchecked(
        Lane::PanicScan,
        "panic-scan-retired-dylint-owned-v1",
        b"Rust panic-surface policy is enforced by Dylint HOLZMAN_PANIC_* rules.",
    )
}

/// Run the policy input scanner lane.
/// # Errors
/// Returns [`RunLaneError`] when policy scanning or outcome evidence fails.
fn policy_scan_outcome(target: &TargetProject) -> Result<LaneOutcome, RunLaneError> {
    let report = policy_run_lane::run(target)?;
    outcome_from_report(Lane::PolicyScan, &report, "policy-scan-v1").map_err(Into::into)
}

/// Run cargo-deny JSON normalization for the deny lane.
/// # Errors
/// Returns [`RunLaneError`] when output decoding or normalization fails.
fn deny_outcome(target: &TargetProject) -> Result<LaneOutcome, RunLaneError> {
    let output = match command_output(target, DENY_TOOL, &["--format", "json", "check"]) {
        Ok(output) => output,
        Err(error) => return Ok(deny_failure_outcome(&error)),
    };
    let normalization = deny_normalization(&output)?;
    Ok(deny_normalization_outcome(&output, &normalization))
}

/// Run one command in the target project and capture raw output.
/// # Errors
/// Returns [`LaneError`] when command construction or execution fails.
fn command_output(
    target: &TargetProject,
    program: &'static str,
    args: &'static [&'static str],
) -> Result<CommandOutput, LaneError> {
    let mut command = CommandIn::new(target, program)?;
    let _ = command.inherit_env();
    let _ = command.args(args);
    command.run_capture_raw()
}

/// Normalize cargo-deny stdout and stderr into typed findings.
/// # Errors
/// Returns [`RunLaneError`] when command output is not valid UTF-8.
fn deny_normalization(output: &CommandOutput) -> Result<DenyNormalization, RunLaneError> {
    let stdout = output.stdout_str()?;
    let stderr = output.stderr_str()?;
    Ok(normalize_deny_json(&format!("{stdout}\n{stderr}")))
}

fn deny_normalization_outcome(output: &CommandOutput, result: &DenyNormalization) -> LaneOutcome {
    if let Some(failure) = result.failure() {
        return LaneOutcome::Failed { failure: failure.clone() };
    }
    let report = report_from_deny(result);
    if output.success() && report.is_clean() {
        return clean_outcome_unchecked(Lane::Deny, "cargo-deny-json", b"deny-clean");
    }
    if report.is_clean() {
        return LaneOutcome::Failed {
            failure: LaneFailure::Tool {
                tool: String::from(DENY_TOOL),
                termination: process_termination(output),
            },
        };
    }
    findings_outcome(Lane::Deny, &report)
}

fn report_from_deny(result: &DenyNormalization) -> LaneReport {
    let mut report = LaneReport::new();
    report.extend_finding(result.findings().iter().cloned());
    report
}

fn deny_failure_outcome(error: &LaneError) -> LaneOutcome {
    deny_missing_binary().failure().map_or_else(
        || LaneOutcome::Failed {
            failure: LaneFailure::Infra {
                tool: String::from(DENY_TOOL),
                reason: error.to_string(),
            },
        },
        |failure| LaneOutcome::Failed { failure: failure.clone() },
    )
}

fn execution_from_outcome(outcome: &LaneOutcome) -> LaneExecution {
    match outcome {
        LaneOutcome::Clean { .. } | LaneOutcome::Skipped { .. } => {
            LaneExecution::new(LaneExit::Clean, String::new())
        }
        LaneOutcome::Findings { findings } => {
            LaneExecution::new(LaneExit::Violations, format!("{} finding(s)\n", findings.len()))
        }
        LaneOutcome::Failed { failure } => {
            LaneExecution::new(LaneExit::Failure, format!("lane failed: {failure:?}\n"))
        }
    }
}

fn line(message: &str) -> String {
    format!("{message}\n")
}
