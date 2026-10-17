use super::PerLaneEntry;
use crate::{error::ReportError, failure::LaneFailure, finding::Finding};
pub(super) fn check_reject_not_empty(
    code_findings: &[Finding],
    gate_failures: &[LaneFailure],
) -> Result<(), ReportError> {
    (!code_findings.is_empty() || !gate_failures.is_empty())
        .then_some(())
        .ok_or(ReportError::EmptyReject)
}

pub(super) fn check_per_lane_not_empty(per_lane: &[PerLaneEntry]) -> Result<(), ReportError> {
    (!per_lane.is_empty()).then_some(()).ok_or(ReportError::EmptyPerLane)
}

pub(super) fn validate_per_lane_pass(per_lane: &[PerLaneEntry]) -> Result<(), ReportError> {
    per_lane.iter().find(|e| !e.outcome().is_pass()).map_or(Ok(()), |first_bad| {
        Err(ReportError::NonPassLaneOutcome(
            *first_bad.lane(),
            format!("{:?}", first_bad.outcome()),
        ))
    })
}

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

pub(super) fn validate_per_lane_pass_scope(
    scope: crate::GateScope,
    per_lane: &[PerLaneEntry],
) -> Result<(), ReportError> {
    let expected = scope.lanes();

    let first_mismatch = expected.iter().zip(per_lane.iter()).enumerate().find_map(
        |(idx, (expected_lane, entry))| {
            (expected_lane != entry.lane()).then_some((idx, *expected_lane, *entry.lane()))
        },
    );

    if let Some((idx, expected_lane, got_lane)) = first_mismatch {
        let detail = mismatch_detail(per_lane, idx, expected_lane, got_lane);
        return Err(ReportError::PerLaneScopeMismatch { scope, error: detail });
    }

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
pub(super) fn validate_per_lane_reject(
    scope: crate::GateScope,
    per_lane: &[PerLaneEntry],
) -> Result<(), ReportError> {
    let canonical = scope.lanes();
    per_lane
        .iter()
        .try_fold(None, |last_pos, entry| {
            validate_reject_entry(canonical, scope, last_pos, *entry.lane())
        })
        .map(|_| ())
}

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
