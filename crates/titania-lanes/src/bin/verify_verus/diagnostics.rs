use std::io::{self, Write};

use titania_lanes::LaneExit;

/// Emit a diagnostic and return the requested lane error.
///
/// # Errors
///
/// Returns `LaneExit::Failure` if stderr writing fails; otherwise returns the
/// supplied lane exit.
pub(crate) fn err_after_stderr<T>(
    args: std::fmt::Arguments<'_>,
    code: LaneExit,
) -> Result<T, LaneExit> {
    write_stderr_line(args).map_err(|_error| LaneExit::Failure)?;
    Err(code)
}

pub(crate) fn lane_after_stderr(args: std::fmt::Arguments<'_>, code: LaneExit) -> LaneExit {
    match write_stderr_line(args) {
        Ok(()) => code,
        Err(_) => LaneExit::Failure,
    }
}

pub(crate) fn rule_error_exit(message: &str) -> LaneExit {
    match write_stderr_line(format_args!("[verify-verus] rule id configuration error: {message}")) {
        Ok(()) => LaneExit::Failure,
        Err(error) => stderr_write_failure(error),
    }
}

pub(crate) fn stderr_write_failure(error: io::Error) -> LaneExit {
    let _inner = error.into_inner();
    LaneExit::Failure
}

/// Write one formatted line to stderr.
///
/// # Errors
///
/// Returns the underlying stderr write error.
pub(crate) fn write_stderr_line(args: std::fmt::Arguments<'_>) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    stderr.write_fmt(args)?;
    stderr.write_all(b"\n")
}
