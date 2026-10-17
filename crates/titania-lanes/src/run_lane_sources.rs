use std::{
    collections::TryReserveError,
    fs::{DirEntry, ReadDir},
    path::{Path, PathBuf},
};

use thiserror::Error;

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

const SOURCE_WALK_LIMITS: WalkLimits = WalkLimits {
    steps: 250_000,
    depth: 64,
    files: 50_000,
    path_bytes: 4_096,
    admitted_path_bytes: 67_108_864,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceWalkResource {
    Steps,
    DirectoryDepth,
    SourceFiles,
    PathBytes,
    AdmittedPathBytes,
}

#[derive(Debug, Error)]
pub enum SourceWalkError {
    #[error("source walk failed at {path}: {source}")]
    ReadDir { path: PathBuf, source: std::io::Error },
    #[error("source entry read failed at {path}: {source}")]
    Entry { path: PathBuf, source: std::io::Error },
    #[error("source path {path} is outside target root: {source}")]
    OutsideRoot { path: PathBuf, source: std::path::StripPrefixError },
    #[error("source walk exhausted {resource:?} limit {limit}; inventory is incomplete")]
    LimitExceeded { resource: SourceWalkResource, limit: usize },
    #[error("source walk could not reserve collection storage: {source}")]
    Allocation { source: TryReserveError },
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

pub fn collect_rust_sources(root: &Path) -> Result<Vec<PathBuf>, SourceWalkError> {
    collect_with_limits(root, SOURCE_WALK_LIMITS)
}

fn collect_with_limits(root: &Path, limits: WalkLimits) -> Result<Vec<PathBuf>, SourceWalkError> {
    require_room(0, limits.steps, SourceWalkResource::Steps)?;
    let mut walk = SourceWalk {
        frames: Vec::new(),
        files: Vec::new(),
        remaining_path_bytes: limits.admitted_path_bytes,
    };
    walk.enter_directory(root, limits)?;
    let completion = (0..limits.steps).find_map(|_| match advance_walk(&mut walk, root, limits) {
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

fn advance_walk(
    walk: &mut SourceWalk,
    root: &Path,
    limits: WalkLimits,
) -> Result<WalkProgress, SourceWalkError> {
    if walk.frames.is_empty() {
        return Ok(WalkProgress::Complete);
    }
    match poll_top(walk)? {
        Some(entry) => walk.visit_entry(root, &entry, limits),
        None => walk.finish_directory(),
    }
}

fn enter_if_wanted(
    walk: &mut SourceWalk,
    path: &Path,
    kind: std::fs::FileType,
    limits: WalkLimits,
) -> Result<(), SourceWalkError> {
    if !kind.is_dir() || skip_dir(path) {
        return Ok(());
    }
    walk.enter_directory(path, limits)
}

fn push_if_wanted(
    walk: &mut SourceWalk,
    root: &Path,
    path: &Path,
    kind: std::fs::FileType,
    limits: WalkLimits,
) -> Result<(), SourceWalkError> {
    let wanted =
        kind.is_file() && !skip_file(path) && path.extension().is_some_and(|ext| ext == "rs");
    if !wanted {
        return Ok(());
    }
    walk.push_source(root, path, limits)
}

fn poll_top(walk: &mut SourceWalk) -> Result<Option<DirEntry>, SourceWalkError> {
    let Some(frame) = walk.frames.last_mut() else {
        return Ok(None);
    };
    frame.entries.next().map_or(Ok(None), |entry| {
        entry
            .map(Some)
            .map_err(|source| SourceWalkError::Entry { path: frame.path.clone(), source })
    })
}

impl SourceWalk {
    fn visit_entry(
        &mut self,
        root: &Path,
        entry: &DirEntry,
        limits: WalkLimits,
    ) -> Result<WalkProgress, SourceWalkError> {
        self.visit(root, entry, limits)?;
        Ok(WalkProgress::Continue)
    }

    fn finish_directory(&mut self) -> Result<WalkProgress, SourceWalkError> {
        match self.frames.pop() {
            Some(_completed) if self.frames.is_empty() => Ok(WalkProgress::Complete),
            Some(_completed) => Ok(WalkProgress::Continue),
            None => Err(SourceWalkError::InvalidTraversalStack),
        }
    }

    fn visit(
        &mut self,
        root: &Path,
        entry: &DirEntry,
        limits: WalkLimits,
    ) -> Result<(), SourceWalkError> {
        let path = entry.path();
        let _within_budget = check_path_length(&path, limits)?;
        let kind = entry
            .file_type()
            .map_err(|source| SourceWalkError::Entry { path: path.clone(), source })?;
        enter_if_wanted(self, &path, kind, limits)?;
        push_if_wanted(self, root, &path, kind, limits)
    }

    fn enter_directory(&mut self, path: &Path, limits: WalkLimits) -> Result<(), SourceWalkError> {
        require_room(self.frames.len(), limits.depth, SourceWalkResource::DirectoryDepth)?;
        self.admit_path(path, limits)?;
        self.frames.try_reserve(1).map_err(|source| SourceWalkError::Allocation { source })?;
        let entries = std::fs::read_dir(path)
            .map_err(|source| SourceWalkError::ReadDir { path: path.to_path_buf(), source })?;
        self.frames.push(DirectoryFrame { path: path.to_path_buf(), entries });
        Ok(())
    }

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

    fn admit_path(&mut self, path: &Path, limits: WalkLimits) -> Result<(), SourceWalkError> {
        let bytes = check_path_length(path, limits)?;
        let exhausted =
            limit_error(SourceWalkResource::AdmittedPathBytes, limits.admitted_path_bytes);
        let remaining = self.remaining_path_bytes.checked_sub(bytes).ok_or(exhausted)?;
        self.remaining_path_bytes = remaining;
        Ok(())
    }
}

fn check_path_length(path: &Path, limits: WalkLimits) -> Result<usize, SourceWalkError> {
    let bytes = path.as_os_str().as_encoded_bytes().len();
    if bytes > limits.path_bytes {
        return Err(limit_error(SourceWalkResource::PathBytes, limits.path_bytes));
    }
    Ok(bytes)
}

const fn require_room(
    current: usize,
    limit: usize,
    resource: SourceWalkResource,
) -> Result<(), SourceWalkError> {
    if current >= limit {
        return Err(limit_error(resource, limit));
    }
    Ok(())
}

const fn limit_error(resource: SourceWalkResource, limit: usize) -> SourceWalkError {
    SourceWalkError::LimitExceeded { resource, limit }
}

fn skip_dir(path: &Path) -> bool {
    path.file_name().and_then(|name| name.to_str()).is_some_and(|name| SKIP_DIRS.contains(&name))
}

fn skip_file(path: &Path) -> bool {
    path.file_name().and_then(|name| name.to_str()).is_some_and(|name| name == "build.rs")
}

#[cfg(test)]
#[path = "run_lane_sources/tests.rs"]
mod tests;
