use std::path::{Path, PathBuf};

use titania_lanes::{
    Finding, LaneReport, RuleId,
    helpers::{WalkError, line_no_from_idx, relative_path, walk_rs_files},
};

const MARKER_PREFIX: &str = "changed by ";
const MARKER_SUFFIX: &str = "cargo-mutants";
const MUTANTS_RULE: &str = "MUTANTS_RESIDUE";

/// Typed error returned by the cargo-mutants residue scanner.
#[derive(Debug, thiserror::Error)]
pub enum MutantsError {
    /// Directory walk failed.
    #[error(transparent)]
    Walk(#[from] WalkError),
    /// Embedded rule id failed validation.
    #[error("invalid rule id {MUTANTS_RULE}: {0}")]
    RuleId(#[from] titania_lanes::RuleIdError),
}

/// Check first-party source files for cargo-mutants residue markers.
///
/// # Errors
///
/// Returns [`MutantsError::Walk`] on traversal failure or [`MutantsError::RuleId`]
/// when the embedded rule id fails validation.
pub(super) fn check_mutants_residue(
    root: &Path,
    report: &mut LaneReport,
) -> Result<(), MutantsError> {
    let rule = RuleId::new(MUTANTS_RULE)?;
    rust_files_in_crates(root)?
        .into_iter()
        .for_each(|file| check_mutant_file(root, &file, &rule, report));
    Ok(())
}

/// Discover all `*.rs` files under `root/crates/*/src/`.
///
/// # Errors
///
/// Returns [`WalkError`] on directory traversal failure.
fn rust_files_in_crates(root: &Path) -> Result<Vec<PathBuf>, WalkError> {
    let crates_dir = root.join("crates");
    let Ok(read) = std::fs::read_dir(&crates_dir) else {
        return Ok(Vec::new());
    };
    read.into_iter()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter_map(|p| source_dir(&p))
        .try_fold(Vec::new(), |mut acc, src| {
            let nested = walk_rs_files(&src)?;
            acc.extend(nested);
            Ok(acc)
        })
}
fn source_dir(path: &Path) -> Option<PathBuf> {
    if !path.is_dir() {
        return None;
    }
    let src = path.join("src");
    src.is_dir().then_some(src)
}

fn check_mutant_file(root: &Path, file: &Path, rule: &RuleId, report: &mut LaneReport) {
    let Ok(text) = std::fs::read_to_string(file) else {
        return;
    };
    text.lines()
        .enumerate()
        .filter(|(_, line)| has_mutants_marker(line))
        .fold((), |(), (idx, _)| push_mutants_finding(root, file, idx, rule, report));
}

fn has_mutants_marker(line: &str) -> bool {
    line.contains(MARKER_PREFIX) && line.contains(MARKER_SUFFIX)
}

fn push_mutants_finding(
    root: &Path,
    file: &Path,
    idx: usize,
    rule: &RuleId,
    report: &mut LaneReport,
) {
    report.push(Finding::new(
        rule.clone(),
        relative_path(root, file),
        line_no_from_idx(idx),
        "cargo-mutants residue marker present",
    ));
}
