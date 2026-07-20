//! Shared cargo-mutants wire types and capped JSON parsing.

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::error::MutantsOutcomesError;

/// Source-file `(line, column)` point.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawPoint {
    /// 1-based line offset.
    pub line: u32,
    /// 1-based column offset.
    pub column: u32,
}

/// cargo-mutants source span (start/end points).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawSpan {
    /// Start point required to build a typed mutant identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<RawPoint>,
    /// End point, which cargo-mutants may omit for single-line mutations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<RawPoint>,
}

/// Artifact class selecting the precise typed parse and cap errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WireArtifact {
    /// `outcomes.json` object.
    Outcomes,
    /// `mutants.json` record list.
    Records,
}

/// Deserialize one cargo-mutants artifact and enforce its entry cap.
///
/// # Errors
/// Returns the artifact-specific parse error when serde rejects the
/// input, or the artifact-specific cap error when `entry_count` exceeds
/// `max_entries`.
pub(super) fn parse_capped_wire<T, F>(
    contents: &str,
    path: &str,
    max_entries: usize,
    artifact: WireArtifact,
    entry_count: F,
) -> Result<T, MutantsOutcomesError>
where
    T: DeserializeOwned,
    F: FnOnce(&T) -> usize,
{
    let parsed =
        serde_json::from_str(contents).map_err(|error| artifact.parse_error(path, &error))?;
    let found = entry_count(&parsed);
    artifact.reject_excess(found, max_entries, path)?;
    Ok(parsed)
}

impl WireArtifact {
    fn parse_error(self, path: &str, error: &serde_json::Error) -> MutantsOutcomesError {
        let reason = error.to_string().into_boxed_str();
        match self {
            Self::Outcomes => outcomes_parse_error(path, reason),
            Self::Records => records_parse_error(path, reason),
        }
    }

    /// Enforce the selected artifact's static entry cap.
    ///
    /// # Errors
    /// Returns the artifact-specific cap error when `found` exceeds
    /// `max`.
    fn reject_excess(
        self,
        found: usize,
        max: usize,
        path: &str,
    ) -> Result<(), MutantsOutcomesError> {
        (found <= max).then_some(()).ok_or_else(|| self.limit_error(found, max, path))
    }

    fn limit_error(self, found: usize, max: usize, path: &str) -> MutantsOutcomesError {
        match self {
            Self::Outcomes => too_many_outcomes_error(path, found, max),
            Self::Records => too_many_records_error(path, found, max),
        }
    }
}

fn outcomes_parse_error(path: &str, reason: Box<str>) -> MutantsOutcomesError {
    MutantsOutcomesError::OutcomesJsonParse { path: Box::from(path), reason }
}

fn records_parse_error(path: &str, reason: Box<str>) -> MutantsOutcomesError {
    MutantsOutcomesError::RecordsJsonParse { path: Box::from(path), reason }
}

fn too_many_outcomes_error(path: &str, found: usize, max: usize) -> MutantsOutcomesError {
    MutantsOutcomesError::TooManyOutcomes { path: Box::from(path), found, max }
}

fn too_many_records_error(path: &str, found: usize, max: usize) -> MutantsOutcomesError {
    MutantsOutcomesError::TooManyRecords { path: Box::from(path), found, max }
}
