//! Shared helpers for the titania-lanes bin implementations.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]
#![deny(clippy::string_slice)]
#![deny(clippy::get_unwrap)]
#![deny(clippy::arithmetic_side_effects)]
#![deny(clippy::dbg_macro)]
#![deny(clippy::as_conversions)]
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

/// Convert a 0-indexed iterator index to a 1-indexed line number (`0` on overflow).
#[must_use]
pub fn line_no_from_idx(idx: usize) -> u32 {
    let Ok(n) = u32::try_from(idx) else {
        return 0;
    };
    n.checked_add(1).map_or(0, |v| v)
}

/// Saturating addition for `usize` (overflow clamps at `usize::MAX`).
#[must_use]
pub const fn saturating_add_usize(a: usize, b: usize) -> usize {
    a.saturating_add(b)
}

/// Net brace delta (`{` minus `}`) for a line, saturating at `i32` bounds.
#[must_use]
pub fn brace_delta(text: &str) -> i32 {
    text.chars().fold(0, |delta, ch| delta.saturating_add(brace_delta_of(ch)))
}

/// Per-character delta contribution for [`brace_delta`].
const fn brace_delta_of(ch: char) -> i32 {
    match ch {
        '{' => 1,
        '}' => -1,
        _ => 0,
    }
}

/// Render a path with backslashes normalized to forward slashes.
#[must_use]
pub fn normalize_slashes(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// Repository-relative path string, normalized to forward slashes.
#[must_use]
pub fn relative_path(root: &Path, p: &Path) -> String {
    p.strip_prefix(root).map_or_else(|_| normalize_slashes(p), normalize_slashes)
}

/// Recursively collect `*.rs` files under `dir` into `out`.
pub fn walk_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    read.flatten().for_each(|entry| descend_entry(entry.path(), out));
}

/// Recurse into a directory entry, collecting `*.rs` files.
fn descend_entry(path: PathBuf, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        walk_rs_files(&path, out);
    } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
        out.push(path);
    }
}

/// Canonical first-party source-path exclusion.
///
/// A path is excluded when it lives under a build/VCS/cache prefix,
/// contains such a segment, or is a build/VCS leaf directory. Shared by
/// the source-length and nightly-features lanes so exclusion logic is
/// not duplicated.
#[must_use]
pub fn is_excluded_source_path(file: &str) -> bool {
    is_bad_prefix(file) || is_bad_segment(file) || is_bad_leaf(file)
}

fn is_bad_prefix(file: &str) -> bool {
    [
        "target/",
        ".git/",
        ".jj/",
        ".beads/",
        ".evidence/",
        ".cargo_temp/",
        "cargo-home/",
        "cargo_home/",
        ".cargo/registry/",
    ]
    .iter()
    .any(|prefix| file.starts_with(prefix))
}

fn is_bad_segment(file: &str) -> bool {
    [
        "/target/",
        "/.git/",
        "/.jj/",
        "/.beads/",
        "/.evidence/",
        "/.cargo_temp/",
        "/cargo-home/",
        "/cargo_home/",
        "/.cargo/registry/",
    ]
    .iter()
    .any(|segment| file.contains(segment))
}

fn is_bad_leaf(file: &str) -> bool {
    matches!(file, "target" | ".git" | ".jj" | ".beads" | ".evidence" | ".cargo_temp")
}
