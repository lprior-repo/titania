//! Proof/test orchestrator — ties discovery, scheduling, lanes, and logging together.

use crate::discovery;
use crate::lanes::{self, Lane};
use crate::logger::RunLogger;
use crate::profiles::Profile;
use crate::scheduler;
use crate::summary::{LaneResult, RunSummary};
use std::path::Path;
use std::time::Instant;

pub struct OrchestratorConfig {
    pub profile: Profile,
    pub max_jobs: usize,
    pub timeout_secs: u64,
    pub fail_fast: bool,
    pub dry_run: bool,
    pub json_output: bool,
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
}

pub fn run_proof(
    workspace_root: &Path,
    config: &OrchestratorConfig,
) -> anyhow::Result<(i32, RunSummary)> {
    let run_id = crate::logger::generate_run_id();
    let logger = RunLogger::new(&run_id);

    let crates = discovery::discover_crates(workspace_root)?;
    let crates = discovery::filter_crates(
        &crates,
        config.include.as_deref(),
        config.exclude.as_deref(),
    );

    let schedule = scheduler::build_schedule(&crates, config.max_jobs);
    let available_lanes = lanes::detect_available_lanes(workspace_root);
    let profile_lanes: std::collections::HashSet<_> =
        config.profile.lanes().iter().copied().collect();

    let mut results = Vec::new();
    let mut any_failure = false;

    for level in &schedule.levels {
        for crate_name in &level.crates {
            for lane in &available_lanes {
                if !profile_lanes.contains(lane.name.as_str()) {
                    continue;
                }

                let result = execute_lane(crate_name, lane, workspace_root, &logger, config)?;

                if result.status == "fail" {
                    any_failure = true;
                    if config.fail_fast {
                        results.push(result);
                        let summary = RunSummary { run_id, results };
                        return Ok((1, summary));
                    }
                }
                results.push(result);
            }
        }
    }

    let summary = RunSummary { run_id, results };
    let exit_code = if any_failure { 1 } else { 0 };
    Ok((exit_code, summary))
}

fn execute_lane(
    crate_name: &str,
    lane: &Lane,
    workspace_root: &Path,
    logger: &RunLogger,
    config: &OrchestratorConfig,
) -> anyhow::Result<LaneResult> {
    let command = lanes::lane_command(lane, crate_name, workspace_root);
    let cmd_str = command.join(" ");

    if config.dry_run {
        logger.log_entry(crate_name, &lane.name, &cmd_str, None, 0, "dry-run")?;
        return Ok(LaneResult {
            crate_name: crate_name.to_string(),
            lane: lane.name.clone(),
            status: "dry-run".to_string(),
            duration_ms: 0,
        });
    }

    let start = Instant::now();
    let status = std::process::Command::new(command.first().map(String::as_str).unwrap_or("echo"))
        .args(command.iter().skip(1))
        .current_dir(workspace_root)
        .output()
        .map(|o| if o.status.success() { "pass" } else { "fail" })
        .unwrap_or("fail");

    let duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
    let exit_code = if status == "pass" { Some(0) } else { Some(1) };

    logger.log_entry(
        crate_name,
        &lane.name,
        &cmd_str,
        exit_code,
        duration_ms,
        status,
    )?;

    Ok(LaneResult {
        crate_name: crate_name.to_string(),
        lane: lane.name.clone(),
        status: status.to_string(),
        duration_ms,
    })
}

pub fn run_proof_for_crate(
    workspace_root: &Path,
    crate_name: &str,
    lane_names: &[String],
    config: &OrchestratorConfig,
) -> anyhow::Result<(i32, RunSummary)> {
    let run_id = crate::logger::generate_run_id();
    let logger = RunLogger::new(&run_id);
    let available_lanes = lanes::detect_available_lanes(workspace_root);

    let mut results = Vec::new();
    let mut any_failure = false;

    for lane_name in lane_names {
        let Some(lane) = available_lanes.iter().find(|l| &l.name == lane_name) else {
            continue;
        };

        let result = execute_lane(crate_name, lane, workspace_root, &logger, config)?;
        if result.status == "fail" {
            any_failure = true;
            if config.fail_fast {
                results.push(result);
                let summary = RunSummary { run_id, results };
                return Ok((1, summary));
            }
        }
        results.push(result);
    }

    let summary = RunSummary { run_id, results };
    let exit_code = if any_failure { 1 } else { 0 };
    Ok((exit_code, summary))
}
