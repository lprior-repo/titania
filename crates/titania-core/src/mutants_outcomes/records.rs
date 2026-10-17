use serde::{Deserialize, Serialize};

use crate::{
    error::MutantsOutcomesError,
    proof_id::{MutantId, MutantOperator},
};

use super::wire::{RawSpan, WireArtifact, parse_capped_wire};

pub const MUTANTS_RECORDS_MAX_ENTRIES: usize = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MutantsRecords(pub Vec<MutantRecord>);

impl MutantsRecords {
    pub fn parse_str(contents: &str, path: &str) -> Result<Self, MutantsOutcomesError> {
        parse_capped_wire(
            contents,
            path,
            MUTANTS_RECORDS_MAX_ENTRIES,
            WireArtifact::Records,
            |records: &Self| records.0.len(),
        )
    }

    #[must_use]
    pub fn as_slice(&self) -> &[MutantRecord] {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> Vec<MutantRecord> {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutantRecord {
    pub name: String,
    pub package: String,
    pub file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<RawSpan>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function: Option<RawFunction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawFunction {
    #[serde(default, rename = "function_name", skip_serializing_if = "Option::is_none")]
    pub function_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<RawSpan>,
}

impl MutantRecord {
    #[must_use]
    pub fn start_point(&self) -> Option<(u32, u32)> {
        self.span
            .as_ref()
            .and_then(|span| span.start.as_ref())
            .map(|point| (point.line, point.column))
    }

    #[must_use]
    pub fn classify_operator(&self) -> MutantOperator {
        match self.genre.as_deref() {
            Some("BinaryOperator") => classify_binary_operator(&self.name),
            Some("UnaryOperator") => MutantOperator::RemoveNegation,
            _ => MutantOperator::DefaultReplace,
        }
    }

    pub fn typed_id(&self, path: &str) -> Result<MutantId, MutantsOutcomesError> {
        let path_label: Box<str> = Box::from(path);
        let (line, column) = self.require_start_point(&path_label)?;
        let relative_path = self.require_relative_path(&path_label)?;
        MutantId::new(&self.package, relative_path, line, column, self.classify_operator()).map_err(
            |error| MutantsOutcomesError::InvalidMutantId {
                path: path_label,
                mutation_name: Box::from(self.name.as_str()),
                reason: error.to_string().into_boxed_str(),
            },
        )
    }

    fn require_start_point(&self, path: &str) -> Result<(u32, u32), MutantsOutcomesError> {
        self.start_point().ok_or_else(|| missing_source_span_error(path, &self.name))
    }

    fn require_relative_path<'a>(&'a self, path: &str) -> Result<&'a str, MutantsOutcomesError> {
        relative_mutant_path(&self.package, &self.file)
            .ok_or_else(|| path_outside_package_error(path, &self.name))
    }
}

#[must_use]
fn missing_source_span_error(path: &str, mutation_name: &str) -> MutantsOutcomesError {
    MutantsOutcomesError::MissingSourceSpan {
        path: Box::from(path),
        mutation_name: Box::from(mutation_name),
    }
}

#[must_use]
fn path_outside_package_error(path: &str, mutation_name: &str) -> MutantsOutcomesError {
    MutantsOutcomesError::PathOutsidePackage {
        path: Box::from(path),
        mutation_name: Box::from(mutation_name),
    }
}

const BINARY_OPERATOR_PATTERNS: &[(&str, MutantOperator)] = &[
    ("replace == with !=", MutantOperator::EqualReplace),
    ("replace != with ==", MutantOperator::NotInserted),
    ("replace && with ||", MutantOperator::AndOr),
    ("replace || with &&", MutantOperator::AndOr),
];

#[must_use]
fn classify_binary_operator(name: &str) -> MutantOperator {
    BINARY_OPERATOR_PATTERNS
        .iter()
        .find(|(pattern, _)| name.contains(pattern))
        .map_or(MutantOperator::ArithmeticOpFlip, |(_, operator)| *operator)
}

#[must_use]
pub fn relative_mutant_path<'a>(package: &str, file: &'a str) -> Option<&'a str> {
    if let Some(workspace_path) = file.strip_prefix("crates/") {
        return workspace_path.strip_prefix(package).and_then(|path| path.strip_prefix('/'));
    }
    file.strip_prefix("./").or(Some(file))
}
