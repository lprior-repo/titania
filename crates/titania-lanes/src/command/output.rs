use std::process::ExitStatus;

use super::{LaneError, OutputStream};

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

    #[must_use]
    pub fn success(&self) -> bool {
        self.status.success()
    }
    #[must_use]
    pub const fn status(&self) -> ExitStatus {
        self.status
    }

    #[must_use]
    pub fn stdout(&self) -> &[u8] {
        &self.stdout
    }

    #[must_use]
    pub fn stderr(&self) -> &[u8] {
        &self.stderr
    }

    #[must_use]
    pub fn into_stdout(self) -> Vec<u8> {
        self.stdout
    }

    #[must_use]
    pub fn into_stderr(self) -> Vec<u8> {
        self.stderr
    }

    pub fn stdout_str(&self) -> Result<&str, LaneError> {
        std::str::from_utf8(&self.stdout).map_err(|_utf8_err| LaneError::NonUtf8Output {
            program: self.program.clone(),
            stream: OutputStream::Stdout,
        })
    }

    pub fn stderr_str(&self) -> Result<&str, LaneError> {
        std::str::from_utf8(&self.stderr).map_err(|_utf8_err| LaneError::NonUtf8Output {
            program: self.program.clone(),
            stream: OutputStream::Stderr,
        })
    }

    pub fn into_result(self) -> Result<Self, LaneError> {
        into_result_inner(self)
    }
}

fn into_result_inner(output: CommandOutput) -> Result<CommandOutput, LaneError> {
    if output.status.success() {
        Ok(output)
    } else {
        let code = output.status.code();
        let stderr = output.stderr_str()?.to_owned();
        Err(LaneError::NonZeroExit { program: output.program, code, stderr })
    }
}
