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
#[derive(Debug, Clone, PartialEq, Eq)]
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
        Ok(Location::Span { file, line_start, col_start, line_end, col_end })
    }

    /// Whether this location is a file span.
    #[must_use]
    pub fn is_span(&self) -> bool {
        matches!(self, Location::Span { .. })
    }

    /// If this location is a span, return the workspace path.
    #[must_use]
    pub fn span_file(&self) -> Option<&WorkspacePath> {
        match self {
            Location::Span { file, .. } => Some(file),
            _ => None,
        }
    }
}

/// Machine-actionable repair suggestion for a [`Finding`].
#[derive(Debug, Clone, PartialEq, Eq)]
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
        Ok(RepairHint::Patch { file, range, replacement })
    }

    /// Whether this hint can be applied automatically.
    #[must_use]
    pub fn is_auto_applicable(&self) -> bool {
        matches!(self, RepairHint::Patch { .. })
    }
}

// Custom serialization for Location (serde derive doesn't work with
// enum variants that have different fields).

impl Serialize for Location {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        match self {
            Location::Span { file, line_start, col_start, line_end, col_end } => {
                use serde::ser::SerializeStruct;
                let mut s = ser.serialize_struct("Location", 5)?;
                s.serialize_field("variant", "span")?;
                s.serialize_field("file", file)?;
                s.serialize_field("line_start", line_start)?;
                s.serialize_field("col_start", col_start)?;
                s.serialize_field("line_end", line_end)?;
                s.serialize_field("col_end", col_end)?;
                s.end()
            }
            Location::Dependency { crate_name, version } => {
                use serde::ser::SerializeStruct;
                let mut s = ser.serialize_struct("Location", 3)?;
                s.serialize_field("variant", "dependency")?;
                s.serialize_field("crate_name", crate_name)?;
                s.serialize_field("version", version)?;
                s.end()
            }
            Location::Manifest { file } => {
                use serde::ser::SerializeStruct;
                let mut s = ser.serialize_struct("Location", 2)?;
                s.serialize_field("variant", "manifest")?;
                s.serialize_field("file", file)?;
                s.end()
            }
            Location::Workspace => {
                use serde::ser::SerializeStruct;
                let mut s = ser.serialize_struct("Location", 1)?;
                s.serialize_field("variant", "workspace")?;
                s.end()
            }
            Location::Tool { name, version } => {
                use serde::ser::SerializeStruct;
                let mut s = ser.serialize_struct("Location", 3)?;
                s.serialize_field("variant", "tool")?;
                s.serialize_field("name", name)?;
                s.serialize_field("version", version)?;
                s.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for Location {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        // Deserialize to JSON value first, inspect variant, then
        // deserialize into the appropriate wire struct.
        let value = serde_json::Value::deserialize(de)?;
        let variant = value
            .get("variant")
            .and_then(|v| v.as_str())
            .ok_or_else(|| serde::de::Error::missing_field("variant"))?;

        match variant {
            "span" => {
                #[derive(Deserialize)]
                struct SpanWire {
                    file: WorkspacePath,
                    line_start: u32,
                    col_start: u32,
                    line_end: u32,
                    col_end: u32,
                }
                let span = SpanWire::deserialize(value).map_err(serde::de::Error::custom)?;
                Self::span(span.file, span.line_start, span.col_start, span.line_end, span.col_end)
                    .map_err(serde::de::Error::custom)
            }
            "dependency" => {
                #[derive(Deserialize)]
                struct DepWire {
                    crate_name: String,
                    version: String,
                }
                let dep = DepWire::deserialize(value).map_err(serde::de::Error::custom)?;
                Ok(Location::Dependency { crate_name: dep.crate_name, version: dep.version })
            }
            "manifest" => {
                #[derive(Deserialize)]
                struct ManifestWire {
                    file: WorkspacePath,
                }
                let m = ManifestWire::deserialize(value).map_err(serde::de::Error::custom)?;
                Ok(Location::Manifest { file: m.file })
            }
            "workspace" => Ok(Location::Workspace),
            "tool" => {
                #[derive(Deserialize)]
                struct ToolWire {
                    name: String,
                    version: String,
                }
                let t = ToolWire::deserialize(value).map_err(serde::de::Error::custom)?;
                Ok(Location::Tool { name: t.name, version: t.version })
            }
            other => Err(serde::de::Error::unknown_variant(
                other,
                &["span", "dependency", "manifest", "workspace", "tool"],
            )),
        }
    }
}

impl Serialize for RepairHint {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        match self {
            RepairHint::Patch { file, range, replacement } => {
                use serde::ser::SerializeStruct;
                let mut s = ser.serialize_struct("RepairHint", 4)?;
                s.serialize_field("variant", "patch")?;
                s.serialize_field("file", file)?;
                s.serialize_field("range", range)?;
                s.serialize_field("replacement", replacement)?;
                s.end()
            }
            RepairHint::UseIteratorPipeline { suggestion } => {
                use serde::ser::SerializeStruct;
                let mut s = ser.serialize_struct("RepairHint", 2)?;
                s.serialize_field("variant", "use_iterator_pipeline")?;
                s.serialize_field("suggestion", suggestion)?;
                s.end()
            }
            RepairHint::FlattenNesting { suggestion } => {
                use serde::ser::SerializeStruct;
                let mut s = ser.serialize_struct("RepairHint", 2)?;
                s.serialize_field("variant", "flatten_nesting")?;
                s.serialize_field("suggestion", suggestion)?;
                s.end()
            }
            RepairHint::UseCheckedArithmetic { op } => {
                use serde::ser::SerializeStruct;
                let mut s = ser.serialize_struct("RepairHint", 2)?;
                s.serialize_field("variant", "use_checked_arithmetic")?;
                s.serialize_field("op", op)?;
                s.end()
            }
            RepairHint::RemoveAllowAttribute { attr } => {
                use serde::ser::SerializeStruct;
                let mut s = ser.serialize_struct("RepairHint", 2)?;
                s.serialize_field("variant", "remove_allow_attribute")?;
                s.serialize_field("attr", attr)?;
                s.end()
            }
            RepairHint::ReplaceDependency { from, to } => {
                use serde::ser::SerializeStruct;
                let mut s = ser.serialize_struct("RepairHint", 3)?;
                s.serialize_field("variant", "replace_dependency")?;
                s.serialize_field("from", from)?;
                s.serialize_field("to", to)?;
                s.end()
            }
            RepairHint::RequiresHumanReview { note } => {
                use serde::ser::SerializeStruct;
                let mut s = ser.serialize_struct("RepairHint", 2)?;
                s.serialize_field("variant", "requires_human_review")?;
                s.serialize_field("note", note)?;
                s.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for RepairHint {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(de)?;
        let variant = value
            .get("variant")
            .and_then(|v| v.as_str())
            .ok_or_else(|| serde::de::Error::missing_field("variant"))?;

        match variant {
            "patch" => {
                #[derive(Deserialize)]
                struct PatchWire {
                    file: String,
                    range: TextRange,
                    replacement: String,
                }
                let p = PatchWire::deserialize(value).map_err(serde::de::Error::custom)?;
                Self::patch(p.file, p.range, p.replacement).map_err(serde::de::Error::custom)
            }
            "use_iterator_pipeline" => {
                #[derive(Deserialize)]
                struct SuggWire {
                    suggestion: String,
                }
                let s = SuggWire::deserialize(value).map_err(serde::de::Error::custom)?;
                Ok(RepairHint::UseIteratorPipeline { suggestion: s.suggestion })
            }
            "flatten_nesting" => {
                #[derive(Deserialize)]
                struct SuggWire {
                    suggestion: String,
                }
                let s = SuggWire::deserialize(value).map_err(serde::de::Error::custom)?;
                Ok(RepairHint::FlattenNesting { suggestion: s.suggestion })
            }
            "use_checked_arithmetic" => {
                #[derive(Deserialize)]
                struct OpWire {
                    op: String,
                }
                let o = OpWire::deserialize(value).map_err(serde::de::Error::custom)?;
                Ok(RepairHint::UseCheckedArithmetic { op: o.op })
            }
            "remove_allow_attribute" => {
                #[derive(Deserialize)]
                struct AttrWire {
                    attr: String,
                }
                let a = AttrWire::deserialize(value).map_err(serde::de::Error::custom)?;
                Ok(RepairHint::RemoveAllowAttribute { attr: a.attr })
            }
            "replace_dependency" => {
                #[derive(Deserialize)]
                struct DepWire {
                    from: String,
                    to: String,
                }
                let d = DepWire::deserialize(value).map_err(serde::de::Error::custom)?;
                Ok(RepairHint::ReplaceDependency { from: d.from, to: d.to })
            }
            "requires_human_review" => {
                #[derive(Deserialize)]
                struct NoteWire {
                    note: String,
                }
                let n = NoteWire::deserialize(value).map_err(serde::de::Error::custom)?;
                Ok(RepairHint::RequiresHumanReview { note: n.note })
            }
            other => Err(serde::de::Error::unknown_variant(
                other,
                &[
                    "patch",
                    "use_iterator_pipeline",
                    "flatten_nesting",
                    "use_checked_arithmetic",
                    "remove_allow_attribute",
                    "replace_dependency",
                    "requires_human_review",
                ],
            )),
        }
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
    /// - Any error from validating the [`RuleId`], [`Location`], or
    ///   [`RepairHint`].
    pub fn new(
        lane: Lane,
        rule_id: RuleId,
        location: Location,
        message: String,
        repair: RepairHint,
        effect: FindingEffect,
    ) -> Result<Self, FindingError> {
        Ok(Finding { lane, rule_id, location, message, repair, effect })
    }

    #[must_use]
    pub fn lane(&self) -> Lane {
        self.lane
    }

    #[must_use]
    pub fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    #[must_use]
    pub fn location(&self) -> &Location {
        &self.location
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    #[must_use]
    pub fn repair(&self) -> &RepairHint {
        &self.repair
    }

    #[must_use]
    pub fn effect(&self) -> FindingEffect {
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
    pub fn has_auto_repair(&self) -> bool {
        self.repair.is_auto_applicable()
    }
}
