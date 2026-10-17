//! Versioned required-versus-optional obligation contract (contract `obligations/v1`).
//!
//! The v1 aggregation boundary treats every [`titania_core::LaneOutcome::Skipped`] as
//! pass-shaped, so a mandatory lane with no execution evidence — skipped as
//! `NotApplicable`, or never run because its tool is missing
//! (`ToolUnavailable`) — still yields a `Pass` report. This module introduces
//! the explicit required-versus-optional distinction without rewriting v1
//! skip semantics:
//!
//! - [`ObligationSet::v1_legacy`] marks every scoped lane optional and
//!   reproduces v1 behaviour exactly (the v1 entry point delegates to it).
//! - [`ObligationSet::all_required`] and [`ObligationSet::requiring`] mark
//!   lanes mandatory: a required lane that is `Skipped` rejects the report
//!   with a typed [`titania_core::LaneFailure::Infra`] gate failure.
//!
//! Boundary status matrix for a required lane:
//!
//! | Status | Wire form | Outcome |
//! |---|---|---|
//! | Skipped | `LaneOutcome::Skipped` (any reason) | Reject (new) |
//! | Failed | `LaneOutcome::Failed` (incl. timeouts) | Reject (pre-existing) |
//! | Missing / substituted / duplicated | identity divergence | `Err`, no
//! report (pre-existing) |
//! | UNPROVEN | no producer: proof-lane no-verdict states arrive as reject
//! findings or `Failed` | Reject via the lane mapping (pre-existing) |
//!
//! The required-skip match in `report_assembly` is exhaustive over
//! [`titania_core::LaneOutcome`] by design: adding a future variant (for example an
//! explicit `Unproven` status) fails compilation there until this contract
//! classifies it for required lanes, so a new non-evidence status can never
//! silently inherit pass treatment.

use thiserror::Error;
use titania_core::{GateScope, Lane};

/// Whether a scoped lane must prove execution or keeps legacy v1 tolerance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObligationKind {
    /// The lane must prove execution. A `Skipped` outcome rejects the report.
    Required,
    /// The lane keeps v1 semantics. A `Skipped` outcome stays pass-shaped.
    Optional,
}

/// Versioned per-lane obligation set bound to one [`GateScope`].
///
/// The required list is stored in canonical scope order. Two sets are equal
/// only when scope and required lanes match exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObligationSet {
    scope: GateScope,
    required: Box<[Lane]>,
}

/// Errors building an [`ObligationSet`] with an explicit required list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ObligationError {
    /// A required lane is not part of the bound scope.
    #[error("lane {lane:?} is not part of scope {scope:?}")]
    LaneNotInScope {
        /// Scope the obligation set is bound to.
        scope: GateScope,
        /// Lane that does not participate in `scope`.
        lane: Lane,
    },
    /// A required lane was listed twice.
    #[error("duplicate required lane {lane:?}")]
    DuplicateLane {
        /// Lane listed more than once.
        lane: Lane,
    },
}

impl ObligationSet {
    /// Legacy v1 contract: every scoped lane is optional.
    ///
    /// Assembly with this set reproduces `assemble_report` behaviour exactly.
    #[must_use]
    pub fn v1_legacy(scope: GateScope) -> Self {
        Self { scope, required: Box::from([]) }
    }

    /// Strict contract: every scoped lane is required.
    ///
    /// Any scoped lane that is `Skipped` rejects the report.
    #[must_use]
    pub fn all_required(scope: GateScope) -> Self {
        let required = scope.lanes().to_vec().into_boxed_slice();
        Self { scope, required }
    }

    /// Contract with exactly `lanes` required; all other scoped lanes optional.
    ///
    /// # Errors
    /// - [`ObligationError::LaneNotInScope`] when a lane is not in `scope`.
    /// - [`ObligationError::DuplicateLane`] when a lane is listed twice.
    pub fn requiring(scope: GateScope, lanes: &[Lane]) -> Result<Self, ObligationError> {
        check_lanes_in_scope(scope, lanes)?;
        check_no_duplicates(lanes)?;
        let required = scope
            .lanes()
            .iter()
            .copied()
            .filter(|lane| lanes.contains(lane))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Ok(Self { scope, required })
    }

    /// Scope this obligation set is bound to.
    #[must_use]
    pub const fn scope(&self) -> GateScope {
        self.scope
    }

    /// Required lanes in canonical scope order.
    #[must_use]
    pub fn required_lanes(&self) -> &[Lane] {
        &self.required
    }

    /// Whether `lane` is mandatory under this contract.
    #[must_use]
    pub fn is_required(&self, lane: Lane) -> bool {
        self.required.contains(&lane)
    }

    /// Obligation kind of `lane` under this contract.
    #[must_use]
    pub fn kind(&self, lane: Lane) -> ObligationKind {
        self.is_required(lane)
            .then_some(ObligationKind::Required)
            .map_or(ObligationKind::Optional, core::convert::identity)
    }
}

/// Reject required lanes that do not participate in `scope`.
///
/// # Errors
/// Returns [`ObligationError::LaneNotInScope`] for the first outsider.
fn check_lanes_in_scope(scope: GateScope, lanes: &[Lane]) -> Result<(), ObligationError> {
    let outsider = lanes.iter().find(|candidate| !scope.lanes().contains(candidate));
    outsider.map_or(Ok(()), |lane| Err(ObligationError::LaneNotInScope { scope, lane: *lane }))
}

/// Reject duplicate entries in the required list.
///
/// # Errors
/// Returns [`ObligationError::DuplicateLane`] for the first repeated lane.
fn check_no_duplicates(lanes: &[Lane]) -> Result<(), ObligationError> {
    let repeated =
        lanes.iter().find(|lane| lanes.iter().filter(|candidate| candidate == lane).count() > 1);
    repeated.map_or(Ok(()), |lane| Err(ObligationError::DuplicateLane { lane: *lane }))
}
