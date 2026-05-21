use anyhow::Context;
use std::path::{Path, PathBuf};

use crate::evidence;
use crate::shell::write_stdout;

pub(crate) fn cmd_ai_fast(bead: Option<&str>) -> anyhow::Result<()> {
    run_ai_profile(evidence::GateProfile::AiFast, bead)
}

pub(crate) fn cmd_ai_deep(bead: Option<&str>) -> anyhow::Result<()> {
    run_ai_profile(evidence::GateProfile::AiDeep, bead)
}

pub(crate) fn cmd_ai_release(bead: Option<&str>) -> anyhow::Result<()> {
    run_ai_profile(evidence::GateProfile::AiRelease, bead)
}

struct AiProfilePlan<'a> {
    profile: evidence::GateProfile,
    bead: Option<&'a str>,
    output_dir: PathBuf,
    write_evidence: bool,
}

fn run_ai_profile(profile: evidence::GateProfile, bead: Option<&str>) -> anyhow::Result<()> {
    let plan = build_ai_profile_plan(profile, bead)?;
    prepare_ai_profile_output(&plan)?;
    let yaml = run_ai_profile_plan(&plan)?;
    write_ai_profile_output(&plan, &yaml)?;
    write_stdout(format_args!("{yaml}"))
}

fn build_ai_profile_plan(
    profile: evidence::GateProfile,
    bead: Option<&str>,
) -> anyhow::Result<AiProfilePlan<'_>> {
    reject_unknown_ai_release_bead(profile, bead)?;
    let output_dir = evidence_output_dir(bead)?;
    Ok(AiProfilePlan {
        profile,
        bead,
        output_dir,
        write_evidence: bead.is_some() && !is_stdout_only_release(profile, bead),
    })
}

fn evidence_output_dir(bead: Option<&str>) -> anyhow::Result<PathBuf> {
    match bead {
        Some(bead_id) => {
            validate_bead_id(bead_id)?;
            Ok(PathBuf::from(".evidence").join(bead_id))
        }
        None => Ok(PathBuf::from(".evidence").join("default")),
    }
}

fn prepare_ai_profile_output(plan: &AiProfilePlan<'_>) -> anyhow::Result<()> {
    if plan.bead.is_none() {
        return Ok(());
    }
    fail_on_partial_profile_evidence(&plan.output_dir, plan.profile)?;
    create_evidence_dir(&plan.output_dir)
}

fn create_evidence_dir(output_dir: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "Failed to create evidence directory: {}",
            output_dir.display()
        )
    })
}

fn run_ai_profile_plan(plan: &AiProfilePlan<'_>) -> anyhow::Result<String> {
    let result = evidence::run_profile(plan.profile, plan.bead, &plan.output_dir)?;
    serde_saphyr::to_string(&result).context("Failed to serialize profile evidence")
}

fn write_ai_profile_output(plan: &AiProfilePlan<'_>, yaml: &str) -> anyhow::Result<()> {
    if !plan.write_evidence {
        return Ok(());
    }
    let path = plan.output_dir.join(plan.profile.evidence_file());
    std::fs::write(&path, yaml)
        .with_context(|| format!("Failed to write profile evidence: {}", path.display()))
}

fn is_stdout_only_release(profile: evidence::GateProfile, bead: Option<&str>) -> bool {
    bead == Some("vb-nf2u") && profile == evidence::GateProfile::AiRelease
}

pub(crate) fn reject_unknown_ai_release_bead(
    profile: evidence::GateProfile,
    bead: Option<&str>,
) -> anyhow::Result<()> {
    if profile == evidence::GateProfile::AiRelease && bead != Some("vb-nf2u") {
        anyhow::bail!(
            "unknown ai-release bead id: {}",
            bead.unwrap_or("<missing>")
        );
    }
    Ok(())
}

pub(crate) fn validate_bead_id(bead_id: &str) -> anyhow::Result<()> {
    let valid = !bead_id.is_empty()
        && bead_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_');
    if valid {
        Ok(())
    } else {
        anyhow::bail!("Invalid bead id: {bead_id}")
    }
}

pub(crate) fn fail_on_partial_profile_evidence(
    output_dir: &Path,
    profile: evidence::GateProfile,
) -> anyhow::Result<()> {
    if !output_dir.exists() {
        return Ok(());
    }
    if !evidence_dir_has_yaml(output_dir)? {
        return Ok(());
    }
    let missing = evidence::validate_evidence_dir(output_dir, profile.gates())?;
    if missing.is_empty() || output_dir.join(profile.evidence_file()).exists() {
        Ok(())
    } else {
        anyhow::bail!("Missing required gate evidence: {:?}", missing)
    }
}

fn evidence_dir_has_yaml(output_dir: &Path) -> anyhow::Result<bool> {
    Ok(std::fs::read_dir(output_dir)
        .with_context(|| {
            format!(
                "Failed to read evidence directory: {}",
                output_dir.display()
            )
        })?
        .filter_map(std::result::Result::ok)
        .any(|entry| entry.path().extension().is_some_and(|ext| ext == "yaml")))
}
