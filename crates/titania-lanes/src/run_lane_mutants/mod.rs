mod baseline;
mod command;
mod constants;
pub mod error;
mod operators;
mod outcomes;
mod report;
mod state;
mod survivors;
mod version;

use std::time::{SystemTime, UNIX_EPOCH};

use titania_core::{Finding, LaneOutcome, SkipReason, TargetProject, ToolKind};

pub(super) use error::MutantsLaneError;
use state::{LaneRunState, NewSurvivor};

use self::{
    baseline::{baseline_path, load_baseline},
    command::run_workspace_command,
    outcomes::{
        baseline_missing_outcome, build_clean_outcome, map_outcome_error, survivor_finding,
    },
    report::{read_workspace_report, validate_report_exit},
    survivors::build_new_survivors,
    version::{
        detect_cargo_mutants_version, major_meets_floor, parse_cargo_mutants_major,
        probe_systemd_run,
    },
};

pub(super) fn outcome(target: &TargetProject) -> Result<LaneOutcome, MutantsLaneError> {
    let workspace_root = target.as_std_path();

    let Some(tool_version) = available_tool_version() else {
        return Ok(tool_unavailable_outcome());
    };

    let baseline_path_value = baseline_path(workspace_root);
    if let Err(MutantsLaneError::BaselineMissing(label)) = load_baseline(&baseline_path_value) {
        return Ok(baseline_missing_outcome(&label));
    }

    let state = LaneRunState { tool_version, cgroup_used: probe_systemd_run() };

    let exit_code = run_workspace_command(workspace_root, state.cgroup_used)?;
    let report = read_workspace_report(workspace_root, exit_code)?;
    validate_report_exit(&report)?;

    let now_unix = current_unix();
    let new_survivors = build_new_survivors(&report.survivor_names, workspace_root, now_unix)?;
    if new_survivors.is_empty() {
        return build_clean_outcome(&state).map_err(|error| map_outcome_error(&error));
    }
    Ok(LaneOutcome::Findings { findings: findings_for_new_survivors(&new_survivors)? })
}

fn findings_for_new_survivors(
    new_survivors: &[NewSurvivor],
) -> Result<Box<[Finding]>, MutantsLaneError> {
    new_survivors
        .iter()
        .map(survivor_finding)
        .collect::<Result<Vec<_>, _>>()
        .map(Vec::into_boxed_slice)
}

fn available_tool_version() -> Option<String> {
    let tool_version = detect_cargo_mutants_version()?;
    let major_version = parse_cargo_mutants_major(&tool_version)?;
    major_meets_floor(major_version).then_some(tool_version)
}

#[must_use]
pub(super) const fn tool_unavailable_outcome() -> LaneOutcome {
    LaneOutcome::Skipped { reason: SkipReason::ToolUnavailable(ToolKind::CargoMutants) }
}

#[must_use]
pub(super) fn current_unix() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use titania_core::{LaneOutcome, SkipReason};

    use super::{current_unix, tool_unavailable_outcome};

    #[test]
    fn tool_unavailable_outcome_records_skip_reason() {
        let outcome = tool_unavailable_outcome();
        let LaneOutcome::Skipped { reason } = outcome else {
            panic!("tool-unavailable must surface as Skipped");
        };
        assert!(matches!(reason, SkipReason::ToolUnavailable(_)));
    }

    #[test]
    fn current_unix_returns_non_zero_on_real_clock() {
        assert!(current_unix() > 1_700_000_000);
    }
}
