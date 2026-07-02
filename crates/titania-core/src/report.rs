//! The aggregated report from a titania-check run.
//!
//! A `Report` is the final output: either a pass (with a receipt), a reject
//! (with findings and failures), or an error (policy or input diagnostics).

use serde::{Deserialize, Serialize};

use crate::{
    diagnostic::{InputDiagnostic, PolicyDiagnostic},
    error::ReportError,
    failure::LaneFailure,
    finding::Finding,
    outcome::LaneOutcome,
    v1_receipt::QualityReceiptV1 as QualityReceipt,
};

/// Classification of a [`Report::Reject`] by which collections are populated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectKind {
    /// Only `code_findings` is non-empty; `gate_failures` is empty.
    CodeOnly,
    /// Only `gate_failures` is non-empty; `code_findings` is empty.
    GateOnly,
    /// Both collections are non-empty.
    Mixed,
}

/// Aggregated report from a titania-check run.
///
/// A `Report` is the final output: either a pass (with a receipt), a reject
/// (with findings and failures), or an error (policy or input diagnostics).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "variant", rename_all = "snake_case")]
pub enum Report {
    /// All lanes passed.
    Pass { receipt: QualityReceipt, per_lane: Box<[LaneOutcome]> },
    /// One or more lanes rejected or failed.
    ///
    /// INVARIANT: at least one of `code_findings` or `gate_failures` is
    /// non-empty. A `Reject` with both empty is a bug — should be `Pass`.
    Reject {
        code_findings: Box<[Finding]>,
        gate_failures: Box<[LaneFailure]>,
        per_lane: Box<[LaneOutcome]>,
    },
    /// Policy configuration error.
    PolicyError { diagnostics: Box<[PolicyDiagnostic]> },
    /// Input or argument error.
    InputError { diagnostics: Box<[InputDiagnostic]> },
}

impl Report {
    /// Create a `Report::Reject`, validating the invariant.
    ///
    /// # Errors
    /// - [`ReportError::EmptyReject`] if both `code_findings` and
    ///   `gate_failures` are empty.
    pub fn reject(
        code_findings: Box<[Finding]>,
        gate_failures: Box<[LaneFailure]>,
        per_lane: Box<[LaneOutcome]>,
    ) -> Result<Self, ReportError> {
        if code_findings.is_empty() && gate_failures.is_empty() {
            return Err(ReportError::EmptyReject);
        }
        Ok(Self::Reject { code_findings, gate_failures, per_lane })
    }
    /// Create a `Report::Pass`.
    ///
    /// # Errors
    /// - [`ReportError::EmptyPerLane`] if `per_lane` is empty.
    pub fn pass(
        receipt: QualityReceipt,
        per_lane: Box<[LaneOutcome]>,
    ) -> Result<Self, ReportError> {
        if per_lane.is_empty() {
            return Err(ReportError::EmptyPerLane);
        }
        Ok(Self::Pass { receipt, per_lane })
    }
    /// Create a `Report::PolicyError`.
    #[must_use]
    pub const fn policy_error(diagnostics: Box<[PolicyDiagnostic]>) -> Self {
        Self::PolicyError { diagnostics }
    }

    /// Create a `Report::InputError`.
    #[must_use]
    pub const fn input_error(diagnostics: Box<[InputDiagnostic]>) -> Self {
        Self::InputError { diagnostics }
    }

    /// Classify the reject kind, if this report is a reject.
    ///
    /// Returns `None` for non-reject reports or if both collections are
    /// empty (invariant violation).
    #[must_use]
    pub fn reject_kind(&self) -> Option<RejectKind> {
        match self {
            Self::Reject { code_findings, gate_failures, .. } => {
                match (code_findings.is_empty(), gate_failures.is_empty()) {
                    (false, true) => Some(RejectKind::CodeOnly),
                    (true, false) => Some(RejectKind::GateOnly),
                    (false, false) => Some(RejectKind::Mixed),
                    (true, true) => None, // invariant violation
                }
            }
            _ => None,
        }
    }

    /// Whether this report represents a pass.
    #[must_use]
    pub const fn is_pass(&self) -> bool {
        matches!(self, Self::Pass { .. })
    }

    /// Whether this report represents a reject.
    #[must_use]
    pub const fn is_reject(&self) -> bool {
        matches!(self, Self::Reject { .. })
    }

    /// Whether this report is a policy error.
    #[must_use]
    pub const fn is_policy_error(&self) -> bool {
        matches!(self, Self::PolicyError { .. })
    }

    /// Whether this report is an input error.
    #[must_use]
    pub const fn is_input_error(&self) -> bool {
        matches!(self, Self::InputError { .. })
    }

    /// If this is a reject, return the code findings.
    #[must_use]
    pub fn code_findings(&self) -> Option<&[Finding]> {
        match self {
            Self::Reject { code_findings, .. } => Some(code_findings),
            _ => None,
        }
    }

    /// If this is a reject, return the gate failures.
    #[must_use]
    pub fn gate_failures(&self) -> Option<&[LaneFailure]> {
        match self {
            Self::Reject { gate_failures, .. } => Some(gate_failures),
            _ => None,
        }
    }
}
