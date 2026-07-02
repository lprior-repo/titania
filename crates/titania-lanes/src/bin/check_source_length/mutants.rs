use std::path::{Path, PathBuf};

use titania_lanes::{
    Finding, LaneReport, RuleId, RuleIdError,
    helpers::{line_no_from_idx, relative_path, walk_rs_files},
};

const MARKER_PREFIX: &str = "changed by ";
const MARKER_SUFFIX: &str = "cargo-mutants";
const MUTANTS_RULE: &str = "MUTANTS_RESIDUE";

/// Check first-party source files for cargo-mutants residue markers.
///
/// # Errors
///
/// Returns a rule-id construction error when the mutants rule id is invalid.
pub(super) fn check_mutants_residue(
    root: &Path,
    report: &mut LaneReport,
) -> Result<(), RuleIdError> {
    let rule = RuleId::new(MUTANTS_RULE)?;
    for file in rust_files_in_crates(root) {
        check_mutant_file(root, &file, &rule, report);
    }
    Ok(())
}

fn rust_files_in_crates(root: &Path) -> Vec<PathBuf> {
    let crates_dir = root.join("crates");
    let Ok(read) = std::fs::read_dir(&crates_dir) else {
        return Vec::new();
    };
    let mut all = Vec::new();
    read.filter_map(Result::ok)
        .filter_map(|entry| source_dir(&entry.path()))
        .for_each(|src| walk_rs_files(&src, &mut all));
    all
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
        .for_each(|(idx, _)| push_mutants_finding(root, file, idx, rule, report));
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
