use std::{
    io::Read,
    sync::mpsc::{self, Receiver, RecvTimeoutError},
    thread,
    time::{Duration, Instant},
};

use super::{LaneError, OutputStream, TERMINATION_GRACE};

/// Result delivered by a reader thread: captured bytes or a typed lane error.
type ReaderResult = Result<Vec<u8>, LaneError>;

pub(super) struct ReaderHandle {
    thread: thread::JoinHandle<()>,
    rx: Receiver<ReaderResult>,
    program: String,
    stream: OutputStream,
}

impl ReaderHandle {
    /// Receive the reader result within `timeout` and join the reader thread.
    ///
    /// # Errors
    /// Returns [`LaneError::Timeout`] if the reader does not respond before
    /// `timeout`, [`LaneError::ReaderThread`] if the reader disconnects or
    /// panics, or the [`LaneError`] produced while reading the pipe.
    pub(super) fn recv_timeout(self, timeout: Duration) -> ReaderResult {
        recv_with_timeout(self.thread, &self.rx, self.program, self.stream, timeout)
    }
}

/// Free-function body of [`ReaderHandle::recv_timeout`] so the receive
/// dispatch sits at module depth rather than inside the `impl`.
///
/// # Errors
/// Returns [`LaneError::Timeout`] if no reader result arrives within `timeout`,
/// [`LaneError::ReaderThread`] if the reader disconnects or panics, or the
/// [`LaneError`] sent by the reader thread.
fn recv_with_timeout(
    thread: thread::JoinHandle<()>,
    rx: &Receiver<ReaderResult>,
    program: String,
    stream: OutputStream,
    timeout: Duration,
) -> ReaderResult {
    match rx.recv_timeout(timeout) {
        Ok(result) => {
            join_finished_reader(thread, program, stream)?;
            result
        }
        Err(RecvTimeoutError::Timeout) => {
            Err(LaneError::Timeout { program, timeout_ms: duration_millis(timeout) })
        }
        Err(RecvTimeoutError::Disconnected) => {
            join_finished_reader(thread, program.clone(), stream)?;
            Err(LaneError::ReaderThread { program, stream })
        }
    }
}

/// Take an owned pipe handle from a child-process stream slot.
///
/// # Errors
/// Returns [`LaneError::PipeUnavailable`] when the child process did not expose
/// the requested stream pipe.
pub(super) fn take_pipe<T>(
    pipe: &mut Option<T>,
    program: String,
    stream: OutputStream,
) -> Result<T, LaneError> {
    pipe.take().ok_or(LaneError::PipeUnavailable { program, stream })
}

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
        let _receiver_alive = tx.send(result).is_ok();
    });
    ReaderHandle { thread, rx, program, stream }
}

/// Drain a reader after terminating its subprocess.
///
/// # Errors
/// Returns `timeout_error` if the reader still does not finish within the
/// termination grace period, or propagates any non-timeout [`LaneError`] from
/// the reader.
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

pub(super) fn remaining_budget(started: Instant, budget: Duration) -> Duration {
    budget.checked_sub(started.elapsed()).map_or(Duration::ZERO, |remaining| remaining)
}

pub(super) fn duration_millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).map_or(u64::MAX, |millis| millis)
}

/// Read a pipe into memory while enforcing `limit` bytes.
///
/// # Errors
/// Returns [`LaneError::OutputLimitExceeded`] if the capture budget cannot be
/// represented or the pipe produces more than `limit` bytes. Returns
/// [`LaneError::Io`] if reading from the pipe fails.
fn read_limited<R: Read>(
    pipe: R,
    limit: usize,
    program: String,
    stream: OutputStream,
) -> Result<Vec<u8>, LaneError> {
    let read_limit = u64::try_from(limit.saturating_add(1)).map_err(|_size_err| {
        LaneError::OutputLimitExceeded { program: program.clone(), stream, limit }
    })?;
    let mut limited = pipe.take(read_limit);
    let mut out = Vec::new();
    let _ = limited
        .read_to_end(&mut out)
        .map_err(|source| LaneError::Io { program: program.clone(), source })?;
    if out.len() > limit {
        Err(LaneError::OutputLimitExceeded { program, stream, limit })
    } else {
        Ok(out)
    }
}

/// Join a reader thread that has already sent or disconnected.
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
