use std::{
    io::Read,
    sync::mpsc::{self, RecvTimeoutError},
    thread,
    time::{Duration, Instant},
};

use super::{LaneError, OutputStream, TERMINATION_GRACE};

/// Background reader handle paired with a typed channel.
///
/// The reader runs on a dedicated thread and pushes a `Result<Vec<u8>, LaneError>`
/// back to the main lane when it finishes (or when it hits the byte cap).
pub(super) struct ReaderHandle {
    /// Join handle for the reader thread.
    thread: thread::JoinHandle<()>,
    /// Typed channel from the reader thread.
    rx: ReaderChannel,
    /// Program name for diagnostic context.
    program: String,
    /// Which stream this reader is draining.
    stream: OutputStream,
}

/// Result type produced by the reader thread.
type ReadOutcome = Result<Vec<u8>, LaneError>;

/// Typed channel from the reader thread.
type ReaderChannel = mpsc::Receiver<ReadOutcome>;
impl ReaderHandle {
    /// Wait up to `timeout` for the reader to finish, then join the thread.
    ///
    /// # Errors
    /// Returns [`LaneError::Timeout`] on channel timeout, [`LaneError::ReaderThread`]
    /// if the reader thread panicked, and whatever the reader produced otherwise.
    pub(super) fn recv_timeout(self, timeout: Duration) -> Result<Vec<u8>, LaneError> {
        match self.rx.recv_timeout(timeout) {
            Ok(result) => {
                join_finished_reader(self.thread, self.program, self.stream)?;
                result
            }
            Err(RecvTimeoutError::Timeout) => Err(LaneError::Timeout {
                program: self.program,
                timeout_ms: duration_millis(timeout),
            }),
            Err(RecvTimeoutError::Disconnected) => {
                join_finished_reader(self.thread, self.program.clone(), self.stream)?;
                Err(LaneError::ReaderThread { program: self.program, stream: self.stream })
            }
        }
    }
}

/// Take the pipe out of an `Option`, returning [`LaneError::PipeUnavailable`] if
/// the OS did not provide one.
///
/// # Errors
/// Returns [`LaneError::PipeUnavailable`] when the pipe is `None`.
pub(super) fn take_pipe<T>(
    pipe: &mut Option<T>,
    program: String,
    stream: OutputStream,
) -> Result<T, LaneError> {
    pipe.take().ok_or(LaneError::PipeUnavailable { program, stream })
}

/// Spawn a thread that reads up to `limit` bytes from `pipe` and reports the
/// result through a channel.
pub(super) fn spawn_reader<R>(
    pipe: R,
    limit: usize,
    program: String,
    stream: OutputStream,
) -> ReaderHandle
where
    R: Read + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    let reader_program = program.clone();
    let thread = thread::spawn(move || {
        let result = read_limited(pipe, limit, reader_program, stream);
        drop(tx.send(result));
    });
    ReaderHandle { thread, rx, program, stream }
}

/// Drain any pending reader output after the child has been terminated.
///
/// # Errors
/// Returns the supplied `timeout_error` when the reader has not finished in
/// time, or the reader's own error otherwise.
pub(super) fn drain_after_termination(
    reader: ReaderHandle,
    timeout_error: LaneError,
) -> Result<(), LaneError> {
    match reader.recv_timeout(TERMINATION_GRACE) {
        Ok(_) => Ok(()),
        Err(LaneError::Timeout { .. }) => Err(timeout_error),
        Err(e) => Err(e),
    }
}

/// Remaining wall-clock budget for the subprocess, clamped to zero.
#[must_use]
pub(super) fn remaining_budget(started: Instant, budget: Duration) -> Duration {
    budget.checked_sub(started.elapsed()).map_or(Duration::ZERO, |remaining| remaining)
}

/// Convert a [`Duration`] to milliseconds, saturating to `u64::MAX` on overflow.
#[must_use]
pub(super) fn duration_millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).map_or(u64::MAX, |ms| ms)
}

/// Read up to `limit + 1` bytes from `pipe`; reject if the read overflows
/// the cap.
///
/// # Errors
/// Returns [`LaneError::OutputLimitExceeded`] when the read overflows the
/// cap, or [`LaneError::Io`] on any underlying I/O failure.
fn read_limited<R: Read>(
    pipe: R,
    limit: usize,
    program: String,
    stream: OutputStream,
) -> ReadOutcome {
    let read_limit = u64::try_from(limit.saturating_add(1))
        .map_err(|_e| LaneError::OutputLimitExceeded { program: program.clone(), stream, limit })?;
    let mut limited = pipe.take(read_limit);
    let mut out = Vec::new();
    let _bytes_read = limited
        .read_to_end(&mut out)
        .map_err(|source| LaneError::Io { program: program.clone(), source })?;
    if out.len() > limit {
        Err(LaneError::OutputLimitExceeded { program, stream, limit })
    } else {
        Ok(out)
    }
}

/// Join the reader thread, translating any panic into [`LaneError::ReaderThread`].
///
/// # Errors
/// Returns [`LaneError::ReaderThread`] if the reader thread panicked.
fn join_finished_reader(
    thread: thread::JoinHandle<()>,
    program: String,
    stream: OutputStream,
) -> Result<(), LaneError> {
    match thread.join() {
        Ok(()) => Ok(()),
        Err(_panic) => Err(LaneError::ReaderThread { program, stream }),
    }
}