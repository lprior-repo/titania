//! Bounded source discovery for embedded run-lane implementations.
//!
//! Production supplies fixed policy limits. One traversal step polls at most
//! one directory entry; rejected entries and directory exhaustion consume work
//! too. No recursive call or partial-success truncation is used. These are
//! application work bounds, not deadlines for filesystem calls or an RSS cap.

use std::collections::TryReserveError;
use std::fs::{DirEntry, ReadDir};
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
    "fixtures",
    "target",
    "tests",
];

/// Fixed production limits; changing these is a reviewed policy change.
/// Root counts as one directory level. Path bytes use the OS representation.
const SOURCE_WALK_LIMITS: WalkLimits = WalkLimits {
    steps: 250_000,
    depth: 64,
    files: 50_000,
    path_bytes: 4_096,
    admitted_path_bytes: 67_108_864,
};

/// Source-discovery resource whose configured limit was exhausted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceWalkResource {
    /// Directory-entry polls, including rejected entries and end-of-directory.
    Steps,
    /// Simultaneously open directory frames, including the root.
    DirectoryDepth,
    /// Collected production Rust paths.
    SourceFiles,
    /// Bytes in one discovered absolute or root-relative path.
    PathBytes,
    /// Cumulative path bytes admitted to directory frames and source results.
    AdmittedPathBytes,
}

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
    /// Traversal stopped without returning a partial source inventory.
    #[error("source walk exhausted {resource:?} limit {limit}; inventory is incomplete")]
    LimitExceeded {
        /// Resource that reached its policy limit.
        resource: SourceWalkResource,
        /// Configured maximum for that resource.
        limit: usize,
    },
    /// Reserving collection storage failed.
    #[error("source walk could not reserve collection storage: {source}")]
    Allocation {
        /// Allocation failure reported by the collection.
        source: TryReserveError,
    },
    /// An exhausted directory disappeared from the owned traversal stack.
    #[error("source walk stack invariant violated")]
    InvalidTraversalStack,
}

#[derive(Debug, Clone, Copy)]
struct WalkLimits {
    steps: usize,
    depth: usize,
    files: usize,
    path_bytes: usize,
    admitted_path_bytes: usize,
}

#[derive(Debug)]
struct DirectoryFrame {
    path: PathBuf,
    entries: ReadDir,
}

#[derive(Debug)]
struct SourceWalk {
    frames: Vec<DirectoryFrame>,
    files: Vec<PathBuf>,
    remaining_path_bytes: usize,
}

#[derive(Debug)]
enum WalkProgress {
    Continue,
    Complete,
}

/// Collect production Rust sources relative to the target root.
///
/// # Errors
/// Returns [`SourceWalkError`] for I/O, allocation, invalid paths, or exhausted
/// policy limits. A limit failure never returns a partial successful inventory.
pub fn collect_rust_sources(root: &Path) -> Result<Vec<PathBuf>, SourceWalkError> {
    collect_with_limits(root, SOURCE_WALK_LIMITS)
}

/// Run the fixed-work traversal; tests may supply small boundary limits.
///
/// # Errors
/// Returns a typed source-walk failure instead of a partial inventory.
fn collect_with_limits(root: &Path, limits: WalkLimits) -> Result<Vec<PathBuf>, SourceWalkError> {
    require_room(0, limits.steps, SourceWalkResource::Steps)?;
    let mut walk = SourceWalk {
        frames: Vec::new(),
        files: Vec::new(),
        remaining_path_bytes: limits.admitted_path_bytes,
    };
    walk.enter_directory(root, limits)?;
    // The range bounds raw polls, not the number of matching Rust files.
    let completion = (0..limits.steps).find_map(|_| match walk.advance(root, limits) {
        Ok(WalkProgress::Continue) => None,
        Ok(WalkProgress::Complete) => Some(Ok(())),
        Err(error) => Some(Err(error)),
    });
    match completion {
        Some(result) => result?,
        None => return Err(limit_error(SourceWalkResource::Steps, limits.steps)),
    }
    walk.files.sort();
    Ok(walk.files)
}

impl SourceWalk {
    /// Poll one frame; filesystem blocking time is outside this work bound.
    ///
    /// # Errors
    /// Returns entry, resource, or traversal-state failures.
    fn advance(&mut self, root: &Path, limits: WalkLimits) -> Result<WalkProgress, SourceWalkError> {
        let entry = match self.frames.last_mut() {
            None => return Ok(WalkProgress::Complete),
            Some(frame) => match frame.entries.next() {
                Some(entry) => Some(entry.map_err(|source| SourceWalkError::Entry {
                    path: frame.path.clone(),
                    source,
                })?),
                None => None,
            },
        };
        match entry {
            Some(entry) => {
                self.visit(root, &entry, limits)?;
                Ok(WalkProgress::Continue)
            }
            None => self.finish_directory(),
        }
    }

    /// Close a completed frame without recursive unwinding.
    ///
    /// # Errors
    /// Returns an invariant error if no owned directory frame exists.
    fn finish_directory(&mut self) -> Result<WalkProgress, SourceWalkError> {
        match self.frames.pop() {
            Some(_completed) if self.frames.is_empty() => Ok(WalkProgress::Complete),
            Some(_completed) => Ok(WalkProgress::Continue),
            None => Err(SourceWalkError::InvalidTraversalStack),
        }
    }

    /// Classify an entry only after checking its path length.
    ///
    /// # Errors
    /// Returns metadata, resource, or path errors.
    fn visit(
        &mut self,
        root: &Path,
        entry: &DirEntry,
        limits: WalkLimits,
    ) -> Result<(), SourceWalkError> {
        let path = entry.path();
        let _path_bytes = check_path_length(&path, limits)?;
        let kind = entry
            .file_type()
            .map_err(|source| SourceWalkError::Entry { path: path.clone(), source })?;
        if kind.is_dir() && !skip_dir(&path) {
            return self.enter_directory(&path, limits);
        }
        if kind.is_file() && !skip_file(&path) && path.extension().is_some_and(|ext| ext == "rs") {
            self.push_source(root, &path, limits)?;
        }
        Ok(())
    }

    /// Admit one directory, bounding both stack depth and path storage.
    ///
    /// # Errors
    /// Returns policy, allocation, or directory-open errors.
    fn enter_directory(&mut self, path: &Path, limits: WalkLimits) -> Result<(), SourceWalkError> {
        require_room(self.frames.len(), limits.depth, SourceWalkResource::DirectoryDepth)?;
        self.admit_path(path, limits)?;
        self.frames.try_reserve(1).map_err(|source| SourceWalkError::Allocation { source })?;
        let entries = std::fs::read_dir(path)
            .map_err(|source| SourceWalkError::ReadDir { path: path.to_path_buf(), source })?;
        self.frames.push(DirectoryFrame { path: path.to_path_buf(), entries });
        Ok(())
    }

    /// Retain a relative Rust path only when all inventory budgets allow it.
    ///
    /// # Errors
    /// Returns path, resource, or collection-allocation failures.
    fn push_source(
        &mut self,
        root: &Path,
        path: &Path,
        limits: WalkLimits,
    ) -> Result<(), SourceWalkError> {
        require_room(self.files.len(), limits.files, SourceWalkResource::SourceFiles)?;
        let relative = path
            .strip_prefix(root)
            .map_err(|source| SourceWalkError::OutsideRoot { path: path.to_path_buf(), source })?;
        self.admit_path(relative, limits)?;
        self.files.try_reserve(1).map_err(|source| SourceWalkError::Allocation { source })?;
        self.files.push(relative.to_path_buf());
        Ok(())
    }

    /// Charge path storage cumulatively; exiting directories does not refund it.
    ///
    /// # Errors
    /// Returns a per-path or cumulative path-byte limit failure.
    fn admit_path(&mut self, path: &Path, limits: WalkLimits) -> Result<(), SourceWalkError> {
        let bytes = check_path_length(path, limits)?;
        self.remaining_path_bytes = self.remaining_path_bytes.checked_sub(bytes).ok_or_else(|| {
            limit_error(SourceWalkResource::AdmittedPathBytes, limits.admitted_path_bytes)
        })?;
        Ok(())
    }
}

/// Check OS-encoded path bytes, not Unicode characters.
///
/// # Errors
/// Returns a limit error if the path is too large for the policy.
fn check_path_length(path: &Path, limits: WalkLimits) -> Result<usize, SourceWalkError> {
    let bytes = path.as_os_str().as_encoded_bytes().len();
    if bytes > limits.path_bytes {
        return Err(limit_error(SourceWalkResource::PathBytes, limits.path_bytes));
    }
    Ok(bytes)
}

/// Ensure a collection has room before adding a resource.
///
/// # Errors
/// Returns the exact resource whose next item would exceed its bound.
fn require_room(
    current: usize,
    limit: usize,
    resource: SourceWalkResource,
) -> Result<(), SourceWalkError> {
    if current >= limit {
        return Err(limit_error(resource, limit));
    }
    Ok(())
}

/// Build a typed limit error without fabricating an observed count.
const fn limit_error(resource: SourceWalkResource, limit: usize) -> SourceWalkError {
    SourceWalkError::LimitExceeded { resource, limit }
}

/// Return true when *path* points at a directory excluded from source discovery.
fn skip_dir(path: &Path) -> bool {
    path.file_name().and_then(|name| name.to_str()).is_some_and(|name| SKIP_DIRS.contains(&name))
}

/// Return true when *path* points at a Rust build script excluded from source discovery.
fn skip_file(path: &Path) -> bool {
    path.file_name().and_then(|name| name.to_str()).is_some_and(|name| name == "build.rs")
}

#[cfg(test)]
#[path = "run_lane_sources/tests.rs"]
mod tests;
