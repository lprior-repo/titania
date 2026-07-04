//! Native v1 `policy-scan` lane for strict-ai TOML/env bypass checks.
//!
//! The lane discovers the target Rust project from the process CWD, loads
//! strict-ai exceptions, scans every package manifest plus Cargo/environment
//! bypass inputs, and emits a stable `LaneReport` to stderr.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use std::{
    ffi::OsStr,
    fmt, io,
    io::Write as _,
    path::{Path, PathBuf, StripPrefixError},
    process::{Command, ExitCode},
    string::FromUtf8Error,
};

use titania_lanes::{
    LaneExit, LaneReport, RuleIdError, current_target_project, exit,
    policy_scan::{exceptions::load_exceptions, scan_policy_inputs_with_exceptions},
};

const DATE_TOOL: &str = "date";
const DATE_FORMAT: &str = "+%F";
const MANIFEST_NAME: &str = "Cargo.toml";
const SKIP_DIRS: &[&str] = &[".beads", ".git", ".moon", ".worktrees", "target"];

fn main() -> ExitCode {
    exit(run())
}

fn run() -> LaneExit {
    match run_checked() {
        Ok(report) => emit_report(&report),
        Err(error) => exit_after_stderr_line(&format!("[policy-scan] {error}"), LaneExit::Failure),
    }
}

/// Execute the policy scan and return the typed lane report.
///
/// # Errors
/// Returns [`PolicyScanError`] when target discovery, policy date capture,
/// manifest collection, exception diagnostics, or scanner rule construction fails.
fn run_checked() -> Result<LaneReport, PolicyScanError> {
    let target =
        current_target_project().map_err(|error| PolicyScanError::Target(error.to_string()))?;
    let root = target.as_std_path();
    let today = policy_date()?;
    let manifests = collect_manifest_paths(root)?;
    let mut report = LaneReport::new();
    let exceptions = load_exceptions(root, &today, &mut report)?;
    scan_policy_inputs_with_exceptions(
        root,
        manifests.iter().map(PathBuf::as_path),
        &exceptions,
        &mut report,
    )?;
    Ok(report)
}

fn emit_report(report: &LaneReport) -> LaneExit {
    write_stderr(&report.render()).map_or(LaneExit::Failure, |()| report_status(report))
}

fn report_status(report: &LaneReport) -> LaneExit {
    if report.is_clean() {
        exit_after_stderr_line("NoViolationFound", LaneExit::Clean)
    } else {
        exit_after_stderr_line(
            "ViolationFound: policy-scan findings are non-empty",
            LaneExit::Violations,
        )
    }
}

/// Capture the current UTC policy date as `YYYY-MM-DD`.
///
/// # Errors
/// Returns [`PolicyScanError`] if the platform date tool cannot run, exits
/// unsuccessfully, or emits non-UTF-8 output.
fn policy_date() -> Result<String, PolicyScanError> {
    let output =
        Command::new(DATE_TOOL).arg(DATE_FORMAT).output().map_err(PolicyScanError::DateCommand)?;
    if !output.status.success() {
        return Err(PolicyScanError::DateStatus(output.status.to_string()));
    }
    String::from_utf8(output.stdout)
        .map_err(PolicyScanError::DateUtf8)
        .map(|stdout| stdout.trim().to_owned())
}

/// Collect every Cargo manifest under `root`, skipping generated/tool state.
///
/// # Errors
/// Returns [`PolicyScanError`] when a directory entry cannot be read.
fn collect_manifest_paths(root: &Path) -> Result<Vec<PathBuf>, PolicyScanError> {
    let mut manifests = Vec::new();
    collect_manifest_paths_into(root, root, &mut manifests)?;
    manifests.sort();
    Ok(manifests)
}

/// Recursively collect Cargo manifests under one directory.
///
/// # Errors
/// Returns [`PolicyScanError`] when this directory or one of its entries cannot be read.
fn collect_manifest_paths_into(
    root: &Path,
    dir: &Path,
    manifests: &mut Vec<PathBuf>,
) -> Result<(), PolicyScanError> {
    std::fs::read_dir(dir)
        .map_err(|source| PolicyScanError::ManifestWalk { path: dir.to_path_buf(), source })?
        .try_for_each(|entry| visit_dir_entry(root, entry, manifests))
}

/// Visit one directory entry while collecting Cargo manifests.
///
/// # Errors
/// Returns [`PolicyScanError`] when the entry metadata cannot be read or a
/// nested directory scan fails.
fn visit_dir_entry(
    root: &Path,
    entry: io::Result<std::fs::DirEntry>,
    manifests: &mut Vec<PathBuf>,
) -> Result<(), PolicyScanError> {
    let entry = entry
        .map_err(|source| PolicyScanError::ManifestWalk { path: root.to_path_buf(), source })?;
    let path = entry.path();
    let file_type = entry
        .file_type()
        .map_err(|source| PolicyScanError::ManifestWalk { path: path.clone(), source })?;
    if file_type.is_dir() && !is_skipped_dir(&path) {
        return collect_manifest_paths_into(root, &path, manifests);
    }
    if file_type.is_file() && entry.file_name() == OsStr::new(MANIFEST_NAME) {
        push_manifest_path(root, &path, manifests)?;
    }
    Ok(())
}

fn is_skipped_dir(path: &Path) -> bool {
    path.file_name().and_then(OsStr::to_str).is_some_and(|name| SKIP_DIRS.contains(&name))
}

/// Push one root-relative manifest path.
///
/// # Errors
/// Returns [`PolicyScanError`] if a collected manifest is unexpectedly outside
/// the target root.
fn push_manifest_path(
    root: &Path,
    path: &Path,
    manifests: &mut Vec<PathBuf>,
) -> Result<(), PolicyScanError> {
    let relative = path.strip_prefix(root).map_err(|source| {
        PolicyScanError::ManifestOutsideRoot { path: path.to_path_buf(), source }
    })?;
    manifests.push(relative.to_path_buf());
    Ok(())
}

/// Write raw text to stderr.
///
/// # Errors
/// Returns the underlying stderr write error.
fn write_stderr(text: &str) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    stderr.write_all(text.as_bytes())
}

/// Write one line to stderr.
///
/// # Errors
/// Returns the underlying stderr write error.
fn write_stderr_line(text: &str) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    stderr.write_all(text.as_bytes())?;
    stderr.write_all(b"\n")
}

fn exit_after_stderr_line(text: &str, code: LaneExit) -> LaneExit {
    match write_stderr_line(text) {
        Ok(()) => code,
        Err(_) => LaneExit::Failure,
    }
}

#[derive(Debug)]
enum PolicyScanError {
    Target(String),
    DateCommand(io::Error),
    DateStatus(String),
    DateUtf8(FromUtf8Error),
    ManifestWalk { path: PathBuf, source: io::Error },
    ManifestOutsideRoot { path: PathBuf, source: StripPrefixError },
    RuleId(RuleIdError),
}

impl From<RuleIdError> for PolicyScanError {
    fn from(error: RuleIdError) -> Self {
        Self::RuleId(error)
    }
}

impl fmt::Display for PolicyScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Target(error) => write!(f, "target discovery failed: {error}"),
            Self::DateCommand(error) => write!(f, "policy date command failed: {error}"),
            Self::DateStatus(status) => write!(f, "policy date command exited with {status}"),
            Self::DateUtf8(error) => write!(f, "policy date command emitted non-UTF-8: {error}"),
            Self::ManifestWalk { path, source } => write_manifest_walk_error(f, path, source),
            Self::ManifestOutsideRoot { path, source } => fmt_outside(f, path, source),
            Self::RuleId(error) => write!(f, "rule id configuration error: {error}"),
        }
    }
}

/// Write the manifest-walk error display body.
///
/// # Errors
/// Returns the formatter write error.
fn write_manifest_walk_error(
    f: &mut fmt::Formatter<'_>,
    path: &Path,
    source: &io::Error,
) -> fmt::Result {
    write!(f, "cannot read manifest tree at {}: {source}", path.display())
}

/// Write the outside-root manifest error display body.
///
/// # Errors
/// Returns the formatter write error.
fn fmt_outside(f: &mut fmt::Formatter<'_>, path: &Path, source: &StripPrefixError) -> fmt::Result {
    write!(f, "collected manifest outside target root: {} ({source})", path.display())
}
