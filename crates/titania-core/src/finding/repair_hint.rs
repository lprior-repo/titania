use serde::{Deserialize, Serialize};

use crate::{error::RepairHintError, text_range::TextRange};

/// Machine-actionable repair suggestion for a [`crate::Finding`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "variant", rename_all = "snake_case")]
pub enum RepairHint {
    /// A byte-range patch to apply to a file.
    Patch { file: String, range: TextRange, replacement: String },
    /// Replace a loop with an iterator pipeline.
    UseIteratorPipeline { suggestion: String },
    /// Flatten deeply nested code.
    FlattenNesting { suggestion: String },
    /// Use checked arithmetic for the given operation.
    UseCheckedArithmetic { op: String },
    /// Remove an `#[allow(...)]` attribute.
    RemoveAllowAttribute { attr: String },
    /// Replace one dependency with another.
    ReplaceDependency { from: String, to: String },
    /// Requires manual review — no automatic fix is safe.
    RequiresHumanReview { note: String },
}

impl RepairHint {
    /// Construct a [`RepairHint::Patch`].
    ///
    /// # Errors
    /// - [`RepairHintError::EmptyRange`] if `range.width() == 0`.
    pub fn patch(
        file: String,
        range: TextRange,
        replacement: String,
    ) -> Result<Self, RepairHintError> {
        if range.width() == 0 {
            return Err(RepairHintError::EmptyRange);
        }
        Ok(Self::Patch { file, range, replacement })
    }

    /// Construct a [`RepairHint::UseIteratorPipeline`].
    #[must_use]
    pub const fn use_iterator_pipeline(suggestion: String) -> Self {
        Self::UseIteratorPipeline { suggestion }
    }

    /// Construct a [`RepairHint::FlattenNesting`].
    #[must_use]
    pub const fn flatten_nesting(suggestion: String) -> Self {
        Self::FlattenNesting { suggestion }
    }

    /// Construct a [`RepairHint::UseCheckedArithmetic`].
    #[must_use]
    pub const fn use_checked_arithmetic(op: String) -> Self {
        Self::UseCheckedArithmetic { op }
    }

    /// Construct a [`RepairHint::RemoveAllowAttribute`].
    #[must_use]
    pub const fn remove_allow_attribute(attr: String) -> Self {
        Self::RemoveAllowAttribute { attr }
    }

    /// Construct a [`RepairHint::ReplaceDependency`].
    #[must_use]
    pub const fn replace_dependency(from: String, to: String) -> Self {
        Self::ReplaceDependency { from, to }
    }

    /// Construct a [`RepairHint::RequiresHumanReview`].
    #[must_use]
    pub const fn requires_human_review(note: String) -> Self {
        Self::RequiresHumanReview { note }
    }

    /// Whether this hint can be applied automatically.
    #[must_use]
    pub const fn is_auto_applicable(&self) -> bool {
        matches!(self, Self::Patch { .. })
    }
}

#[derive(Deserialize)]
#[serde(tag = "variant", rename_all = "snake_case")]
enum RepairHintWire {
    Patch { file: String, range: TextRange, replacement: String },
    UseIteratorPipeline { suggestion: String },
    FlattenNesting { suggestion: String },
    UseCheckedArithmetic { op: String },
    RemoveAllowAttribute { attr: String },
    ReplaceDependency { from: String, to: String },
    RequiresHumanReview { note: String },
}

impl TryFrom<RepairHintWire> for RepairHint {
    type Error = RepairHintError;

    fn try_from(wire: RepairHintWire) -> Result<Self, Self::Error> {
        match wire {
            RepairHintWire::Patch { file, range, replacement } => {
                Self::patch(file, range, replacement)
            }
            RepairHintWire::UseIteratorPipeline { suggestion } => {
                Ok(Self::UseIteratorPipeline { suggestion })
            }
            RepairHintWire::FlattenNesting { suggestion } => {
                Ok(Self::FlattenNesting { suggestion })
            }
            RepairHintWire::UseCheckedArithmetic { op } => Ok(Self::UseCheckedArithmetic { op }),
            RepairHintWire::RemoveAllowAttribute { attr } => {
                Ok(Self::RemoveAllowAttribute { attr })
            }
            RepairHintWire::ReplaceDependency { from, to } => {
                Ok(Self::ReplaceDependency { from, to })
            }
            RepairHintWire::RequiresHumanReview { note } => Ok(Self::RequiresHumanReview { note }),
        }
    }
}

impl<'de> Deserialize<'de> for RepairHint {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        RepairHintWire::deserialize(de)?.try_into().map_err(serde::de::Error::custom)
    }
}
