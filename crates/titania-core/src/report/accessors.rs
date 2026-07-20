//! Constructor and accessor methods for [`Report`].
//!
//! Constructors enforce report invariants and gate the private
//! [`ReportInner`] enum; accessors return typed references to the
//! inner variant's fields. Both live in this file because the
//! newtype pattern keeps the public API small — adding a second
//! file for ~80 lines of tightly coupled methods would be overhead.

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
    /// Create a [`Report`] in the reject state, validating the invariants.
    ///
    /// # Errors
    /// - [`ReportError::EmptyReject`] if both `code_findings` and
    ///   `gate_failures` are empty.
    /// - [`ReportError::PerLaneScopeMismatch`] if `per_lane`'s lane
    ///   identities contain a duplicate, appear out of v1 DAG order, or
    ///   include a lane not in `scope`'s canonical lane DAG.
    ///
    /// # Notes
    /// Per v1 §10, a `Reject` is only required to have at least one of
    /// `code_findings` or `gate_failures` non-empty. `per_lane` may be
    /// empty for CodeOnly/GateOnly rejects; the v1 constructor permits
    /// it. Pass reports still require non-empty `per_lane`.
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

    /// Create a [`Report`] in the pass state.
    ///
    /// # Errors
    /// - [`ReportError::EmptyPerLane`] if `per_lane` is empty.
    /// - [`ReportError::NonPassLaneOutcome`] if any lane outcome is not
    ///   pass-shaped (i.e., not `Clean`, `Skipped`, or informational-only
    ///   `Findings`).
    /// - [`ReportError::PerLaneScopeMismatch`] if `per_lane`'s lane
    ///   identities are not exactly the ordered lane sequence required by
    ///   `receipt.scope()` (missing, extra, duplicate, or out of order).
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

    /// Create a [`Report`] in the policy-error state.
    #[must_use]
    pub const fn policy_error(diagnostics: Box<[PolicyDiagnostic]>) -> Self {
        Self(ReportInner::PolicyError { diagnostics })
    }

    /// Create a [`Report`] in the input-error state.
    #[must_use]
    pub const fn input_error(diagnostics: Box<[InputDiagnostic]>) -> Self {
        Self(ReportInner::InputError { diagnostics })
    }

    /// Classify the reject kind, if this report is a reject.
    ///
    /// Returns `None` for non-reject reports or if both collections are
    /// empty (invariant violation).
    #[must_use]
    pub fn reject_kind(&self) -> Option<RejectKind> {
        match &self.0 {
            ReportInner::Reject { code_findings: c, gate_failures: g, .. } => reject_kind_for(c, g),
            _ => None,
        }
    }

    /// Whether this report represents a pass.
    #[must_use]
    pub const fn is_pass(&self) -> bool {
        matches!(&self.0, ReportInner::Pass { .. })
    }

    /// Whether this report represents a reject.
    #[must_use]
    pub const fn is_reject(&self) -> bool {
        matches!(&self.0, ReportInner::Reject { .. })
    }

    /// Whether this report is a policy error.
    #[must_use]
    pub const fn is_policy_error(&self) -> bool {
        matches!(&self.0, ReportInner::PolicyError { .. })
    }

    /// Whether this report is an input error.
    #[must_use]
    pub const fn is_input_error(&self) -> bool {
        matches!(&self.0, ReportInner::InputError { .. })
    }

    /// If this is a reject, return the code findings.
    #[must_use]
    pub fn code_findings(&self) -> Option<&[Finding]> {
        match &self.0 {
            ReportInner::Reject { code_findings, .. } => Some(code_findings),
            _ => None,
        }
    }

    /// If this is a reject, return the gate failures.
    #[must_use]
    pub fn gate_failures(&self) -> Option<&[LaneFailure]> {
        match &self.0 {
            ReportInner::Reject { gate_failures, .. } => Some(gate_failures),
            _ => None,
        }
    }

    /// If this is a pass, return the receipt.
    #[must_use]
    pub const fn receipt(&self) -> Option<&QualityReceipt> {
        match &self.0 {
            ReportInner::Pass { receipt, .. } => Some(receipt),
            _ => None,
        }
    }

    /// If this is a pass, return the per-lane outcomes.
    #[must_use]
    pub fn per_lane(&self) -> Option<&[PerLaneEntry]> {
        Some(match &self.0 {
            ReportInner::Pass { per_lane, .. } | ReportInner::Reject { per_lane, .. } => per_lane,
            _ => return None,
        })
    }

    /// If this is a policy error, return the diagnostics.
    #[must_use]
    pub fn policy_diagnostics(&self) -> Option<&[PolicyDiagnostic]> {
        match &self.0 {
            ReportInner::PolicyError { diagnostics } => Some(diagnostics),
            _ => None,
        }
    }

    /// If this is an input error, return the diagnostics.
    #[must_use]
    pub fn input_diagnostics(&self) -> Option<&[InputDiagnostic]> {
        match &self.0 {
            ReportInner::InputError { diagnostics } => Some(diagnostics),
            _ => None,
        }
    }

    /// Return the kind of this report.
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
