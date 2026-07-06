//! Internal validators shared by [`Report`] constructors and wire
//! deserialization. Private; not part of the public API.

use super::PerLaneEntry;
use crate::{error::ReportError, failure::LaneFailure, finding::Finding};
/// Check a reject report contains at least one finding or failure.
///
/// # Errors
/// Returns [`ReportError::EmptyReject`] when both reject collections are empty.
pub(super) fn check_reject_not_empty(
    code_findings: &[Finding],
    gate_failures: &[LaneFailure],
) -> Result<(), ReportError> {
    (!code_findings.is_empty() || !gate_failures.is_empty())
        .then_some(())
        .ok_or(ReportError::EmptyReject)
}

/// Check a pass report carries per-lane evidence.
///
/// # Errors
/// Returns [`ReportError::EmptyPerLane`] when `per_lane` is empty.
pub(super) fn check_per_lane_not_empty(per_lane: &[PerLaneEntry]) -> Result<(), ReportError> {
    (!per_lane.is_empty()).then_some(()).ok_or(ReportError::EmptyPerLane)
}

/// Check every lane outcome in `per_lane` is pass-shaped (Clean, Skipped,
/// or Findings with only informational findings).
///
/// # Errors
///
/// Returns [`ReportError::NonPassLaneOutcome`] for the first lane outcome
/// that is not pass-shaped.
pub(super) fn validate_per_lane_pass(per_lane: &[PerLaneEntry]) -> Result<(), ReportError> {
    per_lane.iter().find(|e| !e.outcome().is_pass()).map_or(Ok(()), |first_bad| {
        Err(ReportError::NonPassLaneOutcome(
            *first_bad.lane(),
            format!("{:?}", first_bad.outcome()),
        ))
    })
}

/// Which reject collections are empty, used to classify a reject report.
///
/// Grouping the two emptiness flags into one typed record keeps
/// `reject_kind_from_empty` under the workspace `max-fn-params-bools = 1`
/// policy while remaining explicit and pattern-matchable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RejectEmptiness {
    pub(super) code_empty: bool,
    pub(super) gate_empty: bool,
}

#[must_use]
pub(super) const fn reject_kind_from_empty(
    emptiness: RejectEmptiness,
) -> Option<super::RejectKind> {
    match (emptiness.code_empty, emptiness.gate_empty) {
        (false, true) => Some(super::RejectKind::CodeOnly),
        (true, false) => Some(super::RejectKind::GateOnly),
        (false, false) => Some(super::RejectKind::Mixed),
        (true, true) => None,
    }
}

#[must_use]
pub(super) const fn reject_kind_for(
    code_findings: &[Finding],
    gate_failures: &[LaneFailure],
) -> Option<super::RejectKind> {
    reject_kind_from_empty(RejectEmptiness {
        code_empty: code_findings.is_empty(),
        gate_empty: gate_failures.is_empty(),
    })
}
