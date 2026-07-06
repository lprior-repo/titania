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

use thiserror::Error;

/// Typed error returned by [`walk_rs_files`].
///
/// The directory walk never silently swallows a `read_dir` failure: callers
/// receive this error so they can propagate it through their typed lane error
/// or surface a typed infrastructure failure. `NotFound` is exempt because the
/// titania-lanes callers treat an absent target directory as an empty walk.
#[derive(Debug, Error)]
#[error("failed to walk directory {path}: {source}")]
pub struct WalkError {
    /// Directory whose `read_dir` call failed.
    path: PathBuf,
    /// Underlying filesystem failure.
    #[source]
    source: std::io::Error,
}

impl WalkError {
    /// Construct a typed walk error from a path and an `io::Error`.
    const fn new(path: PathBuf, source: std::io::Error) -> Self {
        Self { path, source }
    }
}

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

/// Recursively collect `*.rs` files under `dir`.
///
/// `NotFound` is treated as an empty walk (callers prune optional target
/// directories); every other `read_dir` failure is propagated via
/// [`WalkError`] so the lane never silently hides a filesystem error.
///
/// # Errors
/// Returns [`WalkError`] when directory traversal fails for any reason other
/// than `ErrorKind::NotFound`.
pub fn walk_rs_files(dir: &Path) -> Result<Vec<PathBuf>, WalkError> {
    let entries = read_dir_list(dir)?;
    entries.iter().try_fold(Vec::new(), |acc, path| collect_descendant(acc, path))
}
/// Open `dir` and collect entry paths, treating `NotFound` as empty.
///
/// # Errors
///
/// Returns [`WalkError`] when `read_dir` fails for any reason other than
/// `ErrorKind::NotFound`.
fn read_dir_list(dir: &Path) -> Result<Vec<PathBuf>, WalkError> {
    let read_dir = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(WalkError::new(dir.to_path_buf(), source)),
    };
    Ok(read_dir.fold(Vec::new(), push_entry_path))
}

/// Fold accumulator that appends each successful entry path.
fn push_entry_path(
    mut acc: Vec<PathBuf>,
    entry: std::io::Result<std::fs::DirEntry>,
) -> Vec<PathBuf> {
    if let Ok(e) = entry {
        acc.push(e.path());
    }
    acc
}

/// Try-fold body: recurse into one directory entry, appending `*.rs` files.
///
/// # Errors
///
/// Returns [`WalkError`] when the recursive walk fails.
fn collect_descendant(mut acc: Vec<PathBuf>, path: &Path) -> Result<Vec<PathBuf>, WalkError> {
    descend_entry(path, &mut acc)?;
    Ok(acc)
}

/// Recurse into a directory entry, collecting `*.rs` files.
///
/// # Errors
///
/// Returns [`WalkError`] when directory traversal fails.
fn descend_entry(path: &Path, out: &mut Vec<PathBuf>) -> Result<(), WalkError> {
    if path.is_dir() {
        let nested = walk_rs_files(path)?;
        out.extend(nested);
    } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
        out.push(path.to_path_buf());
    }
    Ok(())
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
