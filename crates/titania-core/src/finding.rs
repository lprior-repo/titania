//! Structured findings from lane analysis.
//!
//! A `Finding` records a single violation (or informational note) observed
//! during lane execution, together with where it occurred and how it should
//! be repaired.

use serde::{Deserialize, Serialize};

use crate::{error::FindingError, lane::Lane, rule_id::RuleId};

mod location;
mod repair_hint;

pub use location::Location;
pub use repair_hint::RepairHint;

/// Whether a [`Finding`] causes the lane to reject or merely notes an issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingEffect {
    /// This finding must be resolved for the lane to pass.
    Reject,
    /// Informational only — the lane passes regardless.
    Informational,
}

/// A single finding from a lane analysis pass.
///
/// Once constructed via [`Finding::new`], all invariants are enforced: the
/// lane is known, the rule id is valid, the location is valid, and the
/// repair hint (if any) is applicable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    lane: Lane,
    rule_id: RuleId,
    location: Location,
    message: String,
    repair: RepairHint,
    effect: FindingEffect,
}

impl Finding {
    #[allow(clippy::too_many_arguments)]
    /// Construct a [`Finding`].
    ///
    /// # Errors
    /// - [`FindingError::EmptyMessage`] if `message` is empty.
    pub fn new(
        lane: Lane,
        rule_id: RuleId,
        location: Location,
        message: String,
        repair: RepairHint,
        effect: FindingEffect,
    ) -> Result<Self, FindingError> {
        if message.is_empty() {
            return Err(FindingError::EmptyMessage);
        }
        Ok(Self { lane, rule_id, location, message, repair, effect })
    }

    #[must_use]
    pub const fn lane(&self) -> Lane {
        self.lane
    }

    #[must_use]
    pub const fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    #[must_use]
    pub const fn location(&self) -> &Location {
        &self.location
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    #[must_use]
    pub const fn repair(&self) -> &RepairHint {
        &self.repair
    }

    #[must_use]
    pub const fn effect(&self) -> FindingEffect {
        self.effect
    }

    /// Whether this finding rejects the lane.
    #[must_use]
    pub fn is_reject(&self) -> bool {
        self.effect == FindingEffect::Reject
    }

    /// Whether this finding is informational only.
    #[must_use]
    pub fn is_informational(&self) -> bool {
        self.effect == FindingEffect::Informational
    }

    /// Whether this finding has an auto-applicable repair.
    #[must_use]
    pub const fn has_auto_repair(&self) -> bool {
        self.repair.is_auto_applicable()
    }
}
