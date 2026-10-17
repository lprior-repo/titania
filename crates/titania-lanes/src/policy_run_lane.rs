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
        &crate::policy_scan::env_vars::real_env,
        &mut report,
    )?;
    Ok(report)
}

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

fn collect_manifest_paths(root: &Path) -> Result<Vec<PathBuf>, PolicyRunError> {
    let mut manifests = Vec::new();
    collect_manifest_paths_into(root, root, &mut manifests)?;
    manifests.sort();
    Ok(manifests)
}

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
    if file_type.is_file()
        && entry.file_name() == OsStr::new(MANIFEST_NAME)
        && belongs_to_target_workspace(parent, root, &path)
    {
        push_manifest_path(root, &path, manifests)?;
    }
    Ok(())
}

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

fn belongs_to_target_workspace(parent: &Path, root: &Path, path: &Path) -> bool {
    parent == root || !is_workspace_boundary(path)
}
