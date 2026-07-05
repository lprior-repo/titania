use std::{
    fs,
    path::{Path, PathBuf},
};

use thiserror::Error;
use titania_core::TargetProject;
use titania_lanes::{CommandIn, LaneError};

use super::registry::ProofTarget;

#[derive(Debug, Error)]
pub(crate) enum VerusToolError {
    #[error("failed to prepare verus: {source}")]
    Prepare { source: LaneError },
    #[error("failed to run verus: {source}")]
    Run { source: LaneError },
    #[error("cannot write {}: {source}", path.display())]
    WriteLog { path: PathBuf, source: std::io::Error },
    #[error("verus target {target} failed with status {status:?}; see {}", log_path.display())]
    Status { target: String, status: Option<i32>, log_path: PathBuf },
}

#[must_use]
pub(crate) fn verus_on_path(target: &TargetProject) -> bool {
    let Ok(mut command) = CommandIn::new(target, "verus") else {
        return false;
    };
    let _ = command.inherit_env();
    let _ = command.arg("--version");
    command.run_capture_raw().is_ok()
}

/// Run Verus for one proof target and persist its stdout/stderr log.
///
/// # Errors
///
/// Returns command construction, command execution, log write, or non-zero
/// Verus status failures.
pub(crate) fn run_verus_target(
    target: &TargetProject,
    proof_target: &ProofTarget,
    evidence_dir: &Path,
) -> Result<(), VerusToolError> {
    let log_path = evidence_dir.join(format!("{}.log", safe_log_name(proof_target.path())));
    let mut command =
        CommandIn::new(target, "verus").map_err(|source| VerusToolError::Prepare { source })?;
    let _ = command.inherit_env();
    let _ = command.arg(proof_target.path());
    let output = command.run_capture_raw().map_err(|source| VerusToolError::Run { source })?;
    write_log(&log_path, output.stdout(), output.stderr())?;
    if output.success() {
        Ok(())
    } else {
        Err(VerusToolError::Status {
            target: proof_target.path().to_owned(),
            status: output.status().code(),
            log_path,
        })
    }
}

fn safe_log_name(proof_target: &str) -> String {
    proof_target
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect()
}

/// Write a Verus output log.
///
/// # Errors
///
/// Returns filesystem write failures with the target path included.
fn write_log(path: &Path, stdout: &[u8], stderr: &[u8]) -> Result<(), VerusToolError> {
    let mut body = String::from_utf8_lossy(stdout).into_owned();
    body.push_str("\n--- stderr ---\n");
    body.push_str(&String::from_utf8_lossy(stderr));
    fs::write(path, body)
        .map_err(|source| VerusToolError::WriteLog { path: path.to_path_buf(), source })
}
