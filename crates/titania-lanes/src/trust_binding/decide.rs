//! Obligation classification: the decision procedure of the trust/binding audit.
//!
//! [`audit_one`](crate::trust_binding::audit::audit_all) applies the contract
//! decision table in order: smoke, unapproved trust, binding presence,
//! vacuity, then evidence. Each step fails closed.

use super::{
    audit::{Obligation, VerifierEvidence, Waiver},
    classify::{BindingVerdict, BlockReason, TrustMarker},
    scan::ScannedFile,
};

/// Classify one obligation against its scanned file and evidence record.
pub(crate) fn audit_one(
    obligation: &Obligation,
    file: Option<&ScannedFile>,
    evidence: Option<&VerifierEvidence>,
    waivers: &[Waiver],
    current_revision: &str,
) -> BindingVerdict {
    let Some(current) = file else {
        return BindingVerdict::Blocked { reason: BlockReason::MissingBinding };
    };
    if current.signals().is_smoke() {
        return BindingVerdict::Blocked { reason: BlockReason::SmokeOnly };
    }
    if !uncovered_markers(current, obligation.id(), waivers).is_empty() {
        return BindingVerdict::Blocked { reason: BlockReason::UnapprovedTrust };
    }
    if current.signals().has_path_attr()
        && !current.signals().path_binds_production(obligation.production_path())
    {
        return BindingVerdict::Blocked { reason: BlockReason::MissingBinding };
    }
    if !current.signals().has_path_attr() && current.signals().production_ref_count() == 0 {
        return BindingVerdict::Blocked { reason: BlockReason::MissingBinding };
    }
    if current.signals().production_ref_count() == 0 {
        return BindingVerdict::Blocked { reason: BlockReason::VacuousProof };
    }
    verdict_for_evidence(current, obligation, evidence, waivers, current_revision)
}

/// Classify the evidence stage: source linkage without passing evidence,
/// verified production at the current revision, or a closed failure.
fn verdict_for_evidence(
    file: &ScannedFile,
    obligation: &Obligation,
    evidence: Option<&VerifierEvidence>,
    waivers: &[Waiver],
    current_revision: &str,
) -> BindingVerdict {
    let Some(record) = evidence else {
        return BindingVerdict::SourceLinked {
            waived_markers: covered_markers(file, obligation.id(), waivers),
        };
    };
    if record.exit_code() != 0 {
        return BindingVerdict::Blocked { reason: BlockReason::MissingEvidence };
    }
    if record.revision() != current_revision {
        return BindingVerdict::Blocked { reason: BlockReason::MirrorDrift };
    }
    BindingVerdict::VerifiedProduction {
        binding: file.path().to_owned(),
        trusted_base: record.trusted_base(),
    }
}

/// Trust markers present in `file` that no explicit waiver covers for
/// `obligation_id`.
pub(crate) fn uncovered_markers(
    file: &ScannedFile,
    obligation_id: &str,
    waivers: &[Waiver],
) -> Vec<TrustMarker> {
    TrustMarker::all()
        .iter()
        .copied()
        .filter(|marker| !file.signals().marker_lines(*marker).is_empty())
        .filter(|marker| !is_waived(waivers, obligation_id, *marker))
        .collect()
}

/// Trust markers present in `file` that an explicit waiver covers for
/// `obligation_id`. Waived markers stay reported as source linkage, never
/// as production behavior verification.
fn covered_markers(
    file: &ScannedFile,
    obligation_id: &str,
    waivers: &[Waiver],
) -> Vec<TrustMarker> {
    TrustMarker::all()
        .iter()
        .copied()
        .filter(|marker| !file.signals().marker_lines(*marker).is_empty())
        .filter(|marker| is_waived(waivers, obligation_id, *marker))
        .collect()
}

/// `true` when an explicit waiver covers `marker` for `obligation_id`.
fn is_waived(waivers: &[Waiver], obligation_id: &str, marker: TrustMarker) -> bool {
    waivers.iter().any(|waiver| {
        waiver.obligation_id() == obligation_id && waiver.marker() == marker && waiver.is_explicit()
    })
}

/// Find the scanned file registered for `proof_file`, if present.
pub(crate) fn find_file<'a>(files: &'a [ScannedFile], proof_file: &str) -> Option<&'a ScannedFile> {
    files.iter().find(|file| file.path() == proof_file)
}

/// Find the evidence record claiming `obligation_id`, if present.
pub(crate) fn find_evidence<'a>(
    evidences: &'a [VerifierEvidence],
    obligation_id: &str,
) -> Option<&'a VerifierEvidence> {
    evidences.iter().find(|record| record.obligation_id() == obligation_id)
}
