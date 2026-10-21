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

#[path = "run_lane_kani.rs"]
pub(super) mod run_lane_kani;
#[path = "run_lane_mutants/mod.rs"]
pub mod run_lane_mutants;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaneExecution {
    exit: LaneExit,
    stderr: String,
}

impl LaneExecution {
    #[must_use]
    pub const fn exit(&self) -> LaneExit {
        self.exit
    }

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
    #[error(transparent)]
    Kani(#[from] run_lane_kani::KaniLaneError),
    #[error(transparent)]
    Mutants(#[from] run_lane_mutants::MutantsLaneError),
    #[error("{0}")]
    Internal(String),
}

#[must_use]
pub fn execute_lane(lane: Lane) -> LaneExecution {
    match execute_lane_checked(lane) {
        Ok(execution) => execution,
        Err(error) => LaneExecution::new(LaneExit::Failure, format!("{error}\n")),
    }
}

fn execute_lane_checked(lane: Lane) -> Result<LaneExecution, RunLaneError> {
    match run_cargo_lane::run(lane) {
        run_cargo_lane::CargoRun::Complete(outcome) => execute_cargo_lane(lane, &outcome),
        run_cargo_lane::CargoRun::Error { exit, message } => {
            Ok(LaneExecution::new(exit, line(&message)))
        }
        run_cargo_lane::CargoRun::Unsupported => execute_non_cargo_lane(lane),
    }
}

fn execute_cargo_lane(lane: Lane, outcome: &LaneOutcome) -> Result<LaneExecution, RunLaneError> {
    let target = current_target_project()?;
    write_artifacts(&target, lane, outcome)?;
    Ok(execution_from_outcome(outcome))
}

fn execute_non_cargo_lane(lane: Lane) -> Result<LaneExecution, RunLaneError> {
    let target = current_target_project()?;
    let outcome = non_cargo_outcome(&target, lane)?;
    write_artifacts(&target, lane, &outcome)?;
    Ok(execution_from_outcome(&outcome))
}

fn non_cargo_outcome(target: &TargetProject, lane: Lane) -> Result<LaneOutcome, RunLaneError> {
    match lane {
        Lane::AstGrep => ast_grep_outcome(target),
        Lane::Dylint => Ok(run_lane_dylint::outcome(target)),
        Lane::PanicScan => Ok(panic_scan_outcome(target)),
        Lane::PolicyScan => policy_scan_outcome(target),
        Lane::Deny => deny_outcome(target),
        Lane::Kani => kani_outcome(target),
        Lane::Mutants => mutants_outcome(target),
        Lane::Fmt | Lane::Compile | Lane::Clippy | Lane::Test | Lane::Build => {
            Err(RunLaneError::Internal(format!("cargo lane {} was not dispatched", lane.name())))
        }
    }
}

fn kani_outcome(target: &TargetProject) -> Result<LaneOutcome, RunLaneError> {
    run_lane_kani::outcome(target).map_err(Into::into)
}

fn mutants_outcome(target: &TargetProject) -> Result<LaneOutcome, RunLaneError> {
    run_lane_mutants::outcome(target).map_err(Into::into)
}

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

fn panic_scan_outcome(_target: &TargetProject) -> LaneOutcome {
    clean_outcome_unchecked(
        Lane::PanicScan,
        "panic-scan-retired-dylint-owned-v1",
        b"Rust panic-surface policy is enforced by Dylint HOLZMAN_PANIC_* rules.",
    )
}

fn policy_scan_outcome(target: &TargetProject) -> Result<LaneOutcome, RunLaneError> {
    let report = policy_run_lane::run(target)?;
    outcome_from_report(Lane::PolicyScan, &report, "policy-scan-v1").map_err(Into::into)
}

fn deny_outcome(target: &TargetProject) -> Result<LaneOutcome, RunLaneError> {
    let output =
        match command_output(target, DENY_TOOL, &["--format", "json", "--offline", "check"]) {
            Ok(output) => output,
            Err(error) => return Ok(deny_failure_outcome(&error)),
        };
    let normalization = deny_normalization(&output)?;
    Ok(deny_normalization_outcome(&output, &normalization))
}

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
        LaneOutcome::Failed { failure } => execution_from_failure(failure),
    }
}

fn execution_from_failure(failure: &LaneFailure) -> LaneExecution {
    match failure {
        LaneFailure::Tool { tool, termination } => LaneExecution::new(
            LaneExit::Violations,
            format!("gate failure: {tool} exited {termination:?}\n"),
        ),
        LaneFailure::Infra { .. }
        | LaneFailure::Suspicious { .. }
        | LaneFailure::Resource { .. } => {
            LaneExecution::new(LaneExit::Failure, format!("lane failed: {failure:?}\n"))
        }
    }
}

fn line(message: &str) -> String {
    format!("{message}\n")
}
