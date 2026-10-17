use super::{
    PerLaneEntry, QualityReceipt, RejectKind, Report, ReportInner, ReportKind,
    validators::{
        check_per_lane_not_empty, check_reject_not_empty, reject_kind_for, validate_per_lane_pass,
        validate_per_lane_pass_scope, validate_per_lane_reject,
    },
};
use crate::{
    diagnostic::{InputDiagnostic, PolicyDiagnostic},
    error::ReportError,
    failure::LaneFailure,
    finding::Finding,
    gate_scope::GateScope,
};

impl Report {
    pub fn reject(
        scope: crate::GateScope,
        code_findings: Box<[Finding]>,
        gate_failures: Box<[LaneFailure]>,
        per_lane: Box<[PerLaneEntry]>,
    ) -> Result<Self, ReportError> {
        check_reject_not_empty(&code_findings, &gate_failures)?;
        validate_per_lane_reject(scope, &per_lane)?;
        Ok(Self(ReportInner::Reject { code_findings, gate_failures, per_lane }))
    }

    pub fn pass(
        receipt: QualityReceipt,
        per_lane: Box<[PerLaneEntry]>,
    ) -> Result<Self, ReportError> {
        let scope: GateScope = *receipt.scope();
        check_per_lane_not_empty(&per_lane)?;
        validate_per_lane_pass_scope(scope, &per_lane)?;
        validate_per_lane_pass(&per_lane)?;
        Ok(Self(ReportInner::Pass { receipt, per_lane }))
    }

    #[must_use]
    pub const fn policy_error(diagnostics: Box<[PolicyDiagnostic]>) -> Self {
        Self(ReportInner::PolicyError { diagnostics })
    }

    #[must_use]
    pub const fn input_error(diagnostics: Box<[InputDiagnostic]>) -> Self {
        Self(ReportInner::InputError { diagnostics })
    }

    #[must_use]
    pub fn reject_kind(&self) -> Option<RejectKind> {
        match &self.0 {
            ReportInner::Reject { code_findings: c, gate_failures: g, .. } => reject_kind_for(c, g),
            _ => None,
        }
    }

    #[must_use]
    pub const fn is_pass(&self) -> bool {
        matches!(&self.0, ReportInner::Pass { .. })
    }

    #[must_use]
    pub const fn is_reject(&self) -> bool {
        matches!(&self.0, ReportInner::Reject { .. })
    }

    #[must_use]
    pub const fn is_policy_error(&self) -> bool {
        matches!(&self.0, ReportInner::PolicyError { .. })
    }

    #[must_use]
    pub const fn is_input_error(&self) -> bool {
        matches!(&self.0, ReportInner::InputError { .. })
    }

    #[must_use]
    pub fn code_findings(&self) -> Option<&[Finding]> {
        match &self.0 {
            ReportInner::Reject { code_findings, .. } => Some(code_findings),
            _ => None,
        }
    }

    #[must_use]
    pub fn gate_failures(&self) -> Option<&[LaneFailure]> {
        match &self.0 {
            ReportInner::Reject { gate_failures, .. } => Some(gate_failures),
            _ => None,
        }
    }

    #[must_use]
    pub const fn receipt(&self) -> Option<&QualityReceipt> {
        match &self.0 {
            ReportInner::Pass { receipt, .. } => Some(receipt),
            _ => None,
        }
    }

    #[must_use]
    pub fn per_lane(&self) -> Option<&[PerLaneEntry]> {
        Some(match &self.0 {
            ReportInner::Pass { per_lane, .. } | ReportInner::Reject { per_lane, .. } => per_lane,
            _ => return None,
        })
    }

    #[must_use]
    pub fn policy_diagnostics(&self) -> Option<&[PolicyDiagnostic]> {
        match &self.0 {
            ReportInner::PolicyError { diagnostics } => Some(diagnostics),
            _ => None,
        }
    }

    #[must_use]
    pub fn input_diagnostics(&self) -> Option<&[InputDiagnostic]> {
        match &self.0 {
            ReportInner::InputError { diagnostics } => Some(diagnostics),
            _ => None,
        }
    }

    #[must_use]
    pub const fn kind(&self) -> ReportKind {
        match &self.0 {
            ReportInner::Pass { .. } => ReportKind::Pass,
            ReportInner::Reject { .. } => ReportKind::Reject,
            ReportInner::PolicyError { .. } => ReportKind::PolicyError,
            ReportInner::InputError { .. } => ReportKind::InputError,
        }
    }
}
