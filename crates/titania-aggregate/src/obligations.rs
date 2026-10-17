use thiserror::Error;
use titania_core::{GateScope, Lane};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObligationKind {
    Required,
    Optional,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObligationSet {
    scope: GateScope,
    required: Box<[Lane]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ObligationError {
    #[error("lane {lane:?} is not part of scope {scope:?}")]
    LaneNotInScope { scope: GateScope, lane: Lane },
    #[error("duplicate required lane {lane:?}")]
    DuplicateLane { lane: Lane },
}

impl ObligationSet {
    #[must_use]
    pub fn v1_legacy(scope: GateScope) -> Self {
        Self { scope, required: Box::from([]) }
    }

    #[must_use]
    pub fn all_required(scope: GateScope) -> Self {
        let required = scope.lanes().to_vec().into_boxed_slice();
        Self { scope, required }
    }

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

    #[must_use]
    pub const fn scope(&self) -> GateScope {
        self.scope
    }

    #[must_use]
    pub fn required_lanes(&self) -> &[Lane] {
        &self.required
    }

    #[must_use]
    pub fn is_required(&self, lane: Lane) -> bool {
        self.required.contains(&lane)
    }

    #[must_use]
    pub fn kind(&self, lane: Lane) -> ObligationKind {
        self.is_required(lane)
            .then_some(ObligationKind::Required)
            .map_or(ObligationKind::Optional, core::convert::identity)
    }
}

fn check_lanes_in_scope(scope: GateScope, lanes: &[Lane]) -> Result<(), ObligationError> {
    let outsider = lanes.iter().find(|candidate| !scope.lanes().contains(candidate));
    outsider.map_or(Ok(()), |lane| Err(ObligationError::LaneNotInScope { scope, lane: *lane }))
}

fn check_no_duplicates(lanes: &[Lane]) -> Result<(), ObligationError> {
    let repeated =
        lanes.iter().find(|lane| lanes.iter().filter(|candidate| candidate == lane).count() > 1);
    repeated.map_or(Ok(()), |lane| Err(ObligationError::DuplicateLane { lane: *lane }))
}
