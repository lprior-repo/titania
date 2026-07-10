use std::{
    borrow::Cow,
    io,
    process::{Child, Command, ExitStatus, Stdio},
    time::Instant,
};

use wait_timeout::ChildExt;

use super::{CommandIn, CommandOutput, EnvPolicy, LaneError, OutputStream};
use crate::command::{
    env_filter::ScrubbedEnv,
    process::{configure_process_group, terminate_child_tree},
    reader::{
        ReaderHandle, drain_after_termination, duration_millis, remaining_budget, spawn_reader,
        take_pipe,
    },
};

impl CommandIn<'_> {
    /// Run the subprocess, capture stdout/stderr, enforce the execution
    /// budget, and reject non-zero exits.
    ///
    /// # Errors
    /// [`LaneError::NonZeroExit`] on a non-zero exit, plus any
    /// [`LaneError`] from [`CommandIn::run_capture_raw`].
    pub fn run(&self) -> Result<CommandOutput, LaneError> {
        self.run_capture_raw()?.into_result()
    }

    /// Alias for [`CommandIn::run`]: checked captured execution.
    ///
    /// # Errors
    /// See [`CommandIn::run`].
    pub fn run_capture(&self) -> Result<CommandOutput, LaneError> {
        self.run()
    }

    /// Run the subprocess, capture stdout/stderr, and enforce execution
    /// and output budgets without checking the exit status.
    ///
    /// # Errors
    /// [`LaneError::Io`] on spawn/wait failure, [`LaneError::Timeout`]
    /// on budget exhaustion, [`LaneError::OutputLimitExceeded`] or
    /// [`LaneError::PipeUnavailable`] on capture failure.
    pub fn run_capture_raw(&self) -> Result<CommandOutput, LaneError> {
        let started = Instant::now();
        let mut cmd = self.base_command();
        let _ = cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = cmd.spawn().map_err(|source| self.io_error(source))?;
        let stdout = take_pipe(&mut child.stdout, self.program_name(), OutputStream::Stdout)?;
        let stderr = take_pipe(&mut child.stderr, self.program_name(), OutputStream::Stderr)?;
        let stdout_reader =
            spawn_reader(stdout, self.budget.max_stdout, self.program_name(), OutputStream::Stdout);
        let stderr_reader =
            spawn_reader(stderr, self.budget.max_stderr, self.program_name(), OutputStream::Stderr);
        capture_to_completion(&mut child, stdout_reader, stderr_reader, started, self)
    }

    /// Run the subprocess with inherited stdout/stderr and enforce the
    /// execution budget without checking the exit status.
    ///
    /// # Errors
    /// [`LaneError::Io`] on spawn/wait failure or [`LaneError::Timeout`]
    /// on budget exhaustion.
    pub fn run_status_raw(&self) -> Result<ExitStatus, LaneError> {
        let mut cmd = self.base_command();
        let _ = cmd.stdin(Stdio::null()).stdout(Stdio::inherit()).stderr(Stdio::inherit());
        let mut child = cmd.spawn().map_err(|source| self.io_error(source))?;
        wait_status_or_timeout(&mut child, self)
    }

    fn base_command(&self) -> Command {
        build_base_command(self)
    }

    /// Terminate a timed-out child and drain both capture readers.
    ///
    /// # Errors
    /// Returns [`LaneError::Io`] if terminating or waiting on the child fails,
    /// propagates reader drain errors, and otherwise returns
    /// [`LaneError::Timeout`] for the owner command.
    fn timeout_child(
        &self,
        child: &mut Child,
        stdout_reader: ReaderHandle,
        stderr_reader: ReaderHandle,
    ) -> Result<CommandOutput, LaneError> {
        terminate_child_tree(child, self.program_name())?;
        let _ = child.wait().map_err(|source| self.io_error(source))?;
        drain_after_termination(stdout_reader, self.timeout_error())?;
        drain_after_termination(stderr_reader, self.timeout_error())?;
        Err(self.timeout_error())
    }

    /// Receive one capture reader result within the remaining command budget.
    ///
    /// # Errors
    /// Returns any [`LaneError`] produced by the reader, with reader-local
    /// timeout errors rewritten to this command's full timeout error.
    fn receive_reader(&self, reader: ReaderHandle, started: Instant) -> Result<Vec<u8>, LaneError> {
        let result = reader.recv_timeout(remaining_budget(started, self.budget.timeout));
        map_reader_result(result, self)
    }

    fn timeout_error(&self) -> LaneError {
        LaneError::Timeout {
            program: self.program_name(),
            timeout_ms: duration_millis(self.budget.timeout),
        }
    }

    fn io_error(&self, source: io::Error) -> LaneError {
        LaneError::Io { program: self.program_name(), source }
    }

    fn program_name(&self) -> String {
        self.program.to_string()
    }
}

/// Wait for `child` to exit within the owner's timeout, mapping a timeout to
/// the owner's [`LaneError::Timeout`] after terminating the process tree.
///
/// # Errors
/// Returns [`LaneError::Io`] if waiting for or terminating the child fails, or
/// [`LaneError::Timeout`] when the child exceeds the owner's execution budget.
fn wait_status_or_timeout(
    child: &mut Child,
    owner: &CommandIn<'_>,
) -> Result<ExitStatus, LaneError> {
    if let Some(status) =
        child.wait_timeout(owner.budget.timeout).map_err(|source| owner.io_error(source))?
    {
        Ok(status)
    } else {
        terminate_child_tree(child, owner.program_name())?;
        let _ = child.wait().map_err(|source| owner.io_error(source))?;
        Err(owner.timeout_error())
    }
}

/// Build the base `Command` (program, cwd, env policy, args, env, `env_remove`)
/// rooted at `owner`. Free-function body of [`CommandIn::base_command`] so the
/// env-remove loop sits at module depth.
fn build_base_command(owner: &CommandIn<'_>) -> Command {
    let mut cmd = Command::new(owner.program.as_ref());
    configure_process_group(&mut cmd);
    let _ = cmd.current_dir(owner.cwd.as_std_path());
    let _ = cmd.env_clear();
    if owner.env_policy == EnvPolicy::Inherit {
        ScrubbedEnv::from_parent_for_target(std::env::vars_os(), owner.cwd.as_std_path())
            .apply_to(&mut cmd);
    }
    let _ = cmd.args(owner.args.iter().map(<Cow<'_, str> as AsRef<str>>::as_ref));
    let _ = cmd.envs(owner.env.iter().map(|(key, value)| (key.as_ref(), value.as_ref())));
    owner.env_remove.iter().for_each(|key| {
        let _ = cmd.env_remove(key.as_ref());
    });
    cmd
}

/// Drive a captured subprocess to completion, handling the timeout branch by
/// terminating the tree and draining the reader threads.
///
/// # Errors
/// Returns [`LaneError::Io`] for wait failures, [`LaneError::Timeout`] when the
/// command exceeds its execution budget, or any [`LaneError`] produced by the
/// stdout/stderr reader threads.
fn capture_to_completion(
    child: &mut Child,
    stdout_reader: ReaderHandle,
    stderr_reader: ReaderHandle,
    started: Instant,
    owner: &CommandIn<'_>,
) -> Result<CommandOutput, LaneError> {
    let Some(status) =
        child.wait_timeout(owner.budget.timeout).map_err(|source| owner.io_error(source))?
    else {
        return owner.timeout_child(child, stdout_reader, stderr_reader);
    };
    let stdout = owner.receive_reader(stdout_reader, started)?;
    let stderr = owner.receive_reader(stderr_reader, started)?;
    Ok(CommandOutput::new(status, stdout, stderr, owner.program_name()))
}

/// Map a reader result, rewriting a [`LaneError::Timeout`] to the owner's.
///
/// # Errors
/// Returns the original [`LaneError`] from `result`, except that reader-local
/// [`LaneError::Timeout`] values are replaced with the owner's timeout error.
fn map_reader_result(
    result: Result<Vec<u8>, LaneError>,
    owner: &CommandIn<'_>,
) -> Result<Vec<u8>, LaneError> {
    match result {
        Ok(out) => Ok(out),
        Err(LaneError::Timeout { .. }) => Err(owner.timeout_error()),
        Err(other) => Err(other),
    }
}
