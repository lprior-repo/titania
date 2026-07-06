//! Pure assembly from lane outcomes into a typed v1 [`Report`].
//!
//! This module does not compute receipt digests. The pass path requires a
//! caller-provided [`QualityReceipt`] produced by the artifact/digest layer, so
//! report assembly never fabricates source, lockfile, policy, toolchain, or lane
//! evidence digests.

use thiserror::Error;
use titania_core::{
    Finding, GateScope, InputDiagnostic, LaneFailure, LaneOutcome, PerLaneEntry, PolicyDiagnostic,
    QualityReceiptV1 as QualityReceipt, Report, ReportError,
};

/// Errors produced while assembling a [`Report`] from in-memory lane outcomes.
#[derive(Debug, Error)]
pub enum ReportAssemblyError {
    /// No lane outcomes were supplied, so neither Pass nor Reject invariants can hold.
    #[error("report assembly requires at least one lane outcome")]
    EmptyOutcomes,
    /// The number of lane outcomes does not match the selected scope.
    #[error("scope {scope:?} expects {expected} lane outcomes, got {actual}")]
    LaneCountMismatch {
        /// Scope being assembled.
        scope: GateScope,
        /// Number of lane outcomes required by the scope.
        expected: usize,
        /// Number of lane outcomes supplied by the caller.
        actual: usize,
    },
    /// The supplied pass receipt was built for a different scope.
    #[error("pass receipt scope {found:?} does not match requested scope {expected:?}")]
    ReceiptScopeMismatch {
        /// Scope requested for this aggregate report.
        expected: GateScope,
        /// Scope recorded in the caller-provided pass receipt.
        found: GateScope,
    },
    /// The supplied pass receipt has the wrong number of lane receipts.
    #[error("pass receipt for {scope:?} must contain {expected} lane receipts, got {actual}")]
    ReceiptLaneCountMismatch {
        /// Scope requested for this aggregate report.
        scope: GateScope,
        /// Number of lane receipts required by the scope.
        expected: usize,
        /// Number of lane receipts carried by the receipt.
        actual: usize,
    },
    /// Core report construction rejected the assembled fields.
    #[error(transparent)]
    Report(#[from] ReportError),
}

/// Assemble a typed [`Report`] from lane outcomes and diagnostics.
///
/// Diagnostics take precedence over lane verdicts. A pass report is emitted only
/// when all supplied findings are informational and no lane failed; the required
/// receipt must be supplied by the caller because receipt digests are computed
/// outside this pure classification step.
///
/// # Errors
/// - [`ReportAssemblyError::EmptyOutcomes`] when no lane outcomes are supplied.
/// - [`ReportAssemblyError::LaneCountMismatch`] when lane outcomes do not
///   include exactly the lanes required by `scope`.
/// - [`ReportAssemblyError::ReceiptScopeMismatch`] when `pass_receipt` belongs
///   to a different scope.
/// - [`ReportAssemblyError::ReceiptLaneCountMismatch`] when `pass_receipt` does
///   not contain one lane receipt for each scoped lane.
/// - [`ReportAssemblyError::Report`] when the core [`Report`] constructor rejects
///   the assembled fields.
pub fn assemble_report(
    scope: GateScope,
    outcomes: Box<[PerLaneEntry]>,
    pass_receipt: QualityReceipt,
    policy_diagnostics: Box<[PolicyDiagnostic]>,
    input_diagnostics: Box<[InputDiagnostic]>,
) -> Result<Report, ReportAssemblyError> {
    check_outcomes_not_empty(&outcomes)?;

    if !input_diagnostics.is_empty() {
        return Ok(Report::input_error(input_diagnostics));
    }

    if !policy_diagnostics.is_empty() {
        return Ok(Report::policy_error(policy_diagnostics));
    }

    check_scope_outcome_count(scope, &outcomes)?;

    let code_findings = rejecting_findings(&outcomes);
    let gate_failures = gate_failures(&outcomes);

    if has_rejection(&code_findings, &gate_failures) {
        return Report::reject(code_findings, gate_failures, outcomes).map_err(Into::into);
    }

    validate_pass_candidate(scope, &pass_receipt)?;
    Report::pass(pass_receipt, outcomes).map_err(Into::into)
}

fn rejecting_findings(entries: &[PerLaneEntry]) -> Box<[Finding]> {
    entries
        .iter()
        .filter_map(|e| findings(e.outcome()))
        .flatten()
        .filter(|finding| finding.is_reject())
        .cloned()
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

fn findings(outcome: &LaneOutcome) -> Option<std::slice::Iter<'_, Finding>> {
    match outcome {
        LaneOutcome::Findings { findings } => Some(findings.iter()),
        LaneOutcome::Clean { .. } | LaneOutcome::Failed { .. } | LaneOutcome::Skipped { .. } => {
            None
        }
    }
}

fn gate_failures(entries: &[PerLaneEntry]) -> Box<[LaneFailure]> {
    entries
        .iter()
        .filter_map(|e| failed(e.outcome()))
        .cloned()
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

const fn failed(outcome: &LaneOutcome) -> Option<&LaneFailure> {
    match outcome {
        LaneOutcome::Failed { failure } => Some(failure),
        LaneOutcome::Clean { .. } | LaneOutcome::Findings { .. } | LaneOutcome::Skipped { .. } => {
            None
        }
    }
}

/// Validate receipt invariants needed for a pass report.
///
/// # Errors
/// Returns [`ReportAssemblyError::ReceiptScopeMismatch`] or
/// [`ReportAssemblyError::ReceiptLaneCountMismatch`] when the supplied receipt
/// does not match the requested pass scope.
fn validate_pass_candidate(
    scope: GateScope,
    pass_receipt: &QualityReceipt,
) -> Result<(), ReportAssemblyError> {
    check_receipt_scope(scope, pass_receipt)?;
    check_receipt_lane_count(scope, pass_receipt)
}

/// Reject empty lane outcome sets before any report variant is built.
///
/// # Errors
/// Returns [`ReportAssemblyError::EmptyOutcomes`] when `outcomes` is empty.
fn check_outcomes_not_empty(entries: &[PerLaneEntry]) -> Result<(), ReportAssemblyError> {
    (!entries.is_empty()).then_some(()).ok_or(ReportAssemblyError::EmptyOutcomes)
}

/// Check the caller supplied exactly one outcome per scoped lane.
///
/// # Errors
/// Returns [`ReportAssemblyError::LaneCountMismatch`] when the outcome count
/// does not match [`GateScope::lanes`].
fn check_scope_outcome_count(
    scope: GateScope,
    entries: &[PerLaneEntry],
) -> Result<(), ReportAssemblyError> {
    let expected = scope.lanes().len();
    let actual = entries.len();
    (actual == expected).then_some(()).ok_or(ReportAssemblyError::LaneCountMismatch {
        scope,
        expected,
        actual,
    })
}

/// Check the pass receipt was produced for the requested scope.
///
/// # Errors
/// Returns [`ReportAssemblyError::ReceiptScopeMismatch`] when the receipt scope
/// differs from `scope`.
fn check_receipt_scope(
    scope: GateScope,
    pass_receipt: &QualityReceipt,
) -> Result<(), ReportAssemblyError> {
    (*pass_receipt.scope() == scope).then_some(()).ok_or_else(|| {
        ReportAssemblyError::ReceiptScopeMismatch { expected: scope, found: *pass_receipt.scope() }
    })
}

/// Check the pass receipt contains one lane receipt per scoped lane.
///
/// # Errors
/// Returns [`ReportAssemblyError::ReceiptLaneCountMismatch`] when the receipt
/// lane count does not match [`GateScope::lanes`].
fn check_receipt_lane_count(
    scope: GateScope,
    pass_receipt: &QualityReceipt,
) -> Result<(), ReportAssemblyError> {
    let expected = scope.lanes().len();
    let actual = pass_receipt.lanes().len();
    (actual == expected).then_some(()).ok_or(ReportAssemblyError::ReceiptLaneCountMismatch {
        scope,
        expected,
        actual,
    })
}

#[must_use]
const fn has_rejection(code_findings: &[Finding], gate_failures: &[LaneFailure]) -> bool {
    !code_findings.is_empty() || !gate_failures.is_empty()
}
