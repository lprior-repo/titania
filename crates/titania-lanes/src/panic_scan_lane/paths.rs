use std::path::{Path, PathBuf};

use super::EXCLUDED_SEGMENTS;

/// Collect production Rust source files under every workspace crate.
#[must_use]
pub(super) fn collect_source_files(root: &Path) -> Vec<PathBuf> {
    let mut files = source_roots(root)
        .into_iter()
        .flat_map(walk_rust_files)
        .filter(|p| !is_excluded_path(p))
        .collect::<Vec<_>>();
    files.sort();
    files
}

fn source_roots(root: &Path) -> Vec<PathBuf> {
    std::iter::once(root.join("src"))
        .chain(crate_source_roots(root))
        .filter(|p| p.is_dir())
        .collect()
}

fn crate_source_roots(root: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(root.join("crates")).map_or_else(
        |_| Vec::new(),
        |entries| entries.filter_map(Result::ok).map(|e| e.path().join("src")).collect(),
    )
}

fn walk_rust_files(dir: PathBuf) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let mut stack: Vec<PathBuf> = vec![dir];
    std::iter::from_fn(|| {
        let top = stack.pop()?;
        visit_walk_dir(&top, &mut stack, &mut out);
        Some(())
    })
    .for_each(drop);
    out.sort();
    out
}

fn visit_walk_dir(top: &Path, stack: &mut Vec<PathBuf>, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(top) else {
        return;
    };
    entries.flatten().for_each(|entry| record_walk_path(entry.path(), stack, out));
}

fn record_walk_path(path: PathBuf, stack: &mut Vec<PathBuf>, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        stack.push(path);
    } else if path.extension().is_some_and(|e| e == "rs") {
        out.push(path);
    }
}

/// Replicate the bash `--glob '!...'`. The list mirrors
/// `check-panic-surface.sh`. We test path segments (not just
/// substrings) so e.g. `models/loom/foo.rs` matches but
/// `my_models_loom/foo.rs` does not.
fn is_excluded_path(path: &Path) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/");
    if EXCLUDED_SEGMENTS.iter().any(|seg| normalized.contains(seg)) {
        return true;
    }
    let name = path.file_name().and_then(|n| n.to_str()).map_or("", |value| value);
    if name == "tests.rs"
        || name == "build.rs"
        || name.ends_with("_tests.rs")
        || name == "check-panic-surface.sh"
        || name == "check_panic_surface.rs"
        || name.starts_with("kani")
    {
        return true;
    }
    false
}

/// Render a path relative to the target root when possible.
#[must_use]
pub(super) fn rel_str(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map_or_else(|_| path.display().to_string(), |rel| rel.display().to_string())
}
