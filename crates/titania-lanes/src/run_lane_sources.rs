//! Source discovery for embedded run-lane implementations.

use std::path::{Path, PathBuf};

use thiserror::Error;

/// Directory names that are not production Rust input for source-scan lanes.
const SKIP_DIRS: &[&str] = &[
    ".beads",
    ".git",
    ".moon",
    ".titania",
    ".worktrees",
    "benches",
    "examples",
    "target",
    "tests",
];

/// Filesystem traversal failures from source discovery.
#[derive(Debug, Error)]
pub enum SourceWalkError {
    /// Reading a directory failed.
    #[error("source walk failed at {path}: {source}")]
    ReadDir {
        /// Directory path being scanned.
        path: PathBuf,
        /// Underlying filesystem error.
        source: std::io::Error,
    },
    /// Reading a directory entry failed.
    #[error("source entry read failed at {path}: {source}")]
    Entry {
        /// Directory or entry path being scanned.
        path: PathBuf,
        /// Underlying filesystem error.
        source: std::io::Error,
    },
    /// A discovered source path did not remain under the target root.
    #[error("source path {path} is outside target root: {source}")]
    OutsideRoot {
        /// Source path being converted to a relative workspace path.
        path: PathBuf,
        /// Underlying prefix-stripping error.
        source: std::path::StripPrefixError,
    },
}

/// Collect production Rust sources relative to the target root.
///
/// # Errors
/// Returns [`SourceWalkError`] when traversal or path relativization fails.
pub fn collect_rust_sources(root: &Path) -> Result<Vec<PathBuf>, SourceWalkError> {
    let mut files = Vec::new();
    collect_rust_sources_into(root, root, &mut files)?;
    files.sort();
    Ok(files)
}

/// Recursively collect Rust sources under one directory.
///
/// # Errors
/// Returns [`SourceWalkError`] when traversal or path relativization fails.
fn collect_rust_sources_into(
    root: &Path,
    dir: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), SourceWalkError> {
    std::fs::read_dir(dir)
        .map_err(|source| SourceWalkError::ReadDir { path: dir.to_path_buf(), source })?
        .try_for_each(|entry| visit_source_entry(root, entry, files))
}

/// Visit one source directory entry.
///
/// # Errors
/// Returns [`SourceWalkError`] when entry metadata or recursion fails.
fn visit_source_entry(
    root: &Path,
    entry: std::io::Result<std::fs::DirEntry>,
    files: &mut Vec<PathBuf>,
) -> Result<(), SourceWalkError> {
    let entry =
        entry.map_err(|source| SourceWalkError::Entry { path: root.to_path_buf(), source })?;
    let path = entry.path();
    let file_type = entry
        .file_type()
        .map_err(|source| SourceWalkError::Entry { path: path.clone(), source })?;
    if file_type.is_dir() && !skip_dir(&path) {
        return collect_rust_sources_into(root, &path, files);
    }
    if file_type.is_file() && !skip_file(&path) && path.extension().is_some_and(|ext| ext == "rs") {
        push_relative_source(root, &path, files)?;
    }
    Ok(())
}

/// Append a source path relative to the target root.
///
/// # Errors
/// Returns [`SourceWalkError::OutsideRoot`] when the path escapes root.
fn push_relative_source(
    root: &Path,
    path: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), SourceWalkError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|source| SourceWalkError::OutsideRoot { path: path.to_path_buf(), source })?;
    files.push(relative.to_path_buf());
    Ok(())
}

/// Return true when *path* points at a directory excluded from source discovery.
fn skip_dir(path: &Path) -> bool {
    path.file_name().and_then(|name| name.to_str()).is_some_and(|name| SKIP_DIRS.contains(&name))
}

/// Return true when *path* points at a Rust build script excluded from source discovery.
fn skip_file(path: &Path) -> bool {
    path.file_name().and_then(|name| name.to_str()).is_some_and(|name| name == "build.rs")
}
