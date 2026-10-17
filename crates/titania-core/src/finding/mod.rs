use serde::{Deserialize, Serialize};

mod location;
mod repair_catalog;
mod repair_hint;

pub use location::Location;
pub use repair_catalog::{CatalogRow, catalog_rows};
pub use repair_hint::{RepairHint, RepairHintClass};

use crate::{lane::Lane, rule_id::RuleId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FindingEffect {
    Reject,
    Informational,
}

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

    #[must_use]
    pub fn is_reject(&self) -> bool {
        self.effect == FindingEffect::Reject
    }

    #[must_use]
    pub fn is_informational(&self) -> bool {
        self.effect == FindingEffect::Informational
    }

    #[must_use]
    pub const fn has_auto_repair(&self) -> bool {
        self.repair.is_auto_applicable()
    }
}
