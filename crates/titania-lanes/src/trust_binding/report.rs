//! Blocked-obligation findings: rendering the audit verdict into lane output.
//!
//! Every [`BlockReason`] maps to exactly one typed [`Finding`] with a stable
//! `TRUST_*` rule id. Pass states record nothing.

use super::{
    audit::{Obligation, VerifierEvidence, Waiver},
    classify::{BlockReason, TrustMarker},
    decide::uncovered_markers,
    scan::ScannedFile,
};
use crate::{Finding, LaneReport, RuleId, RuleIdError};

/// Bundled inputs for emitting one blocked-obligation finding.
#[derive(Debug, Clone, Copy)]
pub(crate) struct BlockedCtx<'a> {
    /// Obligation that failed closed.
    pub(crate) obligation: &'a Obligation,
    /// Registered proof file, when present.
    pub(crate) file: Option<&'a ScannedFile>,
    /// Machine-readable block reason.
    pub(crate) reason: BlockReason,
    /// Recorded verifier evidence, when present.
    pub(crate) evidence: Option<&'a VerifierEvidence>,
    /// Independent explicit waivers.
    pub(crate) waivers: &'a [Waiver],
}

/// Push the lane finding for a blocked verdict; pass verdicts are silent.
///
/// # Errors
///
/// Returns [`RuleIdError`] when the embedded lane rule identifier is invalid.
pub(crate) fn push_blocked_finding(
    ctx: &BlockedCtx<'_>,
    report: &mut LaneReport,
) -> Result<(), RuleIdError> {
    let Some(finding) =
        finding_for_blocked(ctx.obligation, ctx.file, ctx.reason, ctx.evidence, ctx.waivers)?
    else {
        return Ok(());
    };
    report.push(finding);
    Ok(())
}

/// Build the lane finding for a blocked obligation.
///
/// Returns `Ok(None)` only for non-blocked reasons, which never happens for
/// [`BlockReason`] inputs; every block reason maps to exactly one finding.
///
/// # Errors
///
/// Returns [`RuleIdError`] when the embedded lane rule identifier is invalid.
pub fn finding_for_blocked(
    obligation: &Obligation,
    file: Option<&ScannedFile>,
    reason: BlockReason,
    evidence: Option<&VerifierEvidence>,
    waivers: &[Waiver],
) -> Result<Option<Finding>, RuleIdError> {
    let path: &str = file.map_or_else(|| obligation.proof_file(), ScannedFile::path);
    let message: String = blocked_message(obligation, file, reason, evidence, waivers);
    let rule: RuleId = RuleId::new(reason.rule_id())?;
    Ok(Some(Finding::new(rule, path, 0, message)))
}

/// Render the human-readable finding message for a blocked obligation.
fn blocked_message(
    obligation: &Obligation,
    file: Option<&ScannedFile>,
    reason: BlockReason,
    evidence: Option<&VerifierEvidence>,
    waivers: &[Waiver],
) -> String {
    match reason {
        BlockReason::SmokeOnly => smoke_message(obligation, file),
        BlockReason::UnapprovedTrust => trust_message(obligation, file, waivers),
        BlockReason::MissingBinding => binding_message(obligation),
        BlockReason::MirrorDrift => drift_message(obligation, evidence),
        BlockReason::VacuousProof => vacuous_message(obligation, file),
        BlockReason::MissingEvidence => evidence_message(obligation, evidence),
    }
}

/// Render the smoke-fixture finding message.
fn smoke_message(obligation: &Obligation, file: Option<&ScannedFile>) -> String {
    let path: &str = file.map_or_else(|| obligation.proof_file(), ScannedFile::path);
    ["smoke fixture '", path, "' cannot satisfy production obligation '", obligation.id(), "'"]
        .concat()
}

/// Render the unapproved-trust finding message, naming every uncovered
/// marker with its first line.
fn trust_message(
    obligation: &Obligation,
    file: Option<&ScannedFile>,
    waivers: &[Waiver],
) -> String {
    let Some(current) = file else {
        return [
            "obligation '",
            obligation.id(),
            "' carries unapproved trust markers with no proof file",
        ]
        .concat();
    };
    let markers: Vec<TrustMarker> = uncovered_markers(current, obligation.id(), waivers);
    let detail: Vec<String> =
        markers.iter().map(|marker| marker_detail(current, *marker)).collect();
    [
        "proof file '",
        current.path(),
        "' carries unapproved trust markers for obligation '",
        obligation.id(),
        "': ",
        &detail.join(", "),
    ]
    .concat()
}

/// Render one `name@first-line` marker detail entry.
fn marker_detail(file: &ScannedFile, marker: TrustMarker) -> String {
    let first: u32 = file.signals().marker_lines(marker).first().map_or(0, |line| *line);
    [marker.name(), "@", &first.to_string()].concat()
}

/// Render the missing-binding finding message.
fn binding_message(obligation: &Obligation) -> String {
    [
        "obligation '",
        obligation.id(),
        "' has no #[path] binding to '",
        obligation.production_path(),
        "' and no production-symbol reference",
    ]
    .concat()
}

/// Render the mirror-drift finding message.
fn drift_message(obligation: &Obligation, evidence: Option<&VerifierEvidence>) -> String {
    let recorded: &str = evidence.map_or("<none>", VerifierEvidence::revision);
    [
        "recorded evidence revision '",
        recorded,
        "' does not match current production revision for obligation '",
        obligation.id(),
        "'; unchanged mirror content cannot prove a moved body",
    ]
    .concat()
}

/// Render the vacuous-proof finding message.
fn vacuous_message(obligation: &Obligation, file: Option<&ScannedFile>) -> String {
    let path: &str = file.map_or_else(|| obligation.proof_file(), ScannedFile::path);
    [
        "proof file '",
        path,
        "' never references production symbol '",
        obligation.production_symbol(),
        "' for obligation '",
        obligation.id(),
        "'; a literal theorem discharges nothing",
    ]
    .concat()
}

/// Render the missing-evidence finding message.
fn evidence_message(obligation: &Obligation, evidence: Option<&VerifierEvidence>) -> String {
    let status: String =
        evidence.map_or_else(|| "<absent>".to_owned(), |record| record.exit_code().to_string());
    [
        "verifier run for obligation '",
        obligation.id(),
        "' exited with status ",
        &status,
        "; a failed run proves nothing",
    ]
    .concat()
}
