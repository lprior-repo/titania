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

fn mismatch_detail(
    per_lane: &[PerLaneEntry],
    idx: usize,
    expected: crate::Lane,
    got: crate::Lane,
) -> crate::error::PerLaneScopeError {
    if per_lane.get(..idx).is_some_and(|prefix| prefix.iter().any(|entry| entry.lane() == &got)) {
        crate::error::PerLaneScopeError::Duplicate(got)
    } else {
        crate::error::PerLaneScopeError::OutOfOrder { previous: expected, got }
    }
}

/// Validate that `per_lane`'s lane identities exactly match the canonical
/// ordered lane sequence required by `scope` for a [`super::Report::pass`].
///
/// Each per-lane identity is checked in lockstep against `scope.lanes()`:
/// the sequence must be the same length, the same order, and free of
/// duplicates. Any deviation produces a typed [`PerLaneScopeError`] wrapped
/// in [`ReportError::PerLaneScopeMismatch`].
///
/// # Errors
///
/// Returns [`ReportError::PerLaneScopeMismatch`] carrying a
/// [`PerLaneScopeError::Missing`], [`PerLaneScopeError::Extra`],
/// [`PerLaneScopeError::OutOfOrder`], or [`PerLaneScopeError::Duplicate`]
/// detail describing the first violation encountered.
pub(super) fn validate_per_lane_pass_scope(
    scope: crate::GateScope,
    per_lane: &[PerLaneEntry],
) -> Result<(), ReportError> {
    let expected = scope.lanes();

    // Walk the overlap in lockstep; first mismatch wins so the caller gets
    // the most actionable diagnostic (the offending position, not the tail).
    let first_mismatch = expected.iter().zip(per_lane.iter()).enumerate().find_map(
        |(idx, (expected_lane, entry))| {
            (expected_lane != entry.lane()).then_some((idx, *expected_lane, *entry.lane()))
        },
    );

    if let Some((idx, expected_lane, got_lane)) = first_mismatch {
        let detail = mismatch_detail(per_lane, idx, expected_lane, got_lane);
        return Err(ReportError::PerLaneScopeMismatch { scope, error: detail });
    }

    // The overlap matched; any remaining length difference is a missing-or-extra
    // condition.
    if let Some(extra_entry) = per_lane.get(expected.len()) {
        return Err(ReportError::PerLaneScopeMismatch {
            scope,
            error: crate::error::PerLaneScopeError::Extra(*extra_entry.lane()),
        });
    }
    if let Some(missing_lane) = expected.get(per_lane.len()).copied() {
        return Err(ReportError::PerLaneScopeMismatch {
            scope,
            error: crate::error::PerLaneScopeError::Missing(missing_lane),
        });
    }
    Ok(())
}
/// The canonical lane DAG is the ordered set of all lanes that appear in any
/// [`crate::GateScope`] (equivalent to `GateScope::Release.lanes()`, since
/// Release is the superset).
///
/// Each per-lane identity must be known, unique, and strictly later in
/// canonical order than every prior entry.
///
/// Used by [`super::Report::reject`] since the reject constructor does not
/// receive a receipt/scope; the v1 DAG is the implicit scope.
///
/// # Errors
///
/// Returns [`ReportError::PerLaneScopeMismatch`] for an unknown, duplicate, or
/// out-of-order lane.
pub(super) fn validate_per_lane_reject(per_lane: &[PerLaneEntry]) -> Result<(), ReportError> {
    let canonical = crate::GateScope::Release.lanes();
    let scope = crate::GateScope::Release;
    per_lane
        .iter()
        .try_fold(None, |last_pos, entry| {
            validate_reject_entry(canonical, scope, last_pos, *entry.lane())
        })
        .map(|_| ())
}

/// Validate one lane and return its canonical position for the next entry.
///
/// # Errors
///
/// Returns [`ReportError::PerLaneScopeMismatch`] when `lane` violates the
/// canonical sequence.
fn validate_reject_entry(
    canonical: &[crate::Lane],
    scope: crate::GateScope,
    previous: Option<usize>,
    lane: crate::Lane,
) -> Result<Option<usize>, ReportError> {
    let Some(pos) = canonical.iter().position(|candidate| *candidate == lane) else {
        return Err(ReportError::PerLaneScopeMismatch {
            scope,
            error: crate::error::PerLaneScopeError::Extra(lane),
        });
    };
    match previous {
        Some(previous_pos) if pos == previous_pos => Err(ReportError::PerLaneScopeMismatch {
            scope,
            error: crate::error::PerLaneScopeError::Duplicate(lane),
        }),
        Some(previous_pos) if pos < previous_pos => {
            let previous_lane = canonical.get(previous_pos).copied().map_or(lane, |item| item);
            Err(ReportError::PerLaneScopeMismatch {
                scope,
                error: crate::error::PerLaneScopeError::OutOfOrder {
                    previous: previous_lane,
                    got: lane,
                },
            })
        }
        None | Some(_) => Ok(Some(pos)),
    }
}
