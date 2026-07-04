//! Repair hint domain type and serde wire adapters.
//!
//! `RepairHint` derives `Serialize` for production wire output. Deserialization
//! uses a private `RepairHintReadWire` intermediate and smart-constructors so
//! that validation (patch range) runs on every deserialize path.

use serde::{Deserialize, Serialize};

use crate::{error::RepairHintError, text_range::TextRange};

/// Machine-actionable repair suggestion for a [`super::Finding`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "variant", rename_all = "snake_case")]
pub enum RepairHint {
    /// A byte-range patch to apply to a file.
    Patch {
        /// Workspace-relative file to patch.
        file: String,
        /// Byte range to replace.
        range: TextRange,
        /// Replacement text for the byte range.
        replacement: String,
    },
    /// Replace a loop with an iterator pipeline.
    UseIteratorPipeline {
        /// Human-readable iterator replacement suggestion.
        suggestion: String,
    },
    /// Flatten deeply nested code.
    FlattenNesting {
        /// Human-readable nesting reduction suggestion.
        suggestion: String,
    },
    /// Use checked arithmetic for the given operation.
    UseCheckedArithmetic {
        /// Arithmetic operation that needs checked handling.
        op: String,
    },
    /// Remove an `#[allow(...)]` attribute.
    RemoveAllowAttribute {
        /// Attribute text that should be removed.
        attr: String,
    },
    /// Replace one dependency with another.
    ReplaceDependency {
        /// Dependency name currently in use.
        from: String,
        /// Replacement dependency name.
        to: String,
    },
    /// Requires manual review — no automatic fix is safe.
    RequiresHumanReview {
        /// Human-readable explanation of the manual review requirement.
        note: String,
    },
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
        validate_patch_range(range)?;
        Ok(Self::Patch { file, range, replacement })
    }

    /// Whether this hint can be applied automatically.
    #[must_use]
    pub const fn is_auto_applicable(&self) -> bool {
        matches!(self, Self::Patch { .. })
    }
}

/// Validate that a patch range is non-empty.
///
/// # Errors
/// - [`RepairHintError::EmptyRange`] when `range.width() == 0`.
const fn validate_patch_range(range: TextRange) -> Result<(), RepairHintError> {
    if range.width() == 0 {
        return Err(RepairHintError::EmptyRange);
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(tag = "variant", rename_all = "snake_case", deny_unknown_fields)]
enum RepairHintReadWire {
    Patch { file: String, range: TextRange, replacement: String },
    UseIteratorPipeline { suggestion: String },
    FlattenNesting { suggestion: String },
    UseCheckedArithmetic { op: String },
    RemoveAllowAttribute { attr: String },
    ReplaceDependency { from: String, to: String },
    RequiresHumanReview { note: String },
}

/// Convert deserialized repair-hint wire data into a validated domain hint.
///
/// # Errors
/// Returns [`RepairHintError`] when a patch range is invalid.
fn repair_hint_from_wire(wire: RepairHintReadWire) -> Result<RepairHint, RepairHintError> {
    match wire {
        RepairHintReadWire::Patch { file, range, replacement } => {
            validate_patch_range(range).map(|()| RepairHint::Patch { file, range, replacement })
        }
        RepairHintReadWire::UseIteratorPipeline { suggestion } => {
            Ok(RepairHint::UseIteratorPipeline { suggestion })
        }
        RepairHintReadWire::FlattenNesting { suggestion } => {
            Ok(RepairHint::FlattenNesting { suggestion })
        }
        RepairHintReadWire::UseCheckedArithmetic { op } => {
            Ok(RepairHint::UseCheckedArithmetic { op })
        }
        RepairHintReadWire::RemoveAllowAttribute { attr } => {
            Ok(RepairHint::RemoveAllowAttribute { attr })
        }
        RepairHintReadWire::ReplaceDependency { from, to } => {
            Ok(RepairHint::ReplaceDependency { from, to })
        }
        RepairHintReadWire::RequiresHumanReview { note } => {
            Ok(RepairHint::RequiresHumanReview { note })
        }
    }
}

impl TryFrom<RepairHintReadWire> for RepairHint {
    type Error = RepairHintError;

    fn try_from(wire: RepairHintReadWire) -> Result<Self, Self::Error> {
        repair_hint_from_wire(wire)
    }
}

impl<'de> Deserialize<'de> for RepairHint {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = RepairHintReadWire::deserialize(deserializer)?;
        repair_hint_from_wire(wire).map_err(serde::de::Error::custom)
    }
}
