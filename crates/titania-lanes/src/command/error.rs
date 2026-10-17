use std::io;

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Error)]
pub enum LaneError {
    #[error("command program must not be empty")]
    EmptyProgram,
    #[error("command program must not contain NUL bytes")]
    InvalidProgram,
    #[error("I/O error running {program}: {source}")]
    Io {
        program: String,
        #[source]
        source: io::Error,
    },
    #[error("subprocess {program} exited with code {code:?}: {stderr}")]
    NonZeroExit { program: String, code: Option<i32>, stderr: String },
    #[error("subprocess {program} produced non-UTF-8 {stream:?}")]
    NonUtf8Output { program: String, stream: OutputStream },
    #[error("subprocess {program} timed out after {timeout_ms} ms")]
    Timeout { program: String, timeout_ms: u64 },
    #[error("subprocess {program} exceeded {stream:?} output limit of {limit} bytes")]
    OutputLimitExceeded { program: String, stream: OutputStream, limit: usize },
    #[error("subprocess {program} {stream:?} pipe was unavailable")]
    PipeUnavailable { program: String, stream: OutputStream },
    #[error("subprocess {program} {stream:?} reader thread failed")]
    ReaderThread { program: String, stream: OutputStream },
}
