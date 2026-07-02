use std::{fs, path::Path};

use titania_core::TargetProject;
use titania_lanes::{LaneExit, LaneReport, RuleId};

use super::{
    VerificationInputs,
    diagnostics::{rule_error_exit, stderr_write_failure, write_stderr_line},
    evidence, target_outcome, trust,
};

const TRUST_FILE: &str = "trust-scan.txt";
const FORBIDDEN_FILE: &str = "trust-forbidden.txt";
const RULE_VERUS_TARGET: &str = "VERUS_TARGET_001";

struct VerusRules {
    trust: trust::TrustRules,
    target: RuleId,
}

struct ScanResult {
    target_failures: Vec<String>,
    forbidden_count: usize,
    external_markers: Vec<String>,
    waiver_exists: bool,
}

pub(crate) fn run_production_targets(
    report: &mut LaneReport,
    target: &TargetProject,
    evidence_dir: &Path,
    inputs: &VerificationInputs,
) -> LaneExit {
    let rules = match load_rules() {
        Ok(rules) => rules,
        Err(exit_code) => return exit_code,
    };
    let scan = match collect_scan_result(target, evidence_dir, inputs, report, &rules) {
        Ok(scan) => scan,
        Err(exit_code) => return exit_code,
    };
    if let Err(exit_code) = finalize_scan(report, evidence_dir, inputs, &rules, &scan) {
        return exit_code;
    }
    if report.is_clean() { LaneExit::Clean } else { LaneExit::Violations }
}

/// Load rule identifiers used by the production Verus run.
///
/// # Errors
///
/// Returns failure when any configured rule id is invalid or its diagnostic
/// cannot be written.
fn load_rules() -> Result<VerusRules, LaneExit> {
    Ok(VerusRules { trust: load_trust_rules()?, target: load_target_rule()? })
}

/// Load trust-boundary rule identifiers.
///
/// # Errors
///
/// Returns failure when a trust rule id is invalid.
fn load_trust_rules() -> Result<trust::TrustRules, LaneExit> {
    trust::TrustRules::new().map_err(|error| rule_error_exit(&error.to_string()))
}

/// Load the Verus-target failure rule identifier.
///
/// # Errors
///
/// Returns failure when the rule id is invalid.
fn load_target_rule() -> Result<RuleId, LaneExit> {
    RuleId::new(RULE_VERUS_TARGET).map_err(|error| rule_error_exit(&error.to_string()))
}

/// Run target and trust scans, returning a compact scan summary.
///
/// # Errors
///
/// Returns failure when target execution or evidence writing fails.
fn collect_scan_result(
    target: &TargetProject,
    evidence_dir: &Path,
    inputs: &VerificationInputs,
    report: &mut LaneReport,
    rules: &VerusRules,
) -> Result<ScanResult, LaneExit> {
    let target_failures = target_outcome::collect_target_failures(
        target,
        &inputs.targets,
        evidence_dir,
        report,
        &rules.target,
    )?;
    let forbidden = collect_forbidden_trust(target, evidence_dir, report, &rules.trust.forbidden)?;
    let external_markers = collect_external_markers(evidence_dir, target)?;
    Ok(ScanResult {
        target_failures,
        forbidden_count: forbidden.len(),
        external_markers,
        waiver_exists: trust::trusted_base_waiver_exists(evidence_dir),
    })
}

/// Emit final evidence and unwaived-marker findings.
///
/// # Errors
///
/// Returns failure when evidence or diagnostics cannot be written.
fn finalize_scan(
    report: &mut LaneReport,
    evidence_dir: &Path,
    inputs: &VerificationInputs,
    rules: &VerusRules,
    scan: &ScanResult,
) -> Result<(), LaneExit> {
    handle_external_markers(
        report,
        &scan.external_markers,
        scan.waiver_exists,
        evidence_dir,
        &rules.trust.external,
    )?;
    append_final_summary(
        &inputs.summary_path,
        &scan.target_failures,
        scan.forbidden_count,
        &scan.external_markers,
        scan.waiver_exists,
    )
}

/// Scan for forbidden Verus trust markers and write evidence when present.
///
/// # Errors
///
/// Returns failure when the forbidden-trust evidence file or diagnostic cannot
/// be written.
fn collect_forbidden_trust(
    target: &TargetProject,
    evidence_dir: &Path,
    report: &mut LaneReport,
    forbidden_rule: &RuleId,
) -> Result<Vec<String>, LaneExit> {
    let forbidden = trust::scan_forbidden_trust(target, forbidden_rule, report);
    if !forbidden.is_empty() {
        emit_forbidden_trust_file(evidence_dir, &forbidden)?;
    }
    Ok(forbidden)
}

/// Write forbidden-trust evidence and emit its location.
///
/// # Errors
///
/// Returns failure when the evidence file or stderr diagnostic cannot be
/// written.
fn emit_forbidden_trust_file(evidence_dir: &Path, forbidden: &[String]) -> Result<(), LaneExit> {
    let path = evidence_dir.join(FORBIDDEN_FILE);
    if let Err(e) = fs::write(&path, forbidden.join("\n")) {
        write_stderr_line(format_args!(
            "[verify-verus] cannot write forbidden-trust file {}: {e}",
            path.display()
        ))
        .map_err(stderr_write_failure)?;
    }
    write_stderr_line(format_args!(
        "[verify-verus] forbidden trust markers found; see {}",
        path.display()
    ))
    .map_err(stderr_write_failure)
}

/// Scan and persist external Verus marker inventory.
///
/// # Errors
///
/// Returns failure when the inventory file or diagnostic cannot be written.
fn collect_external_markers(
    evidence_dir: &Path,
    target: &TargetProject,
) -> Result<Vec<String>, LaneExit> {
    let external_markers = trust::scan_external_markers(target);
    if let Err(e) =
        evidence::write_external_marker_inventory(evidence_dir, TRUST_FILE, &external_markers)
    {
        write_stderr_line(format_args!("[verify-verus] external-marker inventory failed: {e}"))
            .map_err(stderr_write_failure)?;
    }
    Ok(external_markers)
}

/// Report unwaived external markers.
///
/// # Errors
///
/// Returns failure when the waiver diagnostic cannot be written.
fn handle_external_markers(
    report: &mut LaneReport,
    external_markers: &[String],
    waiver_exists: bool,
    evidence_dir: &Path,
    external_rule: &RuleId,
) -> Result<(), LaneExit> {
    if external_markers.is_empty() || waiver_exists {
        return Ok(());
    }
    write_stderr_line(format_args!(
        "[verify-verus] external Verus markers require trusted-base waiver artifact {}; see {}",
        evidence_dir.join(trust::TRUSTED_BASE_WAIVER_FILE).display(),
        evidence_dir.join(TRUST_FILE).display()
    ))
    .map_err(stderr_write_failure)?;
    trust::report_unwaived_external_markers(report, external_markers, external_rule);
    Ok(())
}

/// Append final status lines to the Verus summary.
///
/// # Errors
///
/// Returns failure when the summary or diagnostic cannot be written.
fn append_final_summary(
    summary_path: &Path,
    target_failures: &[String],
    forbidden_count: usize,
    external_markers: &[String],
    waiver_exists: bool,
) -> Result<(), LaneExit> {
    let status = evidence::SummaryStatus {
        target_failures,
        forbidden_count,
        external_marker_count: external_markers.len(),
        external_markers_waived: waiver_exists,
    };
    if let Err(e) = evidence::append_summary_status(summary_path, &status) {
        write_stderr_line(format_args!("[verify-verus] cannot append summary: {e}"))
            .map_err(stderr_write_failure)?;
    }
    Ok(())
}
