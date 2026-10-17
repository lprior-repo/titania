use std::{
    fs, io,
    path::{Path, PathBuf},
};

use thiserror::Error;
use titania_core::{ArtifactOutcome, GateScope, Lane, LaneArtifact, LaneOutcome};

#[derive(Debug, Error)]
pub enum ArtifactWriterError {
    #[error("target root does not exist: {path}")]
    MissingTargetRoot { path: PathBuf },
    #[error("target root is not a directory: {path}")]
    TargetRootNotDirectory { path: PathBuf },
    #[error("failed to create artifact directory {path}: {source}")]
    CreateDir { path: PathBuf, source: io::Error },
    #[error("failed to serialize lane artifact JSON: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("failed to write temporary lane artifact {path}: {source}")]
    WriteTemp { path: PathBuf, source: io::Error },
    #[error("failed to rename temporary lane artifact {from} to {to}: {source}")]
    Rename { from: PathBuf, to: PathBuf, source: io::Error },
}

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
