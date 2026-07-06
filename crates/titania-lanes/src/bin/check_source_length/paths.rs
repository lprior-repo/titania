use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use titania_lanes::helpers::{WalkError, relative_path, walk_rs_files};

pub(super) use titania_lanes::helpers::is_excluded_source_path;

pub(super) fn is_test_like_source_path(file: &str) -> bool {
    file.ends_with("/tests.rs")
        || file.ends_with("_tests.rs")
        || file.contains("/tests/")
        || file.starts_with("tests/")
        || file.contains("/kani/")
        || file.starts_with("kani_")
        || file.contains("kani_")
        || file.contains("/verification/")
        || file.starts_with("verification/")
        || file.contains("/proptest")
        || file.contains("/benches/")
}

pub(super) fn is_titania_hot_source(root: &Path, file: &Path) -> bool {
    let rel = relative_path(root, file);
    !is_test_like_source_path(&rel)
        && (rel.starts_with("crates/titania-core/src/")
            || rel.starts_with("crates/titania-lanes/src/"))
}

/// Build a set of tracked Rust source paths relative to the project root.
///
/// # Errors
///
/// Returns [`WalkError`] when the crates directory walk fails.
pub(super) fn tracked_set(root: &Path) -> Result<HashSet<String>, WalkError> {
    Ok(tracked_rust_files(root)?.iter().map(|p| relative_path(root, p)).collect::<HashSet<_>>())
}
/// Enumerate all tracked Rust source files under `crates/titania-core/src/` and `crates/titania-lanes/src/`.
///
/// # Errors
///
/// Returns [`WalkError`] when the crates directory is unreadable or a lane walk fails.
pub(super) fn tracked_rust_files(root: &Path) -> Result<Vec<PathBuf>, WalkError> {
    let crates_dir = root.join("crates");
    if !crates_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    out.extend(walk_rs_files(&crates_dir.join("titania-core/src"))?);
    out.extend(walk_rs_files(&crates_dir.join("titania-lanes/src"))?);
    Ok(out)
}
