//! Repair hint domain type and serde wire adapters.
//!
//! `RepairHint` is a public newtype wrapper over a private `RepairHintInner` enum.
//! All construction goes through validated constructors (`RepairHint::patch`, etc.).
//! Direct variant construction is impossible because `RepairHintInner` is private.
//!
//! Serde deserialization uses a private `RepairHintReadWire` intermediate so that
//! validation runs on every deserialize path.

use serde::{Deserialize, Serialize};

use crate::text_range::TextRange;

// ── Private inner enum ──────────────────────────────────────────────────────

/// Private inner type for [`RepairHint`].
///
/// Sealed so external crates cannot construct variants directly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
enum RepairHintInner {
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

impl RepairHintInner {
    const fn is_patch(&self) -> bool {
        matches!(self, Self::Patch { .. })
    }
}

// ── Public newtype ──────────────────────────────────────────────────────────

/// Machine-actionable repair suggestion for a [`super::Finding`].
///
/// This is a newtype wrapper over a private inner enum. All construction
/// goes through smart constructors that enforce invariants.
///
/// # Serialization
///
/// `RepairHint` derives `Serialize` for production wire output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct RepairHint(RepairHintInner);

impl RepairHint {
    /// Construct a [`RepairHint`] that proposes an auto-applicable text patch over a [`TextRange`].
    ///
    /// The range is half-open `[start_byte, end_byte)`. A zero-width range
    /// (`start_byte == end_byte`) is a valid insertion patch: `replacement`
    /// is inserted at `start_byte` without consuming any bytes of the
    /// source file. Range *bounds* (`start_byte <= end_byte`) are enforced
    /// by [`TextRange::new`].
    #[must_use]
    pub const fn patch(file: String, range: TextRange, replacement: String) -> Self {
        Self(RepairHintInner::Patch { file, range, replacement })
    }

    /// Construct a [`RepairHint`] suggesting an iterator-pipeline rewrite.
    #[must_use]
    pub const fn use_iterator_pipeline(suggestion: String) -> Self {
        Self(RepairHintInner::UseIteratorPipeline { suggestion })
    }

    /// Construct a [`RepairHint`] suggesting flattening nested option/result levels.
    #[must_use]
    pub const fn flatten_nesting(suggestion: String) -> Self {
        Self(RepairHintInner::FlattenNesting { suggestion })
    }

    /// Construct a [`RepairHint`] suggesting checked arithmetic to avoid overflow.
    #[must_use]
    pub const fn use_checked_arithmetic(op: String) -> Self {
        Self(RepairHintInner::UseCheckedArithmetic { op })
    }

    /// Construct a [`RepairHint`] suggesting removal of a `#[allow(...)]` attribute.
    #[must_use]
    pub const fn remove_allow_attribute(attr: String) -> Self {
        Self(RepairHintInner::RemoveAllowAttribute { attr })
    }

    /// Construct a [`RepairHint`] suggesting a dependency version or source replacement.
    #[must_use]
    pub const fn replace_dependency(from: String, to: String) -> Self {
        Self(RepairHintInner::ReplaceDependency { from, to })
    }

    /// Construct a [`RepairHint`] flagging the finding for manual human review.
    #[must_use]
    pub const fn requires_human_review(note: String) -> Self {
        Self(RepairHintInner::RequiresHumanReview { note })
    }

    /// Whether this hint can be applied automatically.
    #[must_use]
    pub const fn is_auto_applicable(&self) -> bool {
        self.0.is_patch()
    }
}

// ── Wire deserialization ────────────────────────────────────────────────────

/// Intermediate wire representation for [`RepairHint`] deserialization.
///
/// Private — external crates cannot construct or inspect variants directly.
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

/// Construct a [`RepairHint`] that proposes an auto-applicable text patch over a [`TextRange`].
///
/// Zero-width ranges are valid insertion patches; range bounds are
/// enforced upstream by [`TextRange::new`].
const fn construct_patch(file: String, range: TextRange, replacement: String) -> RepairHint {
    RepairHint(RepairHintInner::Patch { file, range, replacement })
}

/// Construct a [`RepairHint`] suggesting an iterator-pipeline rewrite.
const fn iterator_pipeline(suggestion: String) -> RepairHint {
    RepairHint(RepairHintInner::UseIteratorPipeline { suggestion })
}

/// Construct a [`RepairHint`] suggesting flattening nested option/result levels.
const fn flatten_nesting(suggestion: String) -> RepairHint {
    RepairHint(RepairHintInner::FlattenNesting { suggestion })
}

/// Construct a [`RepairHint`] suggesting checked arithmetic to avoid overflow.
const fn checked_arithmetic(op: String) -> RepairHint {
    RepairHint(RepairHintInner::UseCheckedArithmetic { op })
}

/// Construct a [`RepairHint`] suggesting removal of a `#[allow(...)]` attribute.
const fn remove_allow(attr: String) -> RepairHint {
    RepairHint(RepairHintInner::RemoveAllowAttribute { attr })
}

/// Construct a [`RepairHint`] suggesting a dependency version or source replacement.
const fn replace_dependency(from: String, to: String) -> RepairHint {
    RepairHint(RepairHintInner::ReplaceDependency { from, to })
}

/// Construct a [`RepairHint`] flagging the finding for manual human review.
const fn human_review(note: String) -> RepairHint {
    RepairHint(RepairHintInner::RequiresHumanReview { note })
}

/// Convert deserialized repair-hint wire data into a validated domain hint.
///
/// Zero-width ranges are accepted for the `Patch` variant (insertion
/// patches). Range bounds are enforced upstream by [`TextRange::new`].
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

/// Deserialize a [`RepairHint`] from wire format.
///
/// # Errors
/// Propagates serde deserialization errors.
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

// ── RepairHintClass ─────────────────────────────────────────────────────────

/// Stable classification of a [`RepairHint`].
///
/// Seven-variant enum mirroring [`RepairHintInner`] so that [`RepairHint::class`]
/// is exhaustive. The strings returned by [`RepairHintClass::as_str`] are the
/// same literals used in `repair_catalog.tsv` column 4 (catalog rows use
/// `RequiresHumanReview` for the informational `—` marker).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RepairHintClass {
    /// Auto-applicable byte-range patch.
    Patch,
    /// Suggest an iterator-pipeline rewrite.
    UseIteratorPipeline,
    /// Suggest flattening nested control flow.
    FlattenNesting,
    /// Suggest checked arithmetic.
    UseCheckedArithmetic,
    /// Suggest removing a `#[allow(...)]` attribute.
    RemoveAllowAttribute,
    /// Suggest rotating one dependency for another.
    ReplaceDependency,
    /// Hand off to manual review.
    RequiresHumanReview,
}

impl RepairHintClass {
    /// Stable TSV-string equivalent. Used by parity tests in
    /// `titania-output` and `titania-lanes` to compare catalog classes.
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
    /// Exhaustive class lookup. Adding a new [`RepairHintInner`] variant
    /// will fail this match until a class arm is added.
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

    /// Look up the canonical [`RepairHint`] for a rule id via the catalog
    /// in `repair_catalog.tsv`. Thin re-export of
    /// [`super::repair_catalog::for_rule`] so callers can write
    /// `RepairHint::for_rule(rule_id)` without importing the parser module.
    ///
    /// **Contract**:
    /// - Empty `rule_id` → `requires_human_review("unmapped rule_id: ")`.
    /// - Unknown / dynamic / out-of-catalog id →
    ///   `requires_human_review("unmapped rule_id: <id>")`.
    /// - Never panics.
    /// - Never returns `RepairHint::Patch` (no range context available
    ///   here — call sites that need a precise Patch must use
    ///   `titania_lanes::Finding::with_repair`).
    #[must_use]
    pub fn for_rule(rule_id: &str) -> Self {
        super::repair_catalog::for_rule(rule_id)
    }
}
