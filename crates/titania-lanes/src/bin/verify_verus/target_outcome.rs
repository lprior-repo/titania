use std::path::Path;

use titania_core::TargetProject;
use titania_lanes::{Finding, LaneExit, LaneReport, RuleId};

use super::{
    diagnostics::{stderr_write_failure, write_stderr_line},
    registry::ProofTarget,
    verus_tool,
};

/// Run all registered proof targets and collect target failure messages.
///
/// # Errors
///
/// Returns failure when a target failure cannot be recorded.
pub(crate) fn collect_target_failures(
    target: &TargetProject,
    targets: &[ProofTarget],
    evidence_dir: &Path,
    report: &mut LaneReport,
    target_rule: &RuleId,
) -> Result<Vec<String>, LaneExit> {
    targets.iter().try_fold(Vec::new(), |mut failures, proof_target| {
        failures.extend(verus_target_failure(
            target,
            proof_target,
            evidence_dir,
            report,
            target_rule,
        )?);
        Ok(failures)
    })
}

/// Run one Verus proof target and return its failure message if it fails.
///
/// # Errors
///
/// Returns failure when recording a failed target cannot write diagnostics.
fn verus_target_failure(
    target: &TargetProject,
    proof_target: &ProofTarget,
    evidence_dir: &Path,
    report: &mut LaneReport,
    target_rule: &RuleId,
) -> Result<Option<String>, LaneExit> {
    match verus_tool::run_verus_target(target, proof_target, evidence_dir) {
        Ok(()) => Ok(None),
        Err(e) => {
            record_target_failure(report, proof_target, &e, target_rule)?;
            Ok(Some(format!("{}: {e}", proof_target.path())))
        }
    }
}

/// Record one failed proof target in stderr and the lane report.
///
/// # Errors
///
/// Returns failure when stderr writing fails.
fn record_target_failure(
    report: &mut LaneReport,
    proof_target: &ProofTarget,
    error: &str,
    target_rule: &RuleId,
) -> Result<(), LaneExit> {
    write_stderr_line(format_args!(
        "[verify-verus] target {} failed: {error}",
        proof_target.path()
    ))
    .map_err(stderr_write_failure)?;
    report.push(Finding::new(
        target_rule.clone(),
        proof_target.path().to_owned(),
        0,
        error.to_owned(),
    ));
    Ok(())
}
