use thiserror::Error;
use titania_core::{
    Finding, GateScope, InputDiagnostic, Lane, LaneFailure, LaneOutcome, LaneReceipt, PerLaneEntry,
    PolicyDiagnostic, QualityReceiptV1 as QualityReceipt, Report, ReportError, SkipReason,
};

use crate::obligations::ObligationSet;

#[derive(Debug, Error)]
pub enum ReportAssemblyError {
    #[error("report assembly requires a lane outcome or diagnostic")]
    EmptyOutcomes,
    #[error("scope {scope:?} expects lane {expected:?} at index {index}, found {found:?}")]
    LaneIdentityMismatch { scope: GateScope, index: usize, expected: Lane, found: Option<Lane> },
    #[error("pass receipt scope {found:?} does not match requested scope {expected:?}")]
    ReceiptScopeMismatch { expected: GateScope, found: GateScope },
    #[error(
        "pass receipt for {scope:?} must contain lane {expected:?} at index {index}, found {found:?}"
    )]
    ReceiptLaneIdentityMismatch {
        scope: GateScope,
        index: usize,
        expected: Lane,
        found: Option<Lane>,
    },
    #[error(transparent)]
    Report(#[from] ReportError),
}

pub fn assemble_report(
    scope: GateScope,
    outcomes: Box<[PerLaneEntry]>,
    pass_receipt: QualityReceipt,
    policy_diagnostics: Box<[PolicyDiagnostic]>,
    input_diagnostics: Box<[InputDiagnostic]>,
) -> Result<Report, ReportAssemblyError> {
    assemble_report_with_obligations(
        &ObligationSet::v1_legacy(scope),
        outcomes,
        pass_receipt,
        policy_diagnostics,
        input_diagnostics,
    )
}

pub fn assemble_report_with_obligations(
    obligations: &ObligationSet,
    outcomes: Box<[PerLaneEntry]>,
    pass_receipt: QualityReceipt,
    policy_diagnostics: Box<[PolicyDiagnostic]>,
    input_diagnostics: Box<[InputDiagnostic]>,
) -> Result<Report, ReportAssemblyError> {
    let scope = obligations.scope();
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
    let required_skips = required_skip_failures(&outcomes, obligations);

    if has_rejection(&code_findings, &gate_failures) || !required_skips.is_empty() {
        let failures = append_failures(&gate_failures, &required_skips);
        return Report::reject(scope, code_findings, failures, outcomes).map_err(Into::into);
    }

    validate_pass_candidate(scope, &pass_receipt)?;
    Report::pass(pass_receipt, outcomes).map_err(Into::into)
}

fn required_skip_failures(
    entries: &[PerLaneEntry],
    obligations: &ObligationSet,
) -> Box<[LaneFailure]> {
    entries
        .iter()
        .filter(|entry| obligations.is_required(*entry.lane()))
        .filter_map(required_skip_failure)
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

fn required_skip_failure(entry: &PerLaneEntry) -> Option<LaneFailure> {
    match entry.outcome() {
        LaneOutcome::Skipped { reason: SkipReason::PriorCompilationFailure } => None,
        LaneOutcome::Skipped { reason } => Some(LaneFailure::Infra {
            tool: entry.lane().name().to_owned(),
            reason: skip_reason_text(*reason),
        }),
        LaneOutcome::Clean { .. } | LaneOutcome::Findings { .. } | LaneOutcome::Failed { .. } => {
            None
        }
    }
}

fn skip_reason_text(reason: SkipReason) -> String {
    format!("required lane skipped: {reason:?}")
}

fn append_failures(first: &[LaneFailure], second: &[LaneFailure]) -> Box<[LaneFailure]> {
    first.iter().chain(second.iter()).cloned().collect::<Vec<_>>().into_boxed_slice()
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
fn validate_pass_candidate(
    scope: GateScope,
    pass_receipt: &QualityReceipt,
) -> Result<(), ReportAssemblyError> {
    check_receipt_scope(scope, pass_receipt)?;
    check_receipt_lane_identities(scope, pass_receipt)
}

fn check_outcomes_not_empty(entries: &[PerLaneEntry]) -> Result<(), ReportAssemblyError> {
    (!entries.is_empty()).then_some(()).ok_or(ReportAssemblyError::EmptyOutcomes)
}

fn expected_lane(canonical: &[Lane], index: usize) -> Result<Lane, ReportAssemblyError> {
    canonical
        .get(index)
        .copied()
        .or_else(|| canonical.last().copied())
        .ok_or(ReportAssemblyError::EmptyOutcomes)
}

fn check_scope_outcome_identities(
    scope: GateScope,
    entries: &[PerLaneEntry],
) -> Result<(), ReportAssemblyError> {
    let canonical = scope.lanes();
    if entries.len() != canonical.len() {
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

fn check_receipt_scope(
    scope: GateScope,
    pass_receipt: &QualityReceipt,
) -> Result<(), ReportAssemblyError> {
    (*pass_receipt.scope() == scope).then_some(()).ok_or_else(|| {
        ReportAssemblyError::ReceiptScopeMismatch { expected: scope, found: *pass_receipt.scope() }
    })
}

fn check_receipt_lane_identities(
    scope: GateScope,
    pass_receipt: &QualityReceipt,
) -> Result<(), ReportAssemblyError> {
    let canonical = scope.lanes();
    let actual = pass_receipt.lanes();
    if actual.len() != canonical.len() {
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
