//! Path and source-file helpers for the embedded ast-grep lane.

use std::{
    fs,
    path::{Path, PathBuf},
};

use titania_core::WorkspacePath;

use super::AstGrepLaneError;

/// Read a source file after resolving it against the target root.
///
/// # Errors
/// Returns [`AstGrepLaneError::ReadFile`] when the file cannot be read.
pub(super) fn read_source(
    target_root: Option<&Path>,
    path: &Path,
) -> Result<String, AstGrepLaneError> {
    let actual_path = source_path(target_root, path);
    fs::read_to_string(&actual_path)
        .map_err(|source| AstGrepLaneError::ReadFile { path: path_display(&actual_path), source })
}

/// Render an input path as a checked workspace path.
///
/// # Errors
/// Returns path conversion errors when absolute paths cannot be made relative.
pub(super) fn workspace_path(
    target_root: Option<&Path>,
    path: &Path,
) -> Result<WorkspacePath, AstGrepLaneError> {
    if !path.is_absolute() {
        return WorkspacePath::new(&path_to_str(path)?).map_err(Into::into);
    }
    if let Some(relative) = root_relative_path(target_root, path)? {
        return WorkspacePath::new(&relative).map_err(Into::into);
    }
    let rendered = path_to_str(path)?;
    let relative = fixture_relative_path(&rendered)
        .ok_or(AstGrepLaneError::AbsolutePathWithoutRoot { path: rendered })?;
    WorkspacePath::new(&relative).map_err(Into::into)
}

fn source_path(target_root: Option<&Path>, path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    target_root.map_or_else(|| path.to_path_buf(), |root| root.join(path))
}

/// Strip the target root from an absolute source path when possible.
///
/// # Errors
/// Returns [`AstGrepLaneError::NonUtf8Path`] when the relative path is non-UTF-8.
fn root_relative_path(
    target_root: Option<&Path>,
    path: &Path,
) -> Result<Option<String>, AstGrepLaneError> {
    target_root
        .and_then(|root| path.strip_prefix(root).ok())
        .map_or(Ok(None), |relative| path_to_str(relative).map(Some))
}

fn fixture_relative_path(rendered: &str) -> Option<String> {
    let parts = rendered.split('/').collect::<Vec<_>>();
    parts
        .iter()
        .position(|part| *part == "ast_grep")
        .and_then(|index| index.checked_add(1))
        .map(|start| parts.into_iter().skip(start).collect::<Vec<_>>().join("/"))
}

/// Render a path as UTF-8.
///
/// # Errors
/// Returns [`AstGrepLaneError::NonUtf8Path`] when `path` is not valid UTF-8.
fn path_to_str(path: &Path) -> Result<String, AstGrepLaneError> {
    path.to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| AstGrepLaneError::NonUtf8Path { path: path_display(path) })
}

fn path_display(path: &Path) -> String {
    path.display().to_string()
}
