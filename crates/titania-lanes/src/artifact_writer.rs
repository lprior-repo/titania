//! Atomic JSON writer for v1 lane artifacts.
//!
//! A lane artifact is written under `.titania/out/<scope>/<lane>.json`
//! inside the evaluated target project. The writer uses a same-directory
//! temporary file followed by `rename` so readers never observe partial JSON.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use thiserror::Error;
use titania_core::{ArtifactOutcome, GateScope, Lane, LaneArtifact, LaneOutcome};

/// Errors returned while writing a lane artifact.
#[derive(Debug, Error)]
pub enum ArtifactWriterError {
    /// The supplied target root does not exist.
    #[error("target root does not exist: {path}")]
    MissingTargetRoot {
        /// Target root path.
        path: PathBuf,
    },
    /// The supplied target root exists but is not a directory.
    #[error("target root is not a directory: {path}")]
    TargetRootNotDirectory {
        /// Target root path.
        path: PathBuf,
    },
    /// Directory creation failed.
    #[error("failed to create artifact directory {path}: {source}")]
    CreateDir {
        /// Directory path.
        path: PathBuf,
        /// Source I/O error.
        source: io::Error,
    },
    /// JSON serialization failed.
    #[error("failed to serialize lane artifact JSON: {0}")]
    Serialize(#[from] serde_json::Error),
    /// Temporary artifact write failed.
    #[error("failed to write temporary lane artifact {path}: {source}")]
    WriteTemp {
        /// Temporary file path.
        path: PathBuf,
        /// Source I/O error.
        source: io::Error,
    },
    /// Atomic artifact rename failed.
    #[error("failed to rename temporary lane artifact {from} to {to}: {source}")]
    Rename {
        /// Temporary file path.
        from: PathBuf,
        /// Final file path.
        to: PathBuf,
        /// Source I/O error.
        source: io::Error,
    },
}

/// Write one lane outcome to `.titania/out/<scope>/<lane>.json`.
///
/// The returned path is the final artifact path inside `target_root`.
///
/// # Errors
///
/// Returns [`ArtifactWriterError`] when `target_root` is missing or not a
/// directory, when the scope is unsupported by this writer, when parent
/// directory creation fails, when JSON serialization fails, or when the
/// temporary write/rename fails.
pub fn write_lane_artifact(
    target_root: &Path,
    scope: GateScope,
    lane: Lane,
    outcome: &LaneOutcome,
) -> Result<PathBuf, ArtifactWriterError> {
    validate_target_root(target_root)?;

    let artifact_dir = target_root.join(".titania").join("out").join(scope_dir(scope));
    fs::create_dir_all(&artifact_dir)
        .map_err(|source| ArtifactWriterError::CreateDir { path: artifact_dir.clone(), source })?;

    let final_path = artifact_dir.join(lane_file_name(lane));
    let temp_path = artifact_dir.join(temp_file_name(lane));
    let artifact = LaneArtifact::new(lane, ArtifactOutcome::from(outcome));
    let payload = serde_json::to_vec_pretty(&artifact)?;

    fs::write(&temp_path, payload)
        .map_err(|source| ArtifactWriterError::WriteTemp { path: temp_path.clone(), source })?;

    fs::rename(&temp_path, &final_path).map_err(|source| ArtifactWriterError::Rename {
        from: temp_path,
        to: final_path.clone(),
        source,
    })?;

    Ok(final_path)
}

/// Validate that a target root is present and directory-shaped.
///
/// # Errors
///
/// Returns [`ArtifactWriterError::MissingTargetRoot`] or
/// [`ArtifactWriterError::TargetRootNotDirectory`] when the supplied path
/// cannot contain Titania output artifacts.
fn validate_target_root(target_root: &Path) -> Result<(), ArtifactWriterError> {
    if !target_root.exists() {
        return Err(ArtifactWriterError::MissingTargetRoot { path: target_root.to_path_buf() });
    }

    if target_root.is_dir() {
        Ok(())
    } else {
        Err(ArtifactWriterError::TargetRootNotDirectory { path: target_root.to_path_buf() })
    }
}

/// Return the stable directory name for a v1 gate scope.
const fn scope_dir(scope: GateScope) -> &'static str {
    match scope {
        GateScope::Edit => "edit",
        GateScope::Prepush => "prepush",
        GateScope::Release => "release",
        GateScope::Full => "full",
    }
}

const fn lane_stem(lane: Lane) -> &'static str {
    match lane {
        Lane::Fmt => "fmt",
        Lane::Compile => "compile",
        Lane::Clippy => "clippy",
        Lane::AstGrep => "ast-grep",
        Lane::Dylint => "dylint",
        Lane::PanicScan => "panic-scan",
        Lane::PolicyScan => "policy-scan",
        Lane::Test => "test",
        Lane::Deny => "deny",
        Lane::Build => "build",
        Lane::Kani => "kani",
        Lane::Mutants => "mutants",
    }
}

fn lane_file_name(lane: Lane) -> String {
    [lane_stem(lane), ".json"].concat()
}

fn temp_file_name(lane: Lane) -> String {
    [".titania-out-", lane_stem(lane), ".tmp"].concat()
}
