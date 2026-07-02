use std::io;

use thiserror::Error;

/// Which captured stream failed a command-output invariant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    /// Standard output stream.
    Stdout,
    /// Standard error stream.
    Stderr,
}

/// Errors produced by [`super::CommandIn`].
#[derive(Debug, Error)]
pub enum LaneError {
    /// Command program was the empty string.
    #[error("command program must not be empty")]
    EmptyProgram,
    /// Command program contained a NUL byte.
    #[error("command program must not contain NUL bytes")]
    InvalidProgram,
    /// Spawn or wait I/O failure.
    #[error("I/O error running {program}: {source}")]
    Io {
        /// Program that failed.
        program: String,
        /// Underlying `std::io::Error`.
        #[source]
        source: io::Error,
    },
    /// Subprocess exited with a non-zero status.
    #[error("subprocess {program} exited with code {code:?}: {stderr}")]
    NonZeroExit {
        /// Program that exited non-zero.
        program: String,
        /// Raw exit code, if available.
        code: Option<i32>,
        /// Captured stderr text.
        stderr: String,
    },
    /// Captured stream was not valid UTF-8.
    #[error("subprocess {program} produced non-UTF-8 {stream:?}")]
    NonUtf8Output {
        /// Program that produced non-UTF-8 output.
        program: String,
        /// Stream that failed decoding.
        stream: OutputStream,
    },
    /// Subprocess exceeded its execution budget.
    #[error("subprocess {program} timed out after {timeout_ms} ms")]
    Timeout {
        /// Program that timed out.
        program: String,
        /// Configured timeout, in milliseconds.
        timeout_ms: u64,
    },
    /// Captured stream exceeded its byte budget.
    #[error("subprocess {program} exceeded {stream:?} output limit of {limit} bytes")]
    OutputLimitExceeded {
        /// Program that exceeded the output limit.
        program: String,
        /// Stream that exceeded the limit.
        stream: OutputStream,
        /// Byte limit that was exceeded.
        limit: usize,
    },
    /// Captured pipe was unavailable for reading.
    #[error("subprocess {program} {stream:?} pipe was unavailable")]
    PipeUnavailable {
        /// Program whose pipe was unavailable.
        program: String,
        /// Stream whose pipe was unavailable.
        stream: OutputStream,
    },
    /// Background reader thread panicked or disconnected.
    #[error("subprocess {program} {stream:?} reader thread failed")]
    ReaderThread {
        /// Program whose reader thread failed.
        program: String,
        /// Stream whose reader thread failed.
        stream: OutputStream,
    },
}
