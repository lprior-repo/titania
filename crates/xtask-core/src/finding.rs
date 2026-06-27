//! Finding types: what a lane reports when it detects a policy violation.

use serde::{Deserialize, Serialize};

use crate::location::Location;

/// A single quality-policy violation detected by a lane.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Finding {
    /// Which lane produced this finding.
    pub lane: crate::lane::Lane,
    /// The rule that was violated (e.g. `HOLZMAN_PANIC_UNWRAP`).
    pub rule_id: RuleId,
    /// Where the violation was found.
    pub location: Location,
    /// Human-readable explanation.
    pub message: String,
    /// Machine-readable repair guidance.
    pub repair: RepairHint,
    /// Whether this finding causes rejection or is advisory.
    pub effect: FindingEffect,
}

/// A typed rule identifier (e.g. `"HOLZMAN_PANIC_UNWRAP"`).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuleId(pub String);

/// Whether a finding rejects or is informational only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FindingEffect {
    /// Causes `CodeReject`.
    Reject,
    /// Advisory only; does not reject.
    Informational,
}

/// Byte-offset range over UTF-8 source bytes for deterministic patching.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextRange {
    pub start_byte: u32,
    pub end_byte: u32,
}

/// Machine-readable repair instructions for the AI repair loop.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RepairHint {
    /// Replace the byte range with this exact text.
    Patch {
        file: String,
        range: TextRange,
        replacement: String,
    },
    /// Replace the construct with an iterator pipeline.
    UseIteratorPipeline { suggestion: String },
    /// Decompose nested logic into named functions.
    FlattenNesting { suggestion: String },
    /// Replace raw arithmetic with checked variant.
    UseCheckedArithmetic { op: String },
    /// Remove a lint suppression attribute.
    RemoveAllowAttribute { attr: String },
    /// Replace a banned dependency.
    ReplaceDependency { from: String, to: String },
    /// No mechanical fix; requires human judgment.
    RequiresHumanReview { note: String },
}
