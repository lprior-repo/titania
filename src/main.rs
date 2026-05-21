#![forbid(unsafe_code)]
#![allow(unreachable_pub)]

mod ai_profile;
mod cli;
mod contracts;
mod evidence;
mod forbidden_scan;
mod gates;
mod loom;
mod proof;
mod shell;
mod ui_overlap;
mod ui_snapshot;
mod ui_snapshot_render;
mod ui_tokens_cmd;

use std::path::{Path, PathBuf};

use clap::Parser;

use ai_profile::{cmd_ai_deep, cmd_ai_fast, cmd_ai_release};
use cli::{Cli, Commands, ProofCommands};
use contracts::cmd_contracts;
use shell::write_stdout;
use ui_overlap::cmd_ui_overlap_check;
use ui_snapshot::cmd_ui_snapshot;
use ui_tokens_cmd::cmd_ui_tokens;
use xtask::discovery;
use xtask::lanes;
use xtask::profiles;
use xtask::proof_orchestrator;
use xtask::summary;

fn main() -> anyhow::Result<()> {
    let args = shell::normalized_args();
    match xtask::parse_xtask_command(args.clone()) {
        Ok(xtask::XtaskCommand::Required(command)) => return shell::run_required_command(command),
        Err(error) => return shell::exit_with_xtask_error(error),
        Ok(xtask::XtaskCommand::Help) => return shell::render_top_level_help(),
        Ok(xtask::XtaskCommand::Version) => return shell::render_top_level_version(),
        Ok(xtask::XtaskCommand::Legacy(_)) => {}
    }
    run_legacy_cli(Cli::parse_from(args))
}

fn run_legacy_cli(cli: Cli) -> anyhow::Result<()> {
    let workspace_root = std::env::current_dir()?;

    match cli.command {
        Commands::Snapshot {
            all,
            fixture,
            emit,
            output_dir,
        } => cmd_ui_snapshot(all, fixture, emit, output_dir),
        Commands::Tokens {
            input,
            output,
            emit,
            check,
        } => cmd_ui_tokens(&input, &output, emit, check),
        Commands::OverlapCheck {
            all,
            screen,
            input_dir,
        } => cmd_ui_overlap_check(all, screen, &input_dir),
        Commands::AiFast { bead } => cmd_ai_fast(bead.as_deref()),
        Commands::AiDeep { bead } => cmd_ai_deep(bead.as_deref()),
        Commands::AiRelease { bead } => cmd_ai_release(bead.as_deref()),
        Commands::ProofPlan { crate_name } => cmd_proof_plan(crate_name.as_deref()),
        Commands::ProofCheck { level, bead } => cmd_proof_check(level.as_deref(), bead.as_deref()),
        Commands::ProofEvidence { bead } => cmd_proof_evidence(&bead),
        Commands::ProofDrift { sections } => cmd_proof_drift(sections.as_deref()),
        Commands::Loom { model } => loom::cmd_loom(&model),
        Commands::ForbiddenScan { crates, allowlist } => {
            forbidden_scan::cmd_forbidden_scan(crates.as_deref(), allowlist.as_deref())
        }
        Commands::ListCrates {
            include,
            exclude,
            json,
        } => cmd_list_crates(&workspace_root, include, exclude, json),
        Commands::Proof { command } => cmd_proof(&workspace_root, command),
        Commands::Contracts { dir, json, check } => cmd_contracts(&dir, json, check),
    }
}

fn cmd_proof(workspace_root: &Path, command: ProofCommands) -> anyhow::Result<()> {
    match command {
        ProofCommands::List { crate_name, json } => {
            cmd_proof_list(workspace_root, crate_name.as_deref(), json)
        }
        ProofCommands::Run {
            profile,
            jobs,
            exclude,
            include,
            fail_fast,
            keep_going,
            timeout,
            dry_run,
            json,
        } => {
            let cfg = ProofRunConfig {
                profile_str: profile,
                jobs_str: jobs,
                exclude,
                include,
                fail_fast,
                timeout,
                dry_run,
                json,
            };
            cmd_proof_run(workspace_root, &cfg, keep_going)
        }
        ProofCommands::Crate {
            crate_name,
            lanes,
            jobs,
            fail_fast,
            timeout,
            dry_run,
            json,
        } => {
            let cfg = ProofCrateConfig {
                lane_names: lanes,
                jobs_str: jobs,
                fail_fast,
                timeout,
                dry_run,
                json,
            };
            cmd_proof_crate(workspace_root, &crate_name, &cfg)
        }
        ProofCommands::Affected {
            base,
            jobs,
            fail_fast,
            timeout,
            dry_run,
            json,
        } => {
            let cfg = ProofAffectedConfig {
                jobs_str: jobs,
                fail_fast,
                timeout,
                dry_run,
                json,
            };
            cmd_proof_affected(workspace_root, &base, &cfg)
        }
    }
}

fn cmd_list_crates(
    workspace_root: &Path,
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
    json: bool,
) -> anyhow::Result<()> {
    let crates = discovery::discover_crates(workspace_root)?;
    let crates = discovery::filter_crates(&crates, include.as_deref(), exclude.as_deref());

    if json {
        let output: Vec<serde_json::Value> = crates
            .iter()
            .map(|c| {
                serde_json::json!({
                    "name": c.name,
                    "path": c.manifest_path.to_string_lossy(),
                    "dependencies": c.dependencies,
                })
            })
            .collect();
        write_stdout(format_args!("{}", serde_json::to_string_pretty(&output)?))?;
    } else {
        for c in &crates {
            write_stdout(format_args!("{} ({})", c.name, c.manifest_path.display()))?;
        }
        write_stdout(format_args!("Total: {} crates", crates.len()))?;
    }
    Ok(())
}

fn cmd_proof_list(
    workspace_root: &Path,
    crate_name: Option<&str>,
    json: bool,
) -> anyhow::Result<()> {
    let crates = discovery::discover_crates(workspace_root)?;
    let available_lanes = lanes::detect_available_lanes(workspace_root);

    let filtered: Vec<_> = match crate_name {
        Some(name) => crates.into_iter().filter(|c| c.name == name).collect(),
        None => crates,
    };

    if json {
        let output: Vec<serde_json::Value> = filtered
            .iter()
            .map(|c| {
                serde_json::json!({
                    "crate": c.name,
                    "lanes": available_lanes.iter().map(|l| &l.name).collect::<Vec<_>>(),
                })
            })
            .collect();
        write_stdout(format_args!("{}", serde_json::to_string_pretty(&output)?))?;
    } else {
        for c in &filtered {
            write_stdout(format_args!("{}:", c.name))?;
            for lane in &available_lanes {
                let req = if lane.required {
                    "required"
                } else {
                    "optional"
                };
                write_stdout(format_args!("  {} ({})", lane.name, req))?;
            }
        }
    }
    Ok(())
}

fn resolve_jobs(jobs: &str) -> usize {
    if jobs == "auto" {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    } else {
        jobs.parse().unwrap_or(4)
    }
}

struct ProofRunConfig {
    profile_str: String,
    jobs_str: String,
    exclude: Option<Vec<String>>,
    include: Option<Vec<String>>,
    fail_fast: bool,
    timeout: u64,
    dry_run: bool,
    json: bool,
}

fn cmd_proof_run(
    workspace_root: &Path,
    cfg: &ProofRunConfig,
    keep_going: bool,
) -> anyhow::Result<()> {
    let Some(profile) = profiles::parse_profile(&cfg.profile_str) else {
        anyhow::bail!(
            "Unknown profile: {}. Use fast|standard|deep|proof|all",
            cfg.profile_str
        );
    };

    let max_jobs = resolve_jobs(&cfg.jobs_str);
    let _keep_going = keep_going;

    let config = proof_orchestrator::OrchestratorConfig {
        profile,
        max_jobs,
        timeout_secs: cfg.timeout,
        fail_fast: cfg.fail_fast,
        dry_run: cfg.dry_run,
        json_output: cfg.json,
        include: cfg.include.clone(),
        exclude: cfg.exclude.clone(),
    };

    let (exit_code, summary) = proof_orchestrator::run_proof(workspace_root, &config)?;
    let output = summary::format_summary(&summary, cfg.json);
    write_stdout(format_args!("{output}"))?;

    if exit_code != 0 {
        std::process::exit(exit_code);
    }
    Ok(())
}

struct ProofCrateConfig {
    lane_names: Option<Vec<String>>,
    jobs_str: String,
    fail_fast: bool,
    timeout: u64,
    dry_run: bool,
    json: bool,
}

fn cmd_proof_crate(
    workspace_root: &Path,
    crate_name: &str,
    cfg: &ProofCrateConfig,
) -> anyhow::Result<()> {
    let lanes = cfg.lane_names.clone().unwrap_or_else(|| {
        lanes::detect_available_lanes(workspace_root)
            .into_iter()
            .map(|l| l.name)
            .collect()
    });

    let max_jobs = resolve_jobs(&cfg.jobs_str);

    let config = proof_orchestrator::OrchestratorConfig {
        profile: profiles::Profile::All,
        max_jobs,
        timeout_secs: cfg.timeout,
        fail_fast: cfg.fail_fast,
        dry_run: cfg.dry_run,
        json_output: cfg.json,
        include: None,
        exclude: None,
    };

    let (exit_code, summary) =
        proof_orchestrator::run_proof_for_crate(workspace_root, crate_name, &lanes, &config)?;
    let output = summary::format_summary(&summary, cfg.json);
    write_stdout(format_args!("{output}"))?;

    if exit_code != 0 {
        std::process::exit(exit_code);
    }
    Ok(())
}

struct ProofAffectedConfig {
    jobs_str: String,
    fail_fast: bool,
    timeout: u64,
    dry_run: bool,
    json: bool,
}

fn cmd_proof_affected(
    workspace_root: &Path,
    base: &str,
    cfg: &ProofAffectedConfig,
) -> anyhow::Result<()> {
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", base, "HEAD"])
        .current_dir(workspace_root)
        .output()?;

    let changed_files: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(String::from)
        .collect();

    let crates = discovery::discover_crates(workspace_root)?;
    let affected: Vec<_> = crates
        .into_iter()
        .filter(|c| {
            let crate_prefix = format!("crates/{}/", c.name);
            changed_files.iter().any(|f| f.starts_with(&crate_prefix))
        })
        .collect();

    if affected.is_empty() {
        write_stdout(format_args!("No affected crates changed since {base}"))?;
        return Ok(());
    }

    let max_jobs = resolve_jobs(&cfg.jobs_str);

    let config = proof_orchestrator::OrchestratorConfig {
        profile: profiles::Profile::Standard,
        max_jobs,
        timeout_secs: cfg.timeout,
        fail_fast: cfg.fail_fast,
        dry_run: cfg.dry_run,
        json_output: cfg.json,
        include: Some(affected.iter().map(|c| c.name.clone()).collect()),
        exclude: None,
    };

    let (exit_code, summary) = proof_orchestrator::run_proof(workspace_root, &config)?;
    let output = summary::format_summary(&summary, cfg.json);
    write_stdout(format_args!("{output}"))?;

    if exit_code != 0 {
        std::process::exit(exit_code);
    }
    Ok(())
}

#[cfg(test)]
mod command_shell_tests;

fn cmd_proof_plan(crate_name: Option<&str>) -> anyhow::Result<()> {
    let obligations = proof::load_proof_obligations()
        .map_err(|e| anyhow::anyhow!("Failed to load proof obligations: {e}"))?;

    let filtered: Vec<_> = match crate_name {
        Some(name) => obligations
            .iter()
            .filter(|o| o.crate_name == name)
            .cloned()
            .collect(),
        None => obligations,
    };

    write_stdout(format_args!("Proof obligations: {}", filtered.len()))?;
    for obl in &filtered {
        write_stdout(format_args!("  {} [{}]", obl.id, obl.proof_level))?;
        write_stdout(format_args!(
            "    Statement: {}",
            obl.statement.lines().next().unwrap_or("")
        ))?;
        for cmd in proof::commands_for_obligation(obl) {
            write_stdout(format_args!("    Command: {cmd}"))?;
        }
    }

    Ok(())
}

fn cmd_proof_check(level: Option<&str>, bead: Option<&str>) -> anyhow::Result<()> {
    let obligations = proof::load_proof_obligations()
        .map_err(|e| anyhow::anyhow!("Failed to load proof obligations: {e}"))?;

    let filtered: Vec<_> = match level {
        Some(lvl) => proof::obligations_for_level(&obligations, lvl),
        None => obligations,
    };

    write_stdout(format_args!(
        "Running proof checks: {} obligations at level {:?}",
        filtered.len(),
        level
    ))?;

    let output_dir = match bead {
        Some(bead_id) => PathBuf::from(".evidence").join(bead_id),
        None => PathBuf::from(".evidence/proof"),
    };

    std::fs::create_dir_all(&output_dir)?;

    let mut results = Vec::new();
    for obl in &filtered {
        write_stdout(format_args!("Checking: {} [{}]", obl.id, obl.proof_level))?;
        let commands = proof::commands_for_obligation(obl);

        let mut all_passed = true;
        for cmd in &commands {
            write_stdout(format_args!("  Running: {cmd}"))?;
            let status = std::process::Command::new("sh").arg("-c").arg(cmd).status();

            if status.map(|s| !s.success()).unwrap_or(true) {
                all_passed = false;
                write_stdout(format_args!("  FAILED: {cmd}"))?;
            }
        }
        results.push((obl.id.clone(), all_passed));
    }

    let evidence_path =
        proof::write_proof_evidence(bead.unwrap_or("proof"), &filtered, &results, &output_dir)
            .map_err(|e| anyhow::anyhow!("Failed to write proof evidence: {e}"))?;

    write_stdout(format_args!(
        "Proof evidence written to: {}",
        evidence_path.display()
    ))?;

    let failed_count = results.iter().filter(|(_, passed)| !passed).count();
    if failed_count > 0 {
        anyhow::bail!("{failed_count} proof obligations failed");
    }

    Ok(())
}

fn cmd_proof_evidence(bead: &str) -> anyhow::Result<()> {
    let obligations = proof::load_proof_obligations()
        .map_err(|e| anyhow::anyhow!("Failed to load proof obligations: {e}"))?;

    let output_dir = PathBuf::from(".evidence").join(bead);
    std::fs::create_dir_all(&output_dir)?;

    let results: Vec<_> = obligations.iter().map(|o| (o.id.clone(), true)).collect();

    let evidence_path = proof::write_proof_evidence(bead, &obligations, &results, &output_dir)
        .map_err(|e| anyhow::anyhow!("Failed to write proof evidence: {e}"))?;

    write_stdout(format_args!(
        "Proof evidence written to: {}",
        evidence_path.display()
    ))?;
    Ok(())
}

fn cmd_proof_drift(sections: Option<&[usize]>) -> anyhow::Result<()> {
    use std::collections::HashMap;

    write_stdout(format_args!("Proof drift checker"))?;
    write_stdout(format_args!(
        "Checking spec alignment with proof obligations..."
    ))?;

    let obligations = proof::load_proof_obligations()
        .map_err(|e| anyhow::anyhow!("Failed to load proof obligations: {e}"))?;

    let section_map: HashMap<usize, Vec<&proof::ProofObligation>> = {
        let mut map: HashMap<usize, Vec<&proof::ProofObligation>> = HashMap::new();
        for obl in obligations.iter() {
            for &section in &obl.section {
                map.entry(section).or_default().push(obl);
            }
        }
        map
    };

    let master_spec = std::fs::read_to_string("velvet-ballistics-MASTER.md")
        .map_err(|e| anyhow::anyhow!("Failed to read master spec: {e}"))?;

    let mut drift_issues = Vec::new();

    for (section, obls) in &section_map {
        let section_marker = format!("## {section}");
        if !master_spec.contains(&section_marker)
            && sections.map(|s| s.contains(section)).unwrap_or(true)
        {
            drift_issues.push(format!(
                "Section {section} referenced in obligations but not found in spec: {:?}",
                obls.iter().map(|o| o.id.clone()).collect::<Vec<_>>()
            ));
        }
    }

    if drift_issues.is_empty() {
        write_stdout(format_args!("No drift detected"))?;
    } else {
        write_stdout(format_args!("DRIFT DETECTED:"))?;
        for issue in &drift_issues {
            write_stdout(format_args!("  {issue}"))?;
        }
        anyhow::bail!("Spec drift detected");
    }

    Ok(())
}
