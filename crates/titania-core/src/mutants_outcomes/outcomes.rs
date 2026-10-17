use serde::{Deserialize, Serialize};

use crate::error::MutantsOutcomesError;

use super::wire::{RawSpan, WireArtifact, parse_capped_wire};

pub const MUTANTS_OUTCOMES_MAX_ENTRIES: usize = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MutantsOutcomes {
    pub outcomes: Vec<MutantOutcomeEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub missed: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caught: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unviable: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_mutants: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo_mutants_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
}

impl MutantsOutcomes {
    pub fn parse_str(contents: &str, path: &str) -> Result<Self, MutantsOutcomesError> {
        parse_capped_wire(
            contents,
            path,
            MUTANTS_OUTCOMES_MAX_ENTRIES,
            WireArtifact::Outcomes,
            |outcomes: &Self| outcomes.outcomes.len(),
        )
    }

    #[must_use]
    pub fn missed_count(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|entry| matches!(entry.summary, OutcomeSummary::MissedMutant))
            .count()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutantOutcomeEntry {
    pub scenario: OutcomeScenario,
    pub summary: OutcomeSummary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutantScenarioData {
    pub name: String,
    pub package: String,
    pub file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<RawSpan>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OutcomeScenario {
    Baseline(String),
    Mutant {
        #[serde(rename = "Mutant")]
        mutant: MutantScenarioData,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum OutcomeSummary {
    Success,
    MissedMutant,
    Unviable,
    Timeout,
    Failure,
    #[serde(untagged)]
    Other(String),
}
