use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::error::MutantsOutcomesError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawPoint {
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawSpan {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<RawPoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<RawPoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WireArtifact {
    Outcomes,
    Records,
}

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
