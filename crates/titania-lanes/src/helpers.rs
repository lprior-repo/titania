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

#[derive(Debug, Error)]
#[error("failed to walk directory {path}: {source}")]
pub struct WalkError {
    path: PathBuf,
    #[source]
    source: std::io::Error,
}

impl WalkError {
    const fn new(path: PathBuf, source: std::io::Error) -> Self {
        Self { path, source }
    }
}

#[must_use]
pub fn line_no_from_idx(idx: usize) -> u32 {
    let Ok(n) = u32::try_from(idx) else {
        return 0;
    };
    n.checked_add(1).map_or(0, |v| v)
}

#[must_use]
pub const fn saturating_add_usize(a: usize, b: usize) -> usize {
    a.saturating_add(b)
}

#[must_use]
pub fn brace_delta(text: &str) -> i32 {
    text.chars().fold(0, |delta, ch| delta.saturating_add(brace_delta_of(ch)))
}

const fn brace_delta_of(ch: char) -> i32 {
    match ch {
        '{' => 1,
        '}' => -1,
        _ => 0,
    }
}

#[must_use]
pub fn normalize_slashes(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

#[must_use]
pub fn relative_path(root: &Path, p: &Path) -> String {
    p.strip_prefix(root).map_or_else(|_| normalize_slashes(p), normalize_slashes)
}

pub fn walk_rs_files(dir: &Path) -> Result<Vec<PathBuf>, WalkError> {
    let entries = read_dir_list(dir)?;
    entries.iter().try_fold(Vec::new(), |acc, path| collect_descendant(acc, path))
}
fn read_dir_list(dir: &Path) -> Result<Vec<PathBuf>, WalkError> {
    let read_dir = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(WalkError::new(dir.to_path_buf(), source)),
    };
    Ok(read_dir.fold(Vec::new(), push_entry_path))
}

fn push_entry_path(
    mut acc: Vec<PathBuf>,
    entry: std::io::Result<std::fs::DirEntry>,
) -> Vec<PathBuf> {
    if let Ok(e) = entry {
        acc.push(e.path());
    }
    acc
}

fn collect_descendant(mut acc: Vec<PathBuf>, path: &Path) -> Result<Vec<PathBuf>, WalkError> {
    descend_entry(path, &mut acc)?;
    Ok(acc)
}

fn descend_entry(path: &Path, out: &mut Vec<PathBuf>) -> Result<(), WalkError> {
    if path.is_dir() {
        let nested = walk_rs_files(path)?;
        out.extend(nested);
    } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
        out.push(path.to_path_buf());
    }
    Ok(())
}

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
