//! `mutants.json` records, typed-ID conversion, and path handling.

use serde::{Deserialize, Serialize};

use crate::{
    error::MutantsOutcomesError,
    proof_id::{MutantId, MutantOperator},
};

use super::wire::{RawSpan, WireArtifact, parse_capped_wire};

/// Static upper bound on records per `mutants.json` file.
pub const MUTANTS_RECORDS_MAX_ENTRIES: usize = 1_000_000;

/// Flat list of per-mutant records parsed from `mutants.json`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MutantsRecords(pub Vec<MutantRecord>);

impl MutantsRecords {
    /// Parse a cargo-mutants `mutants.json` payload.
    ///
    /// `path` is a caller-provided diagnostic label, typically the
    /// artifact's on-disk path.
    ///
    /// # Errors
    /// - [`MutantsOutcomesError::RecordsJsonParse`] when `contents`
    ///   is malformed JSON or has the wrong wire shape.
    /// - [`MutantsOutcomesError::TooManyRecords`] when the record count
    ///   exceeds [`MUTANTS_RECORDS_MAX_ENTRIES`].
    pub fn parse_str(contents: &str, path: &str) -> Result<Self, MutantsOutcomesError> {
        parse_capped_wire(
            contents,
            path,
            MUTANTS_RECORDS_MAX_ENTRIES,
            WireArtifact::Records,
            |records: &Self| records.0.len(),
        )
    }

    /// Borrow the inner record slice.
    #[must_use]
    pub fn as_slice(&self) -> &[MutantRecord] {
        &self.0
    }

    /// Consume the wrapper and return the inner record vector.
    #[must_use]
    pub fn into_inner(self) -> Vec<MutantRecord> {
        self.0
    }
}

/// One per-mutant record from cargo-mutants `mutants.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutantRecord {
    /// Human-readable cargo-mutants mutation name.
    pub name: String,
    /// Cargo package that owns the mutation.
    pub package: String,
    /// Workspace-relative source file the mutation touches.
    pub file: String,
    /// Source span the mutation targets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<RawSpan>,
    /// cargo-mutants genre tag used for operator classification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    /// Textual replacement cargo-mutants would apply.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
    /// Function context retained for diagnostic surfaces.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function: Option<RawFunction>,
}

/// Optional cargo-mutants function context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawFunction {
    /// Enclosing function or impl-method name.
    #[serde(default, rename = "function_name", skip_serializing_if = "Option::is_none")]
    pub function_name: Option<String>,
    /// Enclosing function return type as cargo-mutants prints it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_type: Option<String>,
    /// Enclosing function source span.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<RawSpan>,
}

impl MutantRecord {
    /// Return the mutation start point when both span components exist.
    #[must_use]
    pub fn start_point(&self) -> Option<(u32, u32)> {
        self.span
            .as_ref()
            .and_then(|span| span.start.as_ref())
            .map(|point| (point.line, point.column))
    }

    /// Map a cargo-mutants record to its typed [`MutantOperator`].
    #[must_use]
    pub fn classify_operator(&self) -> MutantOperator {
        match self.genre.as_deref() {
            Some("BinaryOperator") => classify_binary_operator(&self.name),
            Some("UnaryOperator") => MutantOperator::RemoveNegation,
            _ => MutantOperator::DefaultReplace,
        }
    }

    /// Build a typed [`MutantId`] from this record.
    ///
    /// # Errors
    /// - [`MutantsOutcomesError::MissingSourceSpan`] when `span.start`
    ///   is absent.
    /// - [`MutantsOutcomesError::PathOutsidePackage`] when `file` has a
    ///   mismatched `crates/<package>/` prefix.
    /// - [`MutantsOutcomesError::InvalidMutantId`] when [`MutantId::new`]
    ///   rejects an assembled field.
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

    /// Extract the source start point required by [`MutantId::new`].
    ///
    /// # Errors
    /// Returns [`MutantsOutcomesError::MissingSourceSpan`] when the
    /// record omits `span.start`.
    fn require_start_point(&self, path: &str) -> Result<(u32, u32), MutantsOutcomesError> {
        self.start_point().ok_or_else(|| missing_source_span_error(path, &self.name))
    }

    /// Resolve the package-relative path required by [`MutantId::new`].
    ///
    /// # Errors
    /// Returns [`MutantsOutcomesError::PathOutsidePackage`] when a
    /// workspace-prefixed file names a different package.
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

/// Classify a binary-operator mutation by scanning its textual name.
#[must_use]
fn classify_binary_operator(name: &str) -> MutantOperator {
    if name.contains("replace == with !=") {
        MutantOperator::EqualReplace
    } else if name.contains("replace != with ==") {
        MutantOperator::NotInserted
    } else if name.contains("replace && with ||") || name.contains("replace || with &&") {
        MutantOperator::AndOr
    } else {
        MutantOperator::ArithmeticOpFlip
    }
}

/// Convert a workspace-relative cargo-mutants path to package-relative form.
///
/// `crates/<package>/` is stripped when it matches the declared package;
/// a mismatched workspace package returns `None`. Bare paths pass through
/// after an optional leading `./` is removed.
#[must_use]
pub fn relative_mutant_path<'a>(package: &str, file: &'a str) -> Option<&'a str> {
    if let Some(workspace_path) = file.strip_prefix("crates/") {
        return workspace_path.strip_prefix(package).and_then(|path| path.strip_prefix('/'));
    }
    file.strip_prefix("./").or(Some(file))
}
