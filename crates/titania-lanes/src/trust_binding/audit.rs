use super::{
    classify::{BindingVerdict, TrustMarker},
    decide::{audit_one, find_evidence, find_file},
    report::{BlockedCtx, push_blocked_finding},
    scan::ScannedFile,
};
use crate::{LaneReport, RuleIdError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Obligation {
    id: String,
    production_path: String,
    production_symbol: String,
    proof_file: String,
}

impl Obligation {
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

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn production_path(&self) -> &str {
        &self.production_path
    }

    #[must_use]
    pub fn production_symbol(&self) -> &str {
        &self.production_symbol
    }

    #[must_use]
    pub fn proof_file(&self) -> &str {
        &self.proof_file
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifierEvidence {
    obligation_id: String,
    tool: String,
    tool_version: String,
    revision: String,
    exit_code: i32,
}

impl VerifierEvidence {
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

    #[must_use]
    pub fn obligation_id(&self) -> &str {
        &self.obligation_id
    }

    #[must_use]
    pub fn tool(&self) -> &str {
        &self.tool
    }

    #[must_use]
    pub fn tool_version(&self) -> &str {
        &self.tool_version
    }

    #[must_use]
    pub fn revision(&self) -> &str {
        &self.revision
    }

    #[must_use]
    pub const fn exit_code(&self) -> i32 {
        self.exit_code
    }

    #[must_use]
    pub fn trusted_base(&self) -> String {
        [self.tool.clone(), self.tool_version.clone()].join(" ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Waiver {
    obligation_id: String,
    marker: TrustMarker,
    approver: String,
    reason: String,
}

impl Waiver {
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

    #[must_use]
    pub fn obligation_id(&self) -> &str {
        &self.obligation_id
    }

    #[must_use]
    pub const fn marker(&self) -> TrustMarker {
        self.marker
    }

    #[must_use]
    pub fn is_explicit(&self) -> bool {
        !self.approver.is_empty() && !self.reason.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuditInputs<'a> {
    pub obligations: &'a [Obligation],
    pub files: &'a [ScannedFile],
    pub evidences: &'a [VerifierEvidence],
    pub waivers: &'a [Waiver],
    pub current_revision: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditResult {
    obligation_id: String,
    verdict: BindingVerdict,
}

impl AuditResult {
    #[must_use]
    pub fn new(obligation_id: impl Into<String>, verdict: BindingVerdict) -> Self {
        Self { obligation_id: obligation_id.into(), verdict }
    }

    #[must_use]
    pub fn obligation_id(&self) -> &str {
        &self.obligation_id
    }

    #[must_use]
    pub const fn verdict(&self) -> &BindingVerdict {
        &self.verdict
    }
}

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
