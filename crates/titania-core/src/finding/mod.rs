//! Structured findings from lane analysis.
//!
//! A `Finding` records a single violation (or informational note) observed
//! during lane execution, together with where it occurred and how it should
//! be repaired.

use serde::{Deserialize, Serialize};

mod location;
mod repair_catalog;
mod repair_hint;

pub use location::Location;
pub use repair_catalog::{CatalogRow, catalog_rows};
pub use repair_hint::{RepairHint, RepairHintClass};

use crate::{lane::Lane, rule_id::RuleId};

/// Whether a [`Finding`] causes the lane to reject or merely notes an issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FindingEffect {
    /// This finding must be resolved for the lane to pass.
    Reject,
    /// Informational only — the lane passes regardless.
    Informational,
}

/// A single finding from a lane analysis pass.
///
/// Once constructed via [`Finding::reject`] or [`Finding::informational`], all
/// invariants are enforced: the lane is known, the rule id is valid, the
/// location is valid, and the repair hint (if any) is applicable.
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
    /// Construct a [`Finding`] that rejects the lane.
    ///
    /// The lane, rule id, location, message, and repair hint are assembled into
    /// a new finding whose effect is set to [`FindingEffect::Reject`].
    ///
    /// Validation of individual fields is the responsibility of the caller —
    /// [`RuleId::new`] and [`Location::span`] enforce their own invariants
    /// and return `Result`; [`RepairHint::patch`] is infallible because the
    /// byte-range bounds precondition is owned by the `TextRange` type.
    #[must_use]
    pub const fn reject(
        lane: Lane,
        rule_id: RuleId,
        location: Location,
        message: String,
        repair: RepairHint,
    ) -> Self {
        Self { lane, rule_id, location, message, repair, effect: FindingEffect::Reject }
    }

    /// Construct a [`Finding`] that is informational only.
    ///
    /// Behaves identically to [`Self::reject`] except the effect is set to
    /// [`FindingEffect::Informational`].
    #[must_use]
    pub const fn informational(
        lane: Lane,
        rule_id: RuleId,
        location: Location,
        message: String,
        repair: RepairHint,
    ) -> Self {
        Self { lane, rule_id, location, message, repair, effect: FindingEffect::Informational }
    }

    /// Lane that produced this finding.
    #[must_use]
    pub const fn lane(&self) -> Lane {
        self.lane
    }

    /// Rule identifier that was violated.
    #[must_use]
    pub const fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    /// Location where the finding was observed.
    #[must_use]
    pub const fn location(&self) -> &Location {
        &self.location
    }

    /// Human-readable finding message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Repair hint selected for this finding.
    #[must_use]
    pub const fn repair(&self) -> &RepairHint {
        &self.repair
    }

    /// Whether the finding rejects the lane or is informational.
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
