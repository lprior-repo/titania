use std::path::{Path, PathBuf};

use titania_lanes::{
    Finding, LaneReport,
    helpers::{line_no_from_idx, relative_path, walk_rs_files},
};

const MARKER_PREFIX: &str = "changed by ";
const MARKER_SUFFIX: &str = "cargo-mutants";

pub fn check_mutants_residue(root: &Path, report: &mut LaneReport) {
    for file in rust_files_in_crates(root) {
        check_mutant_file(root, &file, report);
    }
}

fn rust_files_in_crates(root: &Path) -> Vec<PathBuf> {
    let crates_dir = root.join("crates");
    let Ok(read) = std::fs::read_dir(&crates_dir) else {
        return Vec::new();
    };
    let mut all = Vec::new();
    for src in read
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter_map(|path| source_dir(&path))
    {
        walk_rs_files(&src, root, &mut all);
    }
    all
}

fn source_dir(path: &Path) -> Option<PathBuf> {
    if !path.is_dir() {
        return None;
    }
    let src = path.join("src");
    src.is_dir().then_some(src)
}

fn check_mutant_file(root: &Path, file: &Path, report: &mut LaneReport) {
    let Ok(text) = std::fs::read_to_string(file) else {
        return;
    };
    for (idx, line) in text.lines().enumerate() {
        if has_mutants_marker(line) {
            push_mutants_finding(root, file, idx, report);
        }
    }
}

fn has_mutants_marker(line: &str) -> bool {
    line.contains(MARKER_PREFIX) && line.contains(MARKER_SUFFIX)
}

fn push_mutants_finding(root: &Path, file: &Path, idx: usize, report: &mut LaneReport) {
    report.push(Finding::new(
        "MUTANTS-RESIDUE",
        relative_path(root, file),
        line_no_from_idx(idx),
        "cargo-mutants residue marker present",
    ));
}
