//! Structured findings from lane analysis.
//!
//! A `Finding` records a single violation (or informational note) observed
//! during lane execution, together with where it occurred and how it should
//! be repaired.

use serde::{Deserialize, Serialize};

use crate::{
    error::{FindingError, LocationError, RepairHintError},
    lane::Lane,
    rule_id::RuleId,
    text_range::TextRange,
    workspace_path::WorkspacePath,
};

/// Whether a [`Finding`] causes the lane to reject or merely notes an issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingEffect {
    /// This finding must be resolved for the lane to pass.
    Reject,
    /// Informational only — the lane passes regardless.
    Informational,
}

/// Location where a finding was observed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "variant", rename_all = "snake_case")]
pub enum Location {
    /// A byte-offset span within a single source file.
    Span { file: WorkspacePath, line_start: u32, col_start: u32, line_end: u32, col_end: u32 },
    /// A dependency crate and its version.
    Dependency { crate_name: String, version: String },
    /// A manifest file (Cargo.toml, policy.toml, etc.).
    Manifest { file: WorkspacePath },
    /// A workspace-level observation (no file or crate context).
    Workspace,
    /// A tool or its version.
    Tool { name: String, version: String },
}

impl Location {
    /// Construct a [`Location::Span`].
    ///
    /// # Errors
    /// - [`LocationError::LineStartBeforeOne`] if `line_start < 1`.
    /// - [`LocationError::EndBeforeStart`] if `line_end < line_start`.
    /// - [`LocationError::ColEndBeforeStart`] if `col_end < col_start`.
    ///
    /// Line numbers are 1-based; column numbers are 0-based Unicode scalar
    /// values.
    pub fn span(
        file: WorkspacePath,
        line_start: u32,
        col_start: u32,
        line_end: u32,
        col_end: u32,
    ) -> Result<Self, LocationError> {
        if line_start < 1 {
            return Err(LocationError::LineStartBeforeOne);
        }
        if line_end < line_start {
            return Err(LocationError::EndBeforeStart { line_start, line_end });
        }
        if col_end < col_start {
            return Err(LocationError::ColEndBeforeStart { col_start, col_end });
        }
        Ok(Self::Span { file, line_start, col_start, line_end, col_end })
    }
    /// Construct a [`Location::Dependency`].
    #[must_use]
    pub const fn dependency(crate_name: String, version: String) -> Self {
        Self::Dependency { crate_name, version }
    }

    /// Construct a [`Location::Manifest`].
    #[must_use]
    pub const fn manifest(file: WorkspacePath) -> Self {
        Self::Manifest { file }
    }

    /// Construct a [`Location::Workspace`].
    #[must_use]
    pub const fn workspace() -> Self {
        Self::Workspace
    }

    /// Construct a [`Location::Tool`].
    #[must_use]
    pub const fn tool(name: String, version: String) -> Self {
        Self::Tool { name, version }
    }

    /// Whether this location is a file span.
    #[must_use]
    pub const fn is_span(&self) -> bool {
        matches!(self, Self::Span { .. })
    }

    /// If this location is a span, return the workspace path.
    #[must_use]
    pub const fn span_file(&self) -> Option<&WorkspacePath> {
        match self {
            Self::Span { file, .. } => Some(file),
            _ => None,
        }
    }
    /// Return the span file or a typed error if this is not a span location.
    ///
    /// # Errors
    /// Returns [`LocationError::NotSpan`] for non-span variants.
    pub const fn as_file(&self) -> Result<&WorkspacePath, LocationError> {
        match self {
            Self::Span { file, .. } => Ok(file),
            _ => Err(LocationError::NotSpan),
        }
    }

    /// Return the span start line, or `0` for non-span locations.
    #[must_use]
    pub const fn line_start(&self) -> u32 {
        match self {
            Self::Span { line_start, .. } => *line_start,
            _ => 0,
        }
    }

    /// Return the span start column, or `0` for non-span locations.
    #[must_use]
    pub const fn col_start(&self) -> u32 {
        match self {
            Self::Span { col_start, .. } => *col_start,
            _ => 0,
        }
    }

    /// Return the span end line, or `0` for non-span locations.
    #[must_use]
    pub const fn line_end(&self) -> u32 {
        match self {
            Self::Span { line_end, .. } => *line_end,
            _ => 0,
        }
    }

    /// Return the span end column, or `0` for non-span locations.
    #[must_use]
    pub const fn col_end(&self) -> u32 {
        match self {
            Self::Span { col_end, .. } => *col_end,
            _ => 0,
        }
    }
}

/// Machine-actionable repair suggestion for a [`Finding`].
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
enum LocationWire {
    Span { file: WorkspacePath, line_start: u32, col_start: u32, line_end: u32, col_end: u32 },
    Dependency { crate_name: String, version: String },
    Manifest { file: WorkspacePath },
    Workspace,
    Tool { name: String, version: String },
}

impl TryFrom<LocationWire> for Location {
    type Error = LocationError;

    fn try_from(wire: LocationWire) -> Result<Self, Self::Error> {
        match wire {
            LocationWire::Span { file, line_start, col_start, line_end, col_end } => {
                Self::span(file, line_start, col_start, line_end, col_end)
            }
            LocationWire::Dependency { crate_name, version } => {
                Ok(Self::Dependency { crate_name, version })
            }
            LocationWire::Manifest { file } => Ok(Self::Manifest { file }),
            LocationWire::Workspace => Ok(Self::Workspace),
            LocationWire::Tool { name, version } => Ok(Self::Tool { name, version }),
        }
    }
}

impl<'de> Deserialize<'de> for Location {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        LocationWire::deserialize(de)?.try_into().map_err(serde::de::Error::custom)
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
