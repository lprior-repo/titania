use std::process::ExitStatus;

use super::{LaneError, OutputStream};

/// The result of a captured subprocess run.
#[derive(Debug)]
pub struct CommandOutput {
    /// Exit status reported by the OS for the child.
    pub status: ExitStatus,
    /// Captured stdout bytes.
    pub stdout: Vec<u8>,
    /// Captured stderr bytes.
    pub stderr: Vec<u8>,
    /// Program name for diagnostic context.
    program: String,
}

impl CommandOutput {
    /// Build a `CommandOutput` from already-collected pieces.
    pub(super) const fn new(
        status: ExitStatus,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        program: String,
    ) -> Self {
        Self { status, stdout, stderr, program }
    }

    /// Whether the subprocess exited successfully (status code 0).
    #[must_use]
    pub fn success(&self) -> bool {
        self.status.success()
    }

    /// Decode stdout as UTF-8.
    ///
    /// # Errors
    /// Returns [`LaneError::NonUtf8Output`] when stdout is not UTF-8.
    pub fn stdout_str(&self) -> Result<&str, LaneError> {
        std::str::from_utf8(&self.stdout).map_err(|e| {
            let _ = e;
            LaneError::NonUtf8Output {
                program: self.program.clone(),
                stream: OutputStream::Stdout,
            }
        })
    }

    /// Decode stderr as UTF-8.
    ///
    /// # Errors
    /// Returns [`LaneError::NonUtf8Output`] when stderr is not UTF-8.
    pub fn stderr_str(&self) -> Result<&str, LaneError> {
        std::str::from_utf8(&self.stderr).map_err(|e| {
            let _ = e;
            LaneError::NonUtf8Output {
                program: self.program.clone(),
                stream: OutputStream::Stderr,
            }
        })
    }

    /// Convert a non-zero status to [`LaneError::NonZeroExit`].
    ///
    /// # Errors
    /// Returns [`LaneError::NonUtf8Output`] when stderr decoding fails for a
    /// non-zero exit, or [`LaneError::NonZeroExit`] when the subprocess exits
    /// unsuccessfully.
    pub fn into_result(self) -> Result<Self, LaneError> {
        if self.status.success() {
            Ok(self)
        } else {
            let code = self.status.code();
            let stderr = self.stderr_str()?.to_owned();
            Err(LaneError::NonZeroExit { program: self.program, code, stderr })
        }
    }
}