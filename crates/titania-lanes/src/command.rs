//! `CommandIn`: single chokepoint for shelling out from a lane.
//!
//! Every subprocess a lane launches MUST go through [`CommandIn`] so
//! that the working directory, environment policy, execution budget,
//! output budget, exit-code handling, and UTF-8 decoding behavior are
//! explicit and typed.

mod output;
mod process;
mod reader;

use std::{
    borrow::Cow,
    io,
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

pub use output::CommandOutput;
use process::{configure_process_group, terminate_child_tree};
use reader::{
    ReaderHandle, drain_after_termination, duration_millis, remaining_budget, spawn_reader,
    take_pipe,
};
use smallvec::SmallVec;
use thiserror::Error;
use titania_core::TargetProject;
use wait_timeout::ChildExt;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);
const DEFAULT_MAX_STDOUT: usize = 1024 * 1024;
const DEFAULT_MAX_STDERR: usize = 1024 * 1024;
const TERMINATION_GRACE: Duration = Duration::from_secs(1);

/// Which captured stream failed a command-output invariant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    /// Standard output of the child.
    Stdout,
    /// Standard error of the child.
    Stderr,
}

/// Environment policy for a spawned command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvPolicy {
    /// Clear the process environment, then apply explicitly supplied env
    /// pairs. This is the default for deterministic target judgment.
    Clear,
    /// Inherit the parent process environment, then apply explicitly
    /// supplied env pairs.
    Inherit,
}

/// Bounded execution policy for a spawned command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandBudget {
    /// Maximum wall-clock runtime for the child.
    pub timeout: Duration,
    /// Maximum bytes the child may write to stdout.
    pub max_stdout: usize,
    /// Maximum bytes the child may write to stderr.
    pub max_stderr: usize,
}

impl Default for CommandBudget {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_TIMEOUT,
            max_stdout: DEFAULT_MAX_STDOUT,
            max_stderr: DEFAULT_MAX_STDERR,
        }
    }
}

/// Errors produced by [`CommandIn`].
#[derive(Debug, Error)]
pub enum LaneError {
    /// Program name was the empty string.
    #[error("command program must not be empty")]
    EmptyProgram,
    /// Program name contained a NUL byte.
    #[error("command program must not contain NUL bytes")]
    InvalidProgram,
    /// Underlying I/O error from spawning or talking to the child.
    #[error("I/O error running {program}: {source}")]
    Io {
        /// Program name for diagnostic context.
        program: String,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// Child exited with a non-zero code; stderr is captured for context.
    #[error("subprocess {program} exited with code {code:?}: {stderr}")]
    NonZeroExit {
        /// Program name for diagnostic context.
        program: String,
        /// Exit code if the OS provided one.
        code: Option<i32>,
        /// Captured stderr output.
        stderr: String,
    },
    /// Subprocess emitted bytes that are not valid UTF-8.
    #[error("subprocess {program} produced non-UTF-8 {stream:?}")]
    NonUtf8Output {
        /// Program name for diagnostic context.
        program: String,
        /// Which stream failed the UTF-8 check.
        stream: OutputStream,
    },
    /// Subprocess exceeded the configured wall-clock timeout.
    #[error("subprocess {program} timed out after {timeout_ms} ms")]
    Timeout {
        /// Program name for diagnostic context.
        program: String,
        /// Timeout in milliseconds.
        timeout_ms: u64,
    },
    /// Subprocess wrote more bytes than the configured per-stream cap.
    #[error("subprocess {program} exceeded {stream:?} output limit of {limit} bytes")]
    OutputLimitExceeded {
        /// Program name for diagnostic context.
        program: String,
        /// Which stream overflowed.
        stream: OutputStream,
        /// Configured cap in bytes.
        limit: usize,
    },
    /// The child did not provide a pipe for the requested stream.
    #[error("subprocess {program} {stream:?} pipe was unavailable")]
    PipeUnavailable {
        /// Program name for diagnostic context.
        program: String,
        /// Which stream was unavailable.
        stream: OutputStream,
    },
    /// The reader thread for the requested stream returned an error.
    #[error("subprocess {program} {stream:?} reader thread failed")]
    ReaderThread {
        /// Program name for diagnostic context.
        program: String,
        /// Which stream's reader thread failed.
        stream: OutputStream,
    },
}

/// A typed builder for `std::process::Command` rooted at a target project.
type EnvPair<'a> = (Cow<'a, str>, Cow<'a, str>);

/// Builder state for a `std::process::Command` rooted at a target project.
#[derive(Debug)]
pub struct CommandIn<'a> {
    /// Working directory the command is rooted in.
    cwd: &'a TargetProject,
    /// Program name (the first non-flag argument).
    program: Cow<'a, str>,
    /// Positional arguments to pass to the program.
    args: SmallVec<[Cow<'a, str>; 8]>,
    /// Explicit environment pairs to apply.
    env: SmallVec<[EnvPair<'a>; 4]>,
    /// Environment variables to remove.
    env_remove: SmallVec<[Cow<'a, str>; 4]>,
    /// Whether to inherit the parent process environment.
    env_policy: EnvPolicy,
    /// Bounded execution policy.
    budget: CommandBudget,
}

impl<'a> CommandIn<'a> {
    /// Create a new `CommandIn` for `program` inside the target project.
    ///
    /// # Errors
    /// [`LaneError::EmptyProgram`] if `program` is empty;
    /// [`LaneError::InvalidProgram`] if it contains NUL bytes.
    pub fn new(cwd: &'a TargetProject, program: &'a str) -> Result<Self, LaneError> {
        validate_program(program)?;
        Ok(Self {
            cwd,
            program: Cow::Borrowed(program),
            args: SmallVec::new(),
            env: SmallVec::new(),
            env_remove: SmallVec::new(),
            env_policy: EnvPolicy::Clear,
            budget: CommandBudget::default(),
        })
    }
    /// Append a single argument. Returns `&mut self` for chaining.
    pub fn arg(&mut self, a: &'a str) -> &mut Self {
        self.args.push(Cow::Borrowed(a));
        self
    }

    /// Append multiple arguments. Returns `&mut self` for chaining.
    pub fn args(&mut self, as_: &'a [&'a str]) -> &mut Self {
        self.args.extend(as_.iter().map(|s| Cow::Borrowed(*s)));
        self
    }

    /// Set an environment variable in the spawned process.
    pub fn env(&mut self, k: &'a str, v: &'a str) -> &mut Self {
        self.env.push((Cow::Borrowed(k), Cow::Borrowed(v)));
        self
    }

    /// Remove an inherited environment variable from the spawned process.
    pub fn env_remove(&mut self, k: &'a str) -> &mut Self {
        self.env_remove.push(Cow::Borrowed(k));
        self
    }

    /// Explicitly inherit the parent environment.
    pub const fn inherit_env(&mut self) -> &mut Self {
        self.env_policy = EnvPolicy::Inherit;
        self
    }

    /// Replace the default execution/output budget.
    pub const fn budget(&mut self, budget: CommandBudget) -> &mut Self {
        self.budget = budget;
        self
    }

    /// Run the subprocess, capture stdout/stderr, enforce the execution
    /// budget, and reject non-zero exits.
    ///
    /// # Errors
    /// Returns [`LaneError`] when process spawn, capture, timeout, UTF-8
    /// decoding, or non-zero exit handling fails.
    pub fn run(&self) -> Result<CommandOutput, LaneError> {
        self.run_capture_raw()?.into_result()
    }

    /// Alias for [`CommandIn::run`]: checked captured execution.
    ///
    /// # Errors
    /// Returns the same [`LaneError`] variants as [`CommandIn::run`].
    pub fn run_capture(&self) -> Result<CommandOutput, LaneError> {
        self.run()
    }

    /// Run the subprocess, capture stdout/stderr, and enforce execution
    /// and output budgets without checking the exit status.
    ///
    /// # Errors
    /// Returns [`LaneError`] when process spawn, pipe capture, reader budget,
    /// timeout, or UTF-8 metadata handling fails.
    pub fn run_capture_raw(&self) -> Result<CommandOutput, LaneError> {
        let started = Instant::now();
        let mut cmd = self.base_command();
        let _ = cmd.stdin(Stdio::null());
        let _ = cmd.stdout(Stdio::piped());
        let _ = cmd.stderr(Stdio::piped());
        let mut child = cmd.spawn().map_err(|source| self.io_error(source))?;
        let stdout = take_pipe(&mut child.stdout, self.program_name(), OutputStream::Stdout)?;
        let stderr = take_pipe(&mut child.stderr, self.program_name(), OutputStream::Stderr)?;
        let stdout_reader =
            spawn_reader(stdout, self.budget.max_stdout, self.program_name(), OutputStream::Stdout);
        let stderr_reader =
            spawn_reader(stderr, self.budget.max_stderr, self.program_name(), OutputStream::Stderr);
        let Some(status) =
            child.wait_timeout(self.budget.timeout).map_err(|source| self.io_error(source))?
        else {
            return self.timeout_child(child, stdout_reader, stderr_reader);
        };
        let stdout = self.receive_reader(stdout_reader, started)?;
        let stderr = self.receive_reader(stderr_reader, started)?;
        Ok(CommandOutput::new(status, stdout, stderr, self.program_name()))
    }

    /// Run the subprocess with inherited stdout/stderr and enforce the
    /// execution budget without checking the exit status.
    ///
    /// # Errors
    /// Returns [`LaneError`] when process spawn, timeout handling, or child
    /// termination fails.
    pub fn run_status_raw(&self) -> Result<ExitStatus, LaneError> {
        let mut cmd = self.base_command();
        let _ = cmd.stdin(Stdio::null());
        let _ = cmd.stdout(Stdio::inherit());
        let _ = cmd.stderr(Stdio::inherit());
        let mut child = cmd.spawn().map_err(|source| self.io_error(source))?;
        let Some(status) =
            child.wait_timeout(self.budget.timeout).map_err(|source| self.io_error(source))?
        else {
            terminate_child_tree(&mut child, self.program_name())?;
            let _ = child.wait().map_err(|source| self.io_error(source))?;
            return Err(self.timeout_error());
        };
        Ok(status)
    }

    fn base_command(&self) -> Command {
        let mut cmd = Command::new(self.program.as_ref());
        configure_process_group(&mut cmd);
        let _ = cmd.current_dir(self.cwd.as_std_path());
        if self.env_policy == EnvPolicy::Clear {
            let _ = cmd.env_clear();
        }
        let _ = cmd.args(self.args.iter().map(std::convert::AsRef::as_ref));
        let _ = cmd.envs(self.env.iter().map(|(k, v)| (k.as_ref(), v.as_ref())));
        for key in &self.env_remove {
            let _ = cmd.env_remove(key.as_ref());
        }
        cmd
    }

    /// Terminate the child on timeout, drain readers, and report the timeout.
    ///
    /// # Errors
    /// Returns [`LaneError::Io`] on kill failure, [`LaneError::Timeout`] if
    /// the readers don't drain, or the readers' own error otherwise.
    fn timeout_child(
        &self,
        mut child: Child,
        stdout_reader: ReaderHandle,
        stderr_reader: ReaderHandle,
    ) -> Result<CommandOutput, LaneError> {
        terminate_child_tree(&mut child, self.program_name())?;
        let _ = child.wait().map_err(|source| self.io_error(source))?;
        drain_after_termination(stdout_reader, self.timeout_error())?;
        drain_after_termination(stderr_reader, self.timeout_error())?;
        Err(self.timeout_error())
    }

    /// Drain a single reader, mapping a timeout into the lane's timeout error.
    ///
    /// # Errors
    /// Returns the lane's timeout error if the reader times out, or whatever
    /// the reader produced otherwise.
    fn receive_reader(&self, reader: ReaderHandle, started: Instant) -> Result<Vec<u8>, LaneError> {
        match reader.recv_timeout(remaining_budget(started, self.budget.timeout)) {
            Ok(out) => Ok(out),
            Err(LaneError::Timeout { .. }) => Err(self.timeout_error()),
            Err(e) => Err(e),
        }
    }

    /// Build the [`LaneError::Timeout`] for this run.
    fn timeout_error(&self) -> LaneError {
        LaneError::Timeout {
            program: self.program_name(),
            timeout_ms: duration_millis(self.budget.timeout),
        }
    }

    /// Wrap an [`io::Error`] with this lane's program name.
    fn io_error(&self, source: io::Error) -> LaneError {
        LaneError::Io { program: self.program_name(), source }
    }

    /// Borrow the program name as an owned `String` for error context.
    fn program_name(&self) -> String {
        self.program.to_string()
    }
}

/// Validate that the program name is non-empty and contains no NUL byte.
///
/// # Errors
/// Returns [`LaneError::EmptyProgram`] when `program` is empty and
/// [`LaneError::InvalidProgram`] when `program` contains a NUL byte.
fn validate_program(program: &str) -> Result<(), LaneError> {

    if program.is_empty() {
        Err(LaneError::EmptyProgram)
    } else if program.contains('\0') {
        Err(LaneError::InvalidProgram)
    } else {
        Ok(())
    }
}
