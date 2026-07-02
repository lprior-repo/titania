use std::io::{self, Write};

use titania_lanes::{LaneExit, exit};

/// Write the scan banner used by the legacy shell lane.
///
/// # Errors
///
/// Returns the stderr write error if the banner cannot be emitted.
pub fn write_scan_header() -> io::Result<()> {
    write_stderr(&format!(
        "CWD: {}\n\
         Command: bash scripts/check-panic-surface.sh\n\
         ScanDomain: crates/*/src\n\
         NonProductionPathExcluded: tests benches examples fuzz target .beads fixtures \
         build.rs path-scoped tests.rs *_tests.rs kani harnesses loom models\n",
        cwd_string()
    ))
}

/// Write raw text to stderr.
///
/// # Errors
///
/// Returns the underlying stderr write error.
pub fn write_stderr(text: &str) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    stderr.write_all(text.as_bytes())
}

/// Write one line to stderr.
///
/// # Errors
///
/// Returns the underlying stderr write error.
fn write_stderr_line(text: &str) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    stderr.write_all(text.as_bytes())?;
    stderr.write_all(b"\n")
}

/// Write one diagnostic line and convert the requested lane exit into a process exit code.
#[must_use]
pub fn exit_after_stderr_line(text: &str, code: LaneExit) -> std::process::ExitCode {
    match write_stderr_line(text) {
        Ok(()) => exit(code),
        Err(_) => exit(LaneExit::Failure),
    }
}

fn cwd_string() -> String {
    std::env::current_dir().map_or_else(|_| String::from("?"), |p| p.display().to_string())
}
