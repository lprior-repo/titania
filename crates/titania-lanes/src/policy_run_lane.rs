//! Library runner for the v1 policy-scan lane.

use std::{
    ffi::OsStr,
    io,
    path::{Path, PathBuf, StripPrefixError},
    string::FromUtf8Error,
};

use thiserror::Error;

use titania_core::{ManifestStatus, TargetProject, classify_manifest};

use crate::{
    LaneReport, RuleIdError,
    policy_scan::{exceptions::load_exceptions, scan_policy_inputs_with_exceptions},
};

const DATE_TOOL: &str = "date";
const DATE_FORMAT: &str = "+%F";
const MANIFEST_NAME: &str = "Cargo.toml";
const SKIP_DIRS: &[&str] = &[".beads", ".git", ".moon", ".titania", ".worktrees", "target"];

/// Run the policy scanner against all workspace manifests.
///
/// # Errors
/// Returns [`PolicyRunError`] when date, manifest walk, or policy scanning fails.
pub(super) fn run(target: &TargetProject) -> Result<LaneReport, PolicyRunError> {
    let root = target.as_std_path();
    let today = policy_date(target)?;
    let manifests = collect_manifest_paths(root)?;
    let mut report = LaneReport::new();
    let exceptions = load_exceptions(root, &today, &mut report)?;
    scan_policy_inputs_with_exceptions(
        root,
        manifests.iter().map(PathBuf::as_path),
        &exceptions,
        &mut report,
    )?;
    Ok(report)
}

/// Read the current UTC date from the platform `date` command.
///
/// # Errors
///
/// Returns [`PolicyRunError`] when the command fails or emits non-UTF-8 output.
pub(super) fn policy_date(target: &TargetProject) -> Result<String, PolicyRunError> {
    let output = crate::command::CommandIn::new(target, DATE_TOOL)
        .and_then(|mut cmd| cmd.arg(DATE_FORMAT).run_capture_raw())
        .map_err(PolicyRunError::DateCommand)?;
    if !output.success() {
        return Err(PolicyRunError::DateStatus(output.status().to_string()));
    }
    String::from_utf8(output.into_stdout())
        .map_err(PolicyRunError::DateUtf8)
        .map(|stdout| stdout.trim().to_owned())
}

/// Collect workspace `Cargo.toml` paths relative to the target root.
/// Discovers only manifests that belong to the target workspace. A nested
/// directory whose own `Cargo.toml` declares a `[workspace]` table is a
/// separate workspace boundary (for example a fixture or template
/// standalone workspace); its own manifest and any package manifests
/// beneath it are excluded from the target's policy-scan.
///
/// # Errors
/// Returns [`PolicyRunError`] when manifest directory traversal or
/// manifest read for boundary detection fails.
fn collect_manifest_paths(root: &Path) -> Result<Vec<PathBuf>, PolicyRunError> {
    let mut manifests = Vec::new();
    collect_manifest_paths_into(root, root, &mut manifests)?;
    manifests.sort();
    Ok(manifests)
}

/// Recursively collect manifest paths under one directory.
///
/// Stops descending when `dir` itself contains a `Cargo.toml` declaring
/// a `[workspace]` table (a nested workspace boundary that is not the
/// target root).
///
/// # Errors
/// Returns [`PolicyRunError`] when directory traversal fails.
fn collect_manifest_paths_into(
    root: &Path,
    dir: &Path,
    manifests: &mut Vec<PathBuf>,
) -> Result<(), PolicyRunError> {
    if dir != root && is_workspace_boundary(&dir.join(MANIFEST_NAME)) {
        return Ok(());
    }
    std::fs::read_dir(dir)
        .map_err(|source| PolicyRunError::ManifestWalk { path: dir.to_path_buf(), source })?
        .try_for_each(|entry| visit_dir_entry(root, dir, entry, manifests))
}

/// Visit one directory entry during manifest discovery.
///
/// # Errors
/// Returns [`PolicyRunError`] when entry metadata or recursive walking fails.
fn visit_dir_entry(
    root: &Path,
    parent: &Path,
    entry: io::Result<std::fs::DirEntry>,
    manifests: &mut Vec<PathBuf>,
) -> Result<(), PolicyRunError> {
    let entry = entry
        .map_err(|source| PolicyRunError::ManifestWalk { path: root.to_path_buf(), source })?;
    let path = entry.path();
    let file_type = entry
        .file_type()
        .map_err(|source| PolicyRunError::ManifestWalk { path: path.clone(), source })?;
    if file_type.is_dir() && !is_skipped_dir(&path) {
        return collect_manifest_paths_into(root, &path, manifests);
    }
    if file_type.is_file() && entry.file_name() == OsStr::new(MANIFEST_NAME) {
        if parent != root && is_workspace_boundary(&path) {
            return Ok(());
        }
        push_manifest_path(root, &path, manifests)?;
    }
    Ok(())
}

/// Append one manifest path relative to the target root.
///
/// # Errors
/// Returns [`PolicyRunError::ManifestOutsideRoot`] when the path escapes root.
fn push_manifest_path(
    root: &Path,
    path: &Path,
    manifests: &mut Vec<PathBuf>,
) -> Result<(), PolicyRunError> {
    let relative = path.strip_prefix(root).map_err(|source| {
        PolicyRunError::ManifestOutsideRoot { path: path.to_path_buf(), source }
    })?;
    manifests.push(relative.to_path_buf());
    Ok(())
}

fn is_skipped_dir(path: &Path) -> bool {
    path.file_name().and_then(OsStr::to_str).is_some_and(|name| SKIP_DIRS.contains(&name))
}

/// Detect whether a `Cargo.toml` declares its own `[workspace]` table.
///
/// A manifest that opens a workspace is its own workspace boundary; a
/// consumer workspace should not classify it (or anything beneath it) as
/// a member of the consumer's workspace. Missing or unreadable files do
/// not count as a boundary, so member discovery still proceeds.
fn is_workspace_boundary(manifest_path: &Path) -> bool {
    std::fs::read_to_string(manifest_path)
        .is_ok_and(|content| matches!(classify_manifest(&content), ManifestStatus::Workspace))
}

#[derive(Debug, Error)]
pub(super) enum PolicyRunError {
    #[error("policy date command exited with {0}")]
    DateStatus(String),
    #[error("policy date command emitted non-UTF-8: {0}")]
    DateUtf8(#[source] FromUtf8Error),
    #[error("failed to execute date command: {0}")]
    DateCommand(crate::command::LaneError),
    #[error("failed to walk manifests at {path}: {source}")]
    ManifestWalk {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("manifest {path} is outside target root: {source}")]
    ManifestOutsideRoot {
        path: PathBuf,
        #[source]
        source: StripPrefixError,
    },
    #[error("rule id configuration error: {0}")]
    RuleId(#[from] RuleIdError),
}
