use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use titania_lanes::helpers::{relative_path, walk_rs_files};

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

pub(super) fn tracked_set(root: &Path) -> HashSet<String> {
    tracked_rust_files(root)
        .map_or_else(HashSet::new, |files| files.iter().map(|p| relative_path(root, p)).collect())
}

pub(super) fn tracked_rust_files(root: &Path) -> Option<Vec<PathBuf>> {
    let crates_dir = root.join("crates");
    if !crates_dir.is_dir() {
        return None;
    }
    let mut out = Vec::new();
    walk_core_lanes(&crates_dir, &mut out);
    Some(out)
}

fn walk_core_lanes(crates_dir: &Path, out: &mut Vec<PathBuf>) {
    walk_rs_files(&crates_dir.join("titania-core/src"), out);
    walk_rs_files(&crates_dir.join("titania-lanes/src"), out);
}
