//! Fail-closed audit of proof obligations against scanned verification sources.
//!
//! [`audit_all`] classifies every obligation as production behavior
//! verification, source linkage, or a blocked state, and records one typed
//! [`crate::Finding`] per blocked obligation into the lane report.
//! The audit is pure: obligations, scanned files, evidence records, waivers,
//! and the current production revision are all caller-supplied values, so
//! missing evidence can never read as a pass.

use super::{
    classify::{BindingVerdict, TrustMarker},
    decide::{audit_one, find_evidence, find_file},
    report::{BlockedCtx, push_blocked_finding},
    scan::ScannedFile,
};
use crate::{LaneReport, RuleIdError};

/// One production behavior obligation that a verification source must cover.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Obligation {
    /// Stable obligation identity, e.g. `receipt-schema-version`.
    id: String,
    /// Workspace-relative production file under proof.
    production_path: String,
    /// Production symbol under proof.
    production_symbol: String,
    /// Registered workspace-relative proof file for this obligation.
    proof_file: String,
}

impl Obligation {
    /// Construct an obligation from its identity, production binding, and
    /// registered proof file.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        production_path: impl Into<String>,
        production_symbol: impl Into<String>,
        proof_file: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            production_path: production_path.into(),
            production_symbol: production_symbol.into(),
            proof_file: proof_file.into(),
        }
    }

    /// Stable obligation identity.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Workspace-relative production file under proof.
    #[must_use]
    pub fn production_path(&self) -> &str {
        &self.production_path
    }

    /// Production symbol under proof.
    #[must_use]
    pub fn production_symbol(&self) -> &str {
        &self.production_symbol
    }

    /// Registered workspace-relative proof file for this obligation.
    #[must_use]
    pub fn proof_file(&self) -> &str {
        &self.proof_file
    }
}

/// Recorded raw verifier evidence for one obligation.
///
/// Evidence is a caller-supplied value, never a fresh run: the audit checks
/// that the recorded run passed (`exit_code == 0`) at the current production
/// revision. Anything else fails closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifierEvidence {
    /// Obligation this evidence run claims to cover.
    obligation_id: String,
    /// Verifier tool name, e.g. `verus`.
    tool: String,
    /// Verifier tool version.
    tool_version: String,
    /// Production revision the run executed against.
    revision: String,
    /// Process exit status of the recorded run.
    exit_code: i32,
}

impl VerifierEvidence {
    /// Construct a verifier evidence record.
    #[must_use]
    pub fn new(
        obligation_id: impl Into<String>,
        tool: impl Into<String>,
        tool_version: impl Into<String>,
        revision: impl Into<String>,
        exit_code: i32,
    ) -> Self {
        Self {
            obligation_id: obligation_id.into(),
            tool: tool.into(),
            tool_version: tool_version.into(),
            revision: revision.into(),
            exit_code,
        }
    }

    /// Obligation this evidence run claims to cover.
    #[must_use]
    pub fn obligation_id(&self) -> &str {
        &self.obligation_id
    }

    /// Verifier tool name.
    #[must_use]
    pub fn tool(&self) -> &str {
        &self.tool
    }

    /// Verifier tool version.
    #[must_use]
    pub fn tool_version(&self) -> &str {
        &self.tool_version
    }

    /// Production revision the run executed against.
    #[must_use]
    pub fn revision(&self) -> &str {
        &self.revision
    }

    /// Process exit status of the recorded run.
    #[must_use]
    pub const fn exit_code(&self) -> i32 {
        self.exit_code
    }

    /// Trusted-base label naming tool and version, e.g. `verus 0.0.1-test`.
    #[must_use]
    pub fn trusted_base(&self) -> String {
        [self.tool.clone(), self.tool_version.clone()].join(" ")
    }
}

/// Independent explicit approval for one trust marker on one obligation.
///
/// A waiver is explicit only when both `approver` and `reason` are
/// non-empty; hollow waivers cover nothing. A waiver never promotes source
/// linkage to production behavior verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Waiver {
    /// Obligation the waiver applies to.
    obligation_id: String,
    /// Trust marker the waiver covers.
    marker: TrustMarker,
    /// Independent approver identity (review board or owner).
    approver: String,
    /// Reason or ticket reference for the approval.
    reason: String,
}

impl Waiver {
    /// Construct a waiver for one marker on one obligation.
    #[must_use]
    pub fn new(
        obligation_id: impl Into<String>,
        marker: TrustMarker,
        approver: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            obligation_id: obligation_id.into(),
            marker,
            approver: approver.into(),
            reason: reason.into(),
        }
    }

    /// Obligation the waiver applies to.
    #[must_use]
    pub fn obligation_id(&self) -> &str {
        &self.obligation_id
    }

    /// Trust marker the waiver covers.
    #[must_use]
    pub const fn marker(&self) -> TrustMarker {
        self.marker
    }

    /// `true` only when approver and reason are both non-empty.
    #[must_use]
    pub fn is_explicit(&self) -> bool {
        !self.approver.is_empty() && !self.reason.is_empty()
    }
}

/// Borrowed audit inputs: obligations, scanned files, evidence records,
/// waivers, and the current production revision for drift comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuditInputs<'a> {
    /// Proof obligations to classify.
    pub obligations: &'a [Obligation],
    /// Scanned verification sources, matched by workspace-relative path.
    pub files: &'a [ScannedFile],
    /// Recorded verifier evidence, matched by obligation id.
    pub evidences: &'a [VerifierEvidence],
    /// Independent explicit waivers for trust markers.
    pub waivers: &'a [Waiver],
    /// Current production revision; evidence at any other revision is drift.
    pub current_revision: &'a str,
}

/// Audit outcome for one obligation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditResult {
    /// Stable obligation identity.
    obligation_id: String,
    /// Classification of the obligation.
    verdict: BindingVerdict,
}

impl AuditResult {
    /// Construct an audit result.
    #[must_use]
    pub fn new(obligation_id: impl Into<String>, verdict: BindingVerdict) -> Self {
        Self { obligation_id: obligation_id.into(), verdict }
    }

    /// Stable obligation identity.
    #[must_use]
    pub fn obligation_id(&self) -> &str {
        &self.obligation_id
    }

    /// Classification of the obligation.
    #[must_use]
    pub const fn verdict(&self) -> &BindingVerdict {
        &self.verdict
    }
}

/// Audit every obligation, push one finding per blocked obligation into
/// `report`, and return the per-obligation verdicts.
///
/// Pass states (`VerifiedProduction`, `SourceLinked`) record no findings.
///
/// # Errors
///
/// Returns [`RuleIdError`] when an embedded lane rule identifier is invalid.
pub fn audit_all(
    inputs: &AuditInputs<'_>,
    report: &mut LaneReport,
) -> Result<Vec<AuditResult>, RuleIdError> {
    let mut results: Vec<AuditResult> = Vec::with_capacity(inputs.obligations.len());
    for obligation in inputs.obligations {
        results.push(audit_single(inputs, obligation, report)?);
    }
    Ok(results)
}

/// Audit one obligation and record its blocked finding, if any.
///
/// # Errors
///
/// Returns [`RuleIdError`] when the embedded lane rule identifier is invalid.
fn audit_single(
    inputs: &AuditInputs<'_>,
    obligation: &Obligation,
    report: &mut LaneReport,
) -> Result<AuditResult, RuleIdError> {
    let file: Option<&ScannedFile> = find_file(inputs.files, obligation.proof_file());
    let evidence: Option<&VerifierEvidence> = find_evidence(inputs.evidences, obligation.id());
    let verdict: BindingVerdict =
        audit_one(obligation, file, evidence, inputs.waivers, inputs.current_revision);
    if let BindingVerdict::Blocked { reason } = &verdict {
        let ctx: BlockedCtx<'_> =
            BlockedCtx { obligation, file, reason: *reason, evidence, waivers: inputs.waivers };
        push_blocked_finding(&ctx, report)?;
    }
    Ok(AuditResult::new(obligation.id(), verdict))
}
