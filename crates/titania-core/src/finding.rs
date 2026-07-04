//! Structured findings from lane analysis.
//!
//! A `Finding` records a single violation (or informational note) observed
//! during lane execution, together with where it occurred and how it should
//! be repaired.

#![expect(
    clippy::excessive_nesting,
    reason = "Manual serde adapters keep wire-shape validation adjacent to each variant arm."
)]
#![expect(
    clippy::too_many_lines,
    reason = "Manual repair-hint deserialization enumerates all wire variants in one serde entry point."
)]

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
    Span {
        /// Workspace-relative source file path containing the finding.
        file: WorkspacePath,
        /// One-based first line of the finding span.
        line_start: u32,
        /// Zero-based first column of the finding span.
        col_start: u32,
        /// One-based last line of the finding span.
        line_end: u32,
        /// Zero-based last column of the finding span.
        col_end: u32,
    },
    /// A dependency crate and its version.
    Dependency {
        /// Dependency package name.
        crate_name: String,
        /// Dependency version observed by the lane.
        version: String,
    },
    /// A manifest file (Cargo.toml, policy.toml, etc.).
    Manifest {
        /// Workspace-relative manifest path.
        file: WorkspacePath,
    },
    /// A workspace-level observation (no file or crate context).
    Workspace,
    /// A tool or its version.
    Tool {
        /// Tool executable or logical tool name.
        name: String,
        /// Tool version string reported by the lane.
        version: String,
    },
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
}

/// Machine-actionable repair suggestion for a [`Finding`].
#[derive(Debug, Clone, PartialEq, Eq)]
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
        if range.width() == 0 {
            return Err(RepairHintError::EmptyRange);
        }
        Ok(Self::Patch { file, range, replacement })
    }

    /// Whether this hint can be applied automatically.
    #[must_use]
    pub const fn is_auto_applicable(&self) -> bool {
        matches!(self, Self::Patch { .. })
    }
}

// Custom serialization for Location (serde derive doesn't work with
// enum variants that have different fields).

impl Serialize for Location {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Span { file, line_start, col_start, line_end, col_end } => {
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
            Self::Dependency { crate_name, version } => {
                use serde::ser::SerializeStruct;
                let mut s = ser.serialize_struct("Location", 3)?;
                s.serialize_field("variant", "dependency")?;
                s.serialize_field("crate_name", crate_name)?;
                s.serialize_field("version", version)?;
                s.end()
            }
            Self::Manifest { file } => {
                use serde::ser::SerializeStruct;
                let mut s = ser.serialize_struct("Location", 2)?;
                s.serialize_field("variant", "manifest")?;
                s.serialize_field("file", file)?;
                s.end()
            }
            Self::Workspace => {
                use serde::ser::SerializeStruct;
                let mut s = ser.serialize_struct("Location", 1)?;
                s.serialize_field("variant", "workspace")?;
                s.end()
            }
            Self::Tool { name, version } => {
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
                Ok(Self::Dependency { crate_name: dep.crate_name, version: dep.version })
            }
            "manifest" => {
                #[derive(Deserialize)]
                struct ManifestWire {
                    file: WorkspacePath,
                }
                let m = ManifestWire::deserialize(value).map_err(serde::de::Error::custom)?;
                Ok(Self::Manifest { file: m.file })
            }
            "workspace" => Ok(Self::Workspace),
            "tool" => {
                #[derive(Deserialize)]
                struct ToolWire {
                    name: String,
                    version: String,
                }
                let t = ToolWire::deserialize(value).map_err(serde::de::Error::custom)?;
                Ok(Self::Tool { name: t.name, version: t.version })
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
            Self::Patch { file, range, replacement } => {
                use serde::ser::SerializeStruct;
                let mut s = ser.serialize_struct("RepairHint", 4)?;
                s.serialize_field("variant", "patch")?;
                s.serialize_field("file", file)?;
                s.serialize_field("range", range)?;
                s.serialize_field("replacement", replacement)?;
                s.end()
            }
            Self::UseIteratorPipeline { suggestion } => {
                use serde::ser::SerializeStruct;
                let mut s = ser.serialize_struct("RepairHint", 2)?;
                s.serialize_field("variant", "use_iterator_pipeline")?;
                s.serialize_field("suggestion", suggestion)?;
                s.end()
            }
            Self::FlattenNesting { suggestion } => {
                use serde::ser::SerializeStruct;
                let mut s = ser.serialize_struct("RepairHint", 2)?;
                s.serialize_field("variant", "flatten_nesting")?;
                s.serialize_field("suggestion", suggestion)?;
                s.end()
            }
            Self::UseCheckedArithmetic { op } => {
                use serde::ser::SerializeStruct;
                let mut s = ser.serialize_struct("RepairHint", 2)?;
                s.serialize_field("variant", "use_checked_arithmetic")?;
                s.serialize_field("op", op)?;
                s.end()
            }
            Self::RemoveAllowAttribute { attr } => {
                use serde::ser::SerializeStruct;
                let mut s = ser.serialize_struct("RepairHint", 2)?;
                s.serialize_field("variant", "remove_allow_attribute")?;
                s.serialize_field("attr", attr)?;
                s.end()
            }
            Self::ReplaceDependency { from, to } => {
                use serde::ser::SerializeStruct;
                let mut s = ser.serialize_struct("RepairHint", 3)?;
                s.serialize_field("variant", "replace_dependency")?;
                s.serialize_field("from", from)?;
                s.serialize_field("to", to)?;
                s.end()
            }
            Self::RequiresHumanReview { note } => {
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
                Ok(Self::UseIteratorPipeline { suggestion: s.suggestion })
            }
            "flatten_nesting" => {
                #[derive(Deserialize)]
                struct SuggWire {
                    suggestion: String,
                }
                let s = SuggWire::deserialize(value).map_err(serde::de::Error::custom)?;
                Ok(Self::FlattenNesting { suggestion: s.suggestion })
            }
            "use_checked_arithmetic" => {
                #[derive(Deserialize)]
                struct OpWire {
                    op: String,
                }
                let o = OpWire::deserialize(value).map_err(serde::de::Error::custom)?;
                Ok(Self::UseCheckedArithmetic { op: o.op })
            }
            "remove_allow_attribute" => {
                #[derive(Deserialize)]
                struct AttrWire {
                    attr: String,
                }
                let a = AttrWire::deserialize(value).map_err(serde::de::Error::custom)?;
                Ok(Self::RemoveAllowAttribute { attr: a.attr })
            }
            "replace_dependency" => {
                #[derive(Deserialize)]
                struct DepWire {
                    from: String,
                    to: String,
                }
                let d = DepWire::deserialize(value).map_err(serde::de::Error::custom)?;
                Ok(Self::ReplaceDependency { from: d.from, to: d.to })
            }
            "requires_human_review" => {
                #[derive(Deserialize)]
                struct NoteWire {
                    note: String,
                }
                let n = NoteWire::deserialize(value).map_err(serde::de::Error::custom)?;
                Ok(Self::RequiresHumanReview { note: n.note })
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
    #[expect(
        clippy::too_many_arguments,
        reason = "Finding is the public aggregate constructor for six independent v1 finding fields."
    )]
    #[expect(
        clippy::unnecessary_wraps,
        reason = "The Result return preserves the validated-constructor API shared by fallible domain primitives."
    )]
    /// Construct a [`Finding`].
    ///
    /// # Errors
    /// - Any error from validating the [`RuleId`], [`Location`], or
    ///   [`RepairHint`].
    pub const fn new(
        lane: Lane,
        rule_id: RuleId,
        location: Location,
        message: String,
        repair: RepairHint,
        effect: FindingEffect,
    ) -> Result<Self, FindingError> {
        Ok(Self { lane, rule_id, location, message, repair, effect })
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
