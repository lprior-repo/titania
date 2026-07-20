//! `outcomes.json` domain types and parser behavior.

use serde::{Deserialize, Serialize};

use crate::error::MutantsOutcomesError;

use super::wire::{RawSpan, WireArtifact, parse_capped_wire};

/// Static upper bound on outcomes per `outcomes.json` file.
pub const MUTANTS_OUTCOMES_MAX_ENTRIES: usize = 1_000_000;

/// Typed `outcomes.json` contents.
///
/// Aggregate counts and timestamps are optional because cargo-mutants
/// revisions may add fields without removing the scenario list required
/// by the v1.5 lane.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MutantsOutcomes {
    /// Per-scenario entries (Baseline + one entry per mutant).
    pub outcomes: Vec<MutantOutcomeEntry>,
    /// Aggregate missed-mutant count (top-level `missed`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub missed: Option<u64>,
    /// Aggregate caught-mutant count (top-level `caught`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caught: Option<u64>,
    /// Aggregate timeout count (top-level `timeout`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    /// Aggregate unviable count (top-level `unviable`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unviable: Option<u64>,
    /// Aggregate success count (top-level `success`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success: Option<u64>,
    /// Aggregate total-mutant count (top-level `total_mutants`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_mutants: Option<u64>,
    /// cargo-mutants toolchain version (top-level `cargo_mutants_version`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo_mutants_version: Option<String>,
    /// Run start timestamp (top-level `start_time`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    /// Run end timestamp (top-level `end_time`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
}

impl MutantsOutcomes {
    /// Parse a cargo-mutants `outcomes.json` payload.
    ///
    /// `path` is a caller-provided diagnostic label, typically the
    /// artifact's on-disk path.
    ///
    /// # Errors
    /// - [`MutantsOutcomesError::OutcomesJsonParse`] when `contents`
    ///   is malformed JSON or has the wrong wire shape.
    /// - [`MutantsOutcomesError::TooManyOutcomes`] when the entry count
    ///   exceeds [`MUTANTS_OUTCOMES_MAX_ENTRIES`].
    pub fn parse_str(contents: &str, path: &str) -> Result<Self, MutantsOutcomesError> {
        parse_capped_wire(
            contents,
            path,
            MUTANTS_OUTCOMES_MAX_ENTRIES,
            WireArtifact::Outcomes,
            |outcomes: &Self| outcomes.outcomes.len(),
        )
    }

    /// Count of [`OutcomeSummary::MissedMutant`] entries.
    #[must_use]
    pub fn missed_count(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|entry| matches!(entry.summary, OutcomeSummary::MissedMutant))
            .count()
    }
}

/// One entry in the `outcomes` array of an `outcomes.json` payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutantOutcomeEntry {
    /// Scenario this entry describes (Baseline or one mutant).
    pub scenario: OutcomeScenario,
    /// Outcome class cargo-mutants assigned to the scenario.
    pub summary: OutcomeSummary,
    /// Path to the per-scenario log file, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_path: Option<String>,
    /// Path to the per-scenario diff file, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff_path: Option<String>,
}

/// Per-mutant scenario payload (`{"Mutant": {...}}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutantScenarioData {
    /// Human-readable cargo-mutants mutation name.
    pub name: String,
    /// Cargo package that owns the mutation.
    pub package: String,
    /// Workspace-relative source file the mutation touches.
    pub file: String,
    /// Source span the mutation targets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<RawSpan>,
    /// cargo-mutants genre tag, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    /// Textual replacement cargo-mutants would apply.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
}

/// Discriminator for the cargo-mutants `scenario` field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OutcomeScenario {
    /// Baseline marker. The literal remains a string for forward compatibility.
    Baseline(String),
    /// Per-mutant scenario and its metadata.
    Mutant {
        /// Metadata stored under cargo-mutants' `Mutant` map key.
        #[serde(rename = "Mutant")]
        mutant: MutantScenarioData,
    },
}

/// cargo-mutants `summary` discriminator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum OutcomeSummary {
    /// Baseline scenario compiled and tested cleanly.
    Success,
    /// A mutant built and passed all target tests.
    MissedMutant,
    /// A mutant broke the build before tests could run.
    Unviable,
    /// A mutant exceeded the per-scenario wall-clock timeout.
    Timeout,
    /// A mutant surfaced a generic test failure.
    Failure,
    /// Forward-compatible bucket preserving an unknown outcome literal.
    #[serde(untagged)]
    Other(String),
}
