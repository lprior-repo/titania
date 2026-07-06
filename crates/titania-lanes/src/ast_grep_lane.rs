//! Embedded ast-grep lane runner.
//!
//! The v1 lane parses every source file with the real ast-grep Rust
//! engine ([`engine::AstEngine`]), runs the embedded rule YAML catalog,
//! and emits typed [`LaneOutcome`] values. It does not shell out to an
//! `ast-grep` binary.

mod engine;
mod rules;

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use rules::{RULES, RuleDef, repair_hint, rule_applies};
use thiserror::Error;
use titania_core::{
    CommandEvidence, Digest, Finding, FindingEffect, Lane, LaneEvidence, LaneOutcome, Location,
    LocationError, OutcomeError, ProcessTermination, RuleId, RuleIdError, WorkspacePath,
    WorkspacePathError,
};

/// Errors returned by the embedded ast-grep lane.
#[derive(Debug, Error)]
pub enum AstGrepLaneError {
    /// A fixture or target source file path was not UTF-8.
    #[error("path is not valid UTF-8: {path}")]
    NonUtf8Path {
        /// Path that could not be rendered as UTF-8.
        path: String,
    },
    /// An absolute path could not be made workspace-relative without a target root.
    #[error("absolute path requires a target root or known fixture marker: {path}")]
    AbsolutePathWithoutRoot {
        /// Absolute path that cannot safely become a workspace path.
        path: String,
    },
    /// A source line number exceeded the supported u32 range.
    #[error("source line index does not fit in u32: {index}")]
    LineNumberOverflow {
        /// Zero-based line index that could not be represented.
        index: usize,
    },
    /// A source file could not be read.
    #[error("failed to read {path}: {source}")]
    ReadFile {
        /// Source file path.
        path: String,
        /// Filesystem failure.
        #[source]
        source: io::Error,
    },
    /// Rule id validation failed.
    #[error(transparent)]
    RuleId(#[from] RuleIdError),
    /// Finding workspace path validation failed.
    #[error(transparent)]
    WorkspacePath(#[from] WorkspacePathError),
    /// Finding source span validation failed.
    #[error(transparent)]
    Location(#[from] LocationError),
    /// Clean-lane evidence validation failed.
    #[error(transparent)]
    Outcome(#[from] OutcomeError),
}

/// Return the embedded runtime rule IDs in dispatch order.
///
/// Tests use this to prove the runtime dispatch table stays in parity with the
/// YAML files supplied to [`run`].
pub fn embedded_rule_ids() -> impl Iterator<Item = &'static str> {
    RULES.iter().map(|rule| rule.id)
}

/// Run the embedded ast-grep rule catalog over the supplied source paths.
///
/// `rules_yaml` is the compile-time embedded catalog. `fixture_paths` are read
/// directly; unreadable paths are errors, not fabricated findings. `exceptions`
/// suppress only exact `(rule_id, workspace_path)` pairs.
///
/// # Errors
/// Returns [`AstGrepLaneError`] when a source file cannot be read, path/rule
/// identifiers fail validation, or clean-lane evidence cannot be constructed.
pub fn run(
    rules_yaml: &[&'static str],
    fixture_paths: &[PathBuf],
    exceptions: &[(RuleId, String)],
) -> Result<LaneOutcome, AstGrepLaneError> {
    let catalog = catalog_text(rules_yaml);
    let findings = fixture_paths
        .iter()
        .map(|path| scan_path(path.as_path(), &catalog, exceptions))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .into_boxed_slice();

    if findings.is_empty() { clean_outcome() } else { Ok(LaneOutcome::Findings { findings }) }
}

/// Scan one source file against all enabled rule definitions.
///
/// The file is parsed once by the real ast-grep engine; every rule whose
/// YAML id is enabled is asked to detect against the parsed [`AstEngine`].
/// Rules that the engine cannot yet express (architecture import paths,
/// inline-suppression comments) fall back to the legacy string detectors.
///
/// # Errors
/// Returns [`AstGrepLaneError`] when the source path cannot be read or converted
/// to a workspace path, or when a finding cannot be constructed.
fn scan_path(
    path: &Path,
    catalog: &str,
    exceptions: &[(RuleId, String)],
) -> Result<Vec<Finding>, AstGrepLaneError> {
    let source = read_source(path)?;
    let workspace_path = workspace_path(path)?;
    let engine = engine::AstEngine::new(&source);
    RULES
        .iter()
        .copied()
        .filter(|rule| rule_enabled(catalog, rule.id))
        .filter(|rule| rule_applies(rule, &workspace_path))
        .filter_map(|rule| detect_line(rule, &engine, &source).map(|line| (rule, line)))
        .map(|(rule, line)| finding(rule, line, &workspace_path))
        .collect::<Result<Vec<_>, _>>()
        .map(|findings| suppress_exceptions(findings, exceptions))
}

/// Read a source file as UTF-8 text.
///
/// # Errors
/// Returns [`AstGrepLaneError::ReadFile`] when the file cannot be read.
fn read_source(path: &Path) -> Result<String, AstGrepLaneError> {
    fs::read_to_string(path)
        .map_err(|source| AstGrepLaneError::ReadFile { path: path_display(path), source })
}

/// Convert a caller path into a validated workspace path for findings.
///
/// Relative paths are accepted as workspace-relative. Absolute fixture paths may
/// be stripped at the checked-in `ast_grep` fixture root; other absolute paths
/// need a future target-root aware API and are rejected.
///
/// # Errors
/// Returns [`AstGrepLaneError::NonUtf8Path`],
/// [`AstGrepLaneError::AbsolutePathWithoutRoot`], or
/// [`AstGrepLaneError::WorkspacePath`].
fn workspace_path(path: &Path) -> Result<WorkspacePath, AstGrepLaneError> {
    let rendered = path_to_str(path)?;
    let relative = match (path.is_absolute(), fixture_relative_path(&rendered)) {
        (true, Some(path)) => path,
        (true, None) => return Err(AstGrepLaneError::AbsolutePathWithoutRoot { path: rendered }),
        (false, _) => rendered,
    };
    WorkspacePath::new(&relative).map_err(Into::into)
}

fn fixture_relative_path(rendered: &str) -> Option<String> {
    let parts = rendered.split('/').collect::<Vec<_>>();
    parts
        .iter()
        .position(|part| *part == "ast_grep")
        .and_then(|index| index.checked_add(1))
        .map(|start| parts.into_iter().skip(start).collect::<Vec<_>>().join("/"))
}

/// Render a path as UTF-8.
///
/// # Errors
/// Returns [`AstGrepLaneError::NonUtf8Path`] when `path` is not valid UTF-8.
fn path_to_str(path: &Path) -> Result<String, AstGrepLaneError> {
    path.to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| AstGrepLaneError::NonUtf8Path { path: path_display(path) })
}

/// Build a typed finding for a matching rule at the given 0-based line.
///
/// # Errors
/// Returns [`AstGrepLaneError`] when rule id or source span validation fails.
fn finding(
    rule: RuleDef,
    line_index: usize,
    workspace_path: &WorkspacePath,
) -> Result<Finding, AstGrepLaneError> {
    let rule_id = RuleId::new(rule.id)?;
    let line = checked_line_number(line_index)?;
    let location = Location::span(workspace_path.clone(), line, 0, line, 1)?;
    let repair = repair_hint(rule.repair);
    Ok(match rule.effect {
        FindingEffect::Reject => {
            Finding::reject(Lane::AstGrep, rule_id, location, rule.message.to_owned(), repair)
        }
        FindingEffect::Informational => Finding::informational(
            Lane::AstGrep,
            rule_id,
            location,
            rule.message.to_owned(),
            repair,
        ),
    })
}

/// Run a rule's detector and return the 0-based line of its first match.
///
/// `None` when the rule does not apply. Engine-aware: the ast-grep parse
/// always succeeds (syntax errors surface as error nodes inside the tree),
/// so engine detectors run unconditionally and string detectors fall back
/// only for the few rules ast-grep cannot express (inline-suppression
/// comments, path-filtered architecture imports).
fn detect_line(rule: RuleDef, engine: &engine::AstEngine, source: &str) -> Option<usize> {
    rule.detect.run(engine, source)
}

/// Convert a zero-based line index into a one-based `u32` line number.
///
/// # Errors
/// Returns [`AstGrepLaneError::LineNumberOverflow`] when the index or one-based
/// result cannot be represented as `u32`.
fn checked_line_number(index: usize) -> Result<u32, AstGrepLaneError> {
    u32::try_from(index)
        .ok()
        .and_then(|line| line.checked_add(1))
        .ok_or(AstGrepLaneError::LineNumberOverflow { index })
}

fn suppress_exceptions(findings: Vec<Finding>, exceptions: &[(RuleId, String)]) -> Vec<Finding> {
    findings.into_iter().filter(|finding| !is_suppressed(finding, exceptions)).collect()
}

fn is_suppressed(finding: &Finding, exceptions: &[(RuleId, String)]) -> bool {
    finding.location().span_file().is_some_and(|path| exception_matches(finding, path, exceptions))
}

fn exception_matches(
    finding: &Finding,
    path: &WorkspacePath,
    exceptions: &[(RuleId, String)],
) -> bool {
    exceptions.iter().any(|(rule_id, exception_path)| {
        rule_id == finding.rule_id() && exception_path == path.as_str()
    })
}

/// Construct clean evidence for an empty ast-grep run.
///
/// # Errors
/// Returns [`AstGrepLaneError::Outcome`] when command or evidence invariants
/// fail validation.
fn clean_outcome() -> Result<LaneOutcome, AstGrepLaneError> {
    let command = CommandEvidence::new(
        "titania-ast-grep".to_owned(),
        Box::from(["titania-ast-grep".to_owned()]),
    )?;
    let evidence = LaneEvidence::new(
        command,
        "embedded-rules-v1".to_owned(),
        ProcessTermination::Exited { code: 0 },
        Digest::from_bytes(b"ast-grep-clean"),
    )?;
    Ok(LaneOutcome::Clean { evidence })
}

fn catalog_text(rules_yaml: &[&'static str]) -> String {
    rules_yaml.join("\n")
}

fn rule_enabled(catalog: &str, rule_id: &str) -> bool {
    catalog.contains(&["id: ", rule_id].concat())
}

fn path_display(path: &Path) -> String {
    path.display().to_string()
}
