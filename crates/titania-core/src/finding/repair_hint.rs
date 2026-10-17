use serde::{Deserialize, Serialize};

use crate::text_range::TextRange;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
enum RepairHintInner {
    Patch { file: String, range: TextRange, replacement: String },
    UseIteratorPipeline { suggestion: String },
    FlattenNesting { suggestion: String },
    UseCheckedArithmetic { op: String },
    RemoveAllowAttribute { attr: String },
    ReplaceDependency { from: String, to: String },
    RequiresHumanReview { note: String },
}

impl RepairHintInner {
    const fn is_patch(&self) -> bool {
        matches!(self, Self::Patch { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct RepairHint(RepairHintInner);

impl RepairHint {
    #[must_use]
    pub const fn patch(file: String, range: TextRange, replacement: String) -> Self {
        Self(RepairHintInner::Patch { file, range, replacement })
    }

    #[must_use]
    pub const fn use_iterator_pipeline(suggestion: String) -> Self {
        Self(RepairHintInner::UseIteratorPipeline { suggestion })
    }

    #[must_use]
    pub const fn flatten_nesting(suggestion: String) -> Self {
        Self(RepairHintInner::FlattenNesting { suggestion })
    }

    #[must_use]
    pub const fn use_checked_arithmetic(op: String) -> Self {
        Self(RepairHintInner::UseCheckedArithmetic { op })
    }

    #[must_use]
    pub const fn remove_allow_attribute(attr: String) -> Self {
        Self(RepairHintInner::RemoveAllowAttribute { attr })
    }

    #[must_use]
    pub const fn replace_dependency(from: String, to: String) -> Self {
        Self(RepairHintInner::ReplaceDependency { from, to })
    }

    #[must_use]
    pub const fn requires_human_review(note: String) -> Self {
        Self(RepairHintInner::RequiresHumanReview { note })
    }

    #[must_use]
    pub const fn is_auto_applicable(&self) -> bool {
        self.0.is_patch()
    }
}

#[derive(Deserialize)]
enum RepairHintReadWire {
    Patch { file: String, range: TextRange, replacement: String },
    UseIteratorPipeline { suggestion: String },
    FlattenNesting { suggestion: String },
    UseCheckedArithmetic { op: String },
    RemoveAllowAttribute { attr: String },
    ReplaceDependency { from: String, to: String },
    RequiresHumanReview { note: String },
}

const fn construct_patch(file: String, range: TextRange, replacement: String) -> RepairHint {
    RepairHint(RepairHintInner::Patch { file, range, replacement })
}

const fn iterator_pipeline(suggestion: String) -> RepairHint {
    RepairHint(RepairHintInner::UseIteratorPipeline { suggestion })
}

const fn flatten_nesting(suggestion: String) -> RepairHint {
    RepairHint(RepairHintInner::FlattenNesting { suggestion })
}

const fn checked_arithmetic(op: String) -> RepairHint {
    RepairHint(RepairHintInner::UseCheckedArithmetic { op })
}

const fn remove_allow(attr: String) -> RepairHint {
    RepairHint(RepairHintInner::RemoveAllowAttribute { attr })
}

const fn replace_dependency(from: String, to: String) -> RepairHint {
    RepairHint(RepairHintInner::ReplaceDependency { from, to })
}

const fn human_review(note: String) -> RepairHint {
    RepairHint(RepairHintInner::RequiresHumanReview { note })
}

fn repair_hint_from_wire(wire: RepairHintReadWire) -> RepairHint {
    match wire {
        RepairHintReadWire::Patch { file, range, replacement } => {
            construct_patch(file, range, replacement)
        }
        RepairHintReadWire::UseIteratorPipeline { suggestion } => iterator_pipeline(suggestion),
        RepairHintReadWire::FlattenNesting { suggestion } => flatten_nesting(suggestion),
        RepairHintReadWire::UseCheckedArithmetic { op } => checked_arithmetic(op),
        RepairHintReadWire::RemoveAllowAttribute { attr } => remove_allow(attr),
        RepairHintReadWire::ReplaceDependency { from, to } => replace_dependency(from, to),
        RepairHintReadWire::RequiresHumanReview { note } => human_review(note),
    }
}

fn deserialize_repair_hint<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<RepairHint, D::Error> {
    let wire = RepairHintReadWire::deserialize(deserializer)?;
    Ok(repair_hint_from_wire(wire))
}

impl<'de> Deserialize<'de> for RepairHint {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserialize_repair_hint(deserializer)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RepairHintClass {
    Patch,
    UseIteratorPipeline,
    FlattenNesting,
    UseCheckedArithmetic,
    RemoveAllowAttribute,
    ReplaceDependency,
    RequiresHumanReview,
}

impl RepairHintClass {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Patch => "Patch",
            Self::UseIteratorPipeline => "UseIteratorPipeline",
            Self::FlattenNesting => "FlattenNesting",
            Self::UseCheckedArithmetic => "UseCheckedArithmetic",
            Self::RemoveAllowAttribute => "RemoveAllowAttribute",
            Self::ReplaceDependency => "ReplaceDependency",
            Self::RequiresHumanReview => "RequiresHumanReview",
        }
    }
}

impl RepairHint {
    #[must_use]
    pub const fn class(&self) -> RepairHintClass {
        match &self.0 {
            RepairHintInner::Patch { .. } => RepairHintClass::Patch,
            RepairHintInner::UseIteratorPipeline { .. } => RepairHintClass::UseIteratorPipeline,
            RepairHintInner::FlattenNesting { .. } => RepairHintClass::FlattenNesting,
            RepairHintInner::UseCheckedArithmetic { .. } => RepairHintClass::UseCheckedArithmetic,
            RepairHintInner::RemoveAllowAttribute { .. } => RepairHintClass::RemoveAllowAttribute,
            RepairHintInner::ReplaceDependency { .. } => RepairHintClass::ReplaceDependency,
            RepairHintInner::RequiresHumanReview { .. } => RepairHintClass::RequiresHumanReview,
        }
    }

    #[must_use]
    pub fn for_rule(rule_id: &str) -> Self {
        super::repair_catalog::for_rule(rule_id)
    }
}
