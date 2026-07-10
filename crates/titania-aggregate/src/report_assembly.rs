//! Pure assembly from lane outcomes into a typed v1 [`Report`].
//!
//! This module does not compute receipt digests. The pass path requires a
//! caller-provided [`QualityReceipt`] produced by the artifact/digest layer, so
//! report assembly never fabricates source, lockfile, policy, toolchain, or lane
//! evidence digests.

use thiserror::Error;
use titania_core::{
    Finding, GateScope, InputDiagnostic, Lane, LaneFailure, LaneOutcome, LaneReceipt, PerLaneEntry,
    PolicyDiagnostic, QualityReceiptV1 as QualityReceipt, Report, ReportError,
};

/// Errors produced while assembling a [`Report`] from in-memory lane outcomes.
#[derive(Debug, Error)]
pub enum ReportAssemblyError {
    /// No outcomes or diagnostics were supplied, so no report variant can be built.
    #[error("report assembly requires a lane outcome or diagnostic")]
    EmptyOutcomes,
    /// The supplied per-lane outcomes do not match the canonical lane sequence of `scope`.
    ///
    /// Returned when the outcome list omits, duplicates, substitutes, or reorders lanes
    /// relative to [`GateScope::lanes`]. The first divergence (by index) is reported so
    /// the error message names exactly the offending slot.
    #[error("scope {scope:?} expects lane {expected:?} at index {index}, found {found:?}")]
    LaneIdentityMismatch {
        /// Scope being assembled.
        scope: GateScope,
        /// Index into the canonical lane sequence where the divergence was observed.
        index: usize,
        /// Lane required by the canonical sequence at `index`.
        expected: Lane,
        /// Lane observed in the caller's outcomes at `index`, if any.
        found: Option<Lane>,
    },
    /// The supplied pass receipt was built for a different scope.
    #[error("pass receipt scope {found:?} does not match requested scope {expected:?}")]
    ReceiptScopeMismatch {
        /// Scope requested for this aggregate report.
        expected: GateScope,
        /// Scope recorded in the caller-provided pass receipt.
        found: GateScope,
    },
    /// The supplied pass receipt does not match the canonical lane sequence of `scope`.
    ///
    /// Returned when the receipt lane list omits, duplicates, substitutes, or reorders
    /// lanes relative to [`GateScope::lanes`]. The first divergence (by index) is
    /// reported so the error message names exactly the offending slot.
    #[error(
        "pass receipt for {scope:?} must contain lane {expected:?} at index {index}, found {found:?}"
    )]
    ReceiptLaneIdentityMismatch {
        /// Scope requested for this aggregate report.
        scope: GateScope,
        /// Index into the canonical lane sequence where the divergence was observed.
        index: usize,
        /// Lane required by the canonical sequence at `index`.
        expected: Lane,
        /// Lane observed in the receipt at `index`, if any.
        found: Option<Lane>,
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
/// - [`ReportAssemblyError::EmptyOutcomes`] when no lane outcomes or diagnostics
///   are supplied.
/// - [`ReportAssemblyError::LaneIdentityMismatch`] when lane outcomes do not
///   match the canonical lane sequence of `scope` (omission, duplicate,
///   substitution, or reordering).
/// - [`ReportAssemblyError::ReceiptScopeMismatch`] when `pass_receipt` belongs
///   to a different scope.
/// - [`ReportAssemblyError::ReceiptLaneIdentityMismatch`] when `pass_receipt`
///   lane list does not match the canonical lane sequence of `scope`.
/// - [`ReportAssemblyError::Report`] when the core [`Report`] constructor rejects
///   the assembled fields.
pub fn assemble_report(
    scope: GateScope,
    outcomes: Box<[PerLaneEntry]>,
    pass_receipt: QualityReceipt,
    policy_diagnostics: Box<[PolicyDiagnostic]>,
    input_diagnostics: Box<[InputDiagnostic]>,
) -> Result<Report, ReportAssemblyError> {
    if !input_diagnostics.is_empty() {
        return Ok(Report::input_error(input_diagnostics));
    }

    if !policy_diagnostics.is_empty() {
        return Ok(Report::policy_error(policy_diagnostics));
    }

    check_outcomes_not_empty(&outcomes)?;

    check_scope_outcome_identities(scope, &outcomes)?;

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
        .filter_map(|entry| failed(entry.outcome()))
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
/// # Errors
/// Returns [`ReportAssemblyError::ReceiptLaneIdentityMismatch`] when the supplied
/// receipt does not match the requested pass scope.
fn validate_pass_candidate(
    scope: GateScope,
    pass_receipt: &QualityReceipt,
) -> Result<(), ReportAssemblyError> {
    check_receipt_scope(scope, pass_receipt)?;
    check_receipt_lane_identities(scope, pass_receipt)
}

/// Reject empty lane outcome sets before any report variant is built.
///
/// # Errors
/// Returns [`ReportAssemblyError::EmptyOutcomes`] when `outcomes` is empty.
fn check_outcomes_not_empty(entries: &[PerLaneEntry]) -> Result<(), ReportAssemblyError> {
    (!entries.is_empty()).then_some(()).ok_or(ReportAssemblyError::EmptyOutcomes)
}

/// Select the expected lane for a divergence index without unchecked access.
///
/// # Errors
/// Returns [`ReportAssemblyError::EmptyOutcomes`] when the scope has no lanes.
fn expected_lane(canonical: &[Lane], index: usize) -> Result<Lane, ReportAssemblyError> {
    canonical
        .get(index)
        .copied()
        .or_else(|| canonical.last().copied())
        .ok_or(ReportAssemblyError::EmptyOutcomes)
}

/// Check the caller supplied exactly one outcome per scoped lane, in the
/// canonical order required by [`GateScope::lanes`].
///
/// The check rejects omissions, duplicates, substitutions, and reorderings
/// by comparing each entry's [`PerLaneEntry::lane`] against the canonical
/// lane at the same index. The first divergence (by index) is reported so the
/// error names exactly the offending slot.
///
/// # Errors
/// Returns [`ReportAssemblyError::LaneIdentityMismatch`] when an entry is
/// missing, repeated, swapped, or otherwise does not match the canonical
/// lane at its index.
fn check_scope_outcome_identities(
    scope: GateScope,
    entries: &[PerLaneEntry],
) -> Result<(), ReportAssemblyError> {
    let canonical = scope.lanes();
    if entries.len() != canonical.len() {
        // The first divergence is the index where canonical and entries
        // stop agreeing. When the caller supplied fewer entries than
        // canonical, the divergence is the first missing slot. When the
        // caller supplied extra entries, the divergence is the first
        // index past canonical — `expected` is reported as the first
        // canonical lane so the error always names a valid in-scope
        // lane, and the slot number alone distinguishes the two cases.
        let index = entries.len().min(canonical.len());
        let expected = expected_lane(canonical, index)?;
        return Err(ReportAssemblyError::LaneIdentityMismatch {
            scope,
            index,
            expected,
            found: entries.get(index).map(PerLaneEntry::lane).copied(),
        });
    }
    canonical
        .iter()
        .zip(entries.iter())
        .enumerate()
        .find_map(|(index, (expected, entry))| {
            (entry.lane() != expected).then_some((index, *expected, *entry.lane()))
        })
        .map_or(Ok(()), |(index, expected, found)| {
            Err(ReportAssemblyError::LaneIdentityMismatch {
                scope,
                index,
                expected,
                found: Some(found),
            })
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

/// Check the pass receipt contains one lane receipt per scoped lane, in the
/// canonical order required by [`GateScope::lanes`].
///
/// The check rejects omissions, duplicates, substitutions, and reorderings
/// by comparing each receipt's [`LaneReceipt::lane`] against the canonical
/// lane at the same index. The first divergence (by index) is reported
/// so the error names exactly the offending slot.
///
/// # Errors
/// Returns [`ReportAssemblyError::ReceiptLaneIdentityMismatch`] when a
/// receipt is missing, repeated, swapped, or otherwise does not match
/// the canonical lane at its index.
fn check_receipt_lane_identities(
    scope: GateScope,
    pass_receipt: &QualityReceipt,
) -> Result<(), ReportAssemblyError> {
    let canonical = scope.lanes();
    let actual = pass_receipt.lanes();
    if actual.len() != canonical.len() {
        // The first divergence is the index where canonical and actual
        // stop agreeing. When the receipt has fewer entries than
        // canonical, the divergence is the first missing slot. When the
        // receipt has extra entries, the divergence is the first index
        // past canonical — `expected` is reported as the first canonical
        // lane so the error always names a valid in-scope lane, and the
        // slot number alone distinguishes the two cases.
        let index = actual.len().min(canonical.len());
        let expected = expected_lane(canonical, index)?;
        return Err(ReportAssemblyError::ReceiptLaneIdentityMismatch {
            scope,
            index,
            expected,
            found: actual.get(index).map(LaneReceipt::lane).copied(),
        });
    }
    canonical
        .iter()
        .zip(actual.iter())
        .enumerate()
        .find_map(|(index, (expected, receipt))| {
            (receipt.lane() != expected).then_some((index, *expected, *receipt.lane()))
        })
        .map_or(Ok(()), |(index, expected, found)| {
            Err(ReportAssemblyError::ReceiptLaneIdentityMismatch {
                scope,
                index,
                expected,
                found: Some(found),
            })
        })
}

#[must_use]
const fn has_rejection(code_findings: &[Finding], gate_failures: &[LaneFailure]) -> bool {
    !code_findings.is_empty() || !gate_failures.is_empty()
}
