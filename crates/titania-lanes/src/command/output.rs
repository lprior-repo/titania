use std::process::ExitStatus;

use super::{LaneError, OutputStream};

/// The result of a captured subprocess run.
#[derive(Debug)]
pub struct CommandOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    program: String,
}

impl CommandOutput {
    pub(super) const fn new(
        status: ExitStatus,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        program: String,
    ) -> Self {
        Self { status, stdout, stderr, program }
    }

    /// `true` when the subprocess exited successfully.
    #[must_use]
    pub fn success(&self) -> bool {
        self.status.success()
    }
    /// Borrow the process exit status.
    #[must_use]
    pub const fn status(&self) -> ExitStatus {
        self.status
    }

    /// Borrow raw stdout bytes.
    #[must_use]
    pub fn stdout(&self) -> &[u8] {
        &self.stdout
    }

    /// Borrow raw stderr bytes.
    #[must_use]
    pub fn stderr(&self) -> &[u8] {
        &self.stderr
    }

    /// Take ownership of stdout bytes.
    #[must_use]
    pub fn into_stdout(self) -> Vec<u8> {
        self.stdout
    }

    /// Take ownership of stderr bytes.
    #[must_use]
    pub fn into_stderr(self) -> Vec<u8> {
        self.stderr
    }

    /// Decode stdout as UTF-8.
    ///
    /// # Errors
    /// Returns [`LaneError::NonUtf8Output`] when stdout is not UTF-8.
    pub fn stdout_str(&self) -> Result<&str, LaneError> {
        std::str::from_utf8(&self.stdout).map_err(|_utf8_err| LaneError::NonUtf8Output {
            program: self.program.clone(),
            stream: OutputStream::Stdout,
        })
    }

    /// Decode stderr as UTF-8.
    ///
    /// # Errors
    /// Returns [`LaneError::NonUtf8Output`] when stderr is not UTF-8.
    pub fn stderr_str(&self) -> Result<&str, LaneError> {
        std::str::from_utf8(&self.stderr).map_err(|_utf8_err| LaneError::NonUtf8Output {
            program: self.program.clone(),
            stream: OutputStream::Stderr,
        })
    }

    /// Convert a non-zero status to [`LaneError::NonZeroExit`].
    ///
    /// # Errors
    /// Returns [`LaneError::NonUtf8Output`] when stderr decoding fails for a
    /// non-zero exit, or [`LaneError::NonZeroExit`] when the subprocess exits
    /// unsuccessfully.
    pub fn into_result(self) -> Result<Self, LaneError> {
        into_result_inner(self)
    }
}

/// Free-function body of [`CommandOutput::into_result`] so the success/failure
/// branch sits at module depth (the `impl` method would otherwise nest).
///
/// # Errors
/// Returns [`LaneError::NonUtf8Output`] if stderr decoding fails for an
/// unsuccessful subprocess, or [`LaneError::NonZeroExit`] when the subprocess
/// exits unsuccessfully and stderr is valid UTF-8.
fn into_result_inner(output: CommandOutput) -> Result<CommandOutput, LaneError> {
    if output.status.success() {
        Ok(output)
    } else {
        let code = output.status.code();
        let stderr = output.stderr_str()?.to_owned();
        Err(LaneError::NonZeroExit { program: output.program, code, stderr })
    }
}
