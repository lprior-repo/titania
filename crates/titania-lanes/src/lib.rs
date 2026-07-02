//! Pure-domain helpers shared by the titania-lanes CI/CD binaries.
//!
//! Each lane binary lives in `src/bin/<name>.rs` and follows the same
//! shape:
//!
//! 1. Parse argv into a `LaneInput` (path, mode, scope).
//! 2. Run pure check calculations (data → calc → actions layering).
//! 3. Emit typed findings and an exit code.
//!
//! No binary here does I/O outside the filesystem reads the bash
//! originals did. No async, no `unsafe`, no `unwrap`.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]
#![deny(clippy::string_slice)]
#![deny(clippy::get_unwrap)]
#![deny(clippy::arithmetic_side_effects)]
#![deny(clippy::dbg_macro)]
#![deny(clippy::as_conversions)]
#![forbid(unsafe_code)]

use core::fmt::Write as _;
use std::{env, io};

use thiserror::Error;
use titania_core::{TargetProject, TargetProjectError, discover_target};

pub mod command;
pub mod helpers;
pub mod source_line;

pub use command::{CommandBudget, CommandIn, CommandOutput, EnvPolicy, LaneError, OutputStream};
pub use source_line::SourceLine;

/// Errors produced while resolving the target project from the process CWD.
#[derive(Debug, Error)]
pub enum CurrentTargetError {
    /// CWD could not be read.
    #[error("cannot read current directory")]
    CurrentDir(#[source] io::Error),
    /// CWD was read but no valid Cargo target project was discovered.
    #[error(transparent)]
    Target(#[from] TargetProjectError),
}

/// Discover the target Rust project from the current working directory.
///
/// Lanes are launched from the project they should judge; this helper is the
/// single adapter that turns the ambient CWD into the typed `TargetProject`
/// value used by subprocess code.
///
/// # Errors
/// Returns [`CurrentTargetError::CurrentDir`] when CWD cannot be read and
/// [`CurrentTargetError::Target`] when no valid Cargo target project can be
/// discovered from that directory.
pub fn current_target_project() -> Result<TargetProject, CurrentTargetError> {
    let cwd = env::current_dir().map_err(CurrentTargetError::CurrentDir)?;
    discover_target(&cwd).map_err(CurrentTargetError::Target)
}
/// One typed finding produced by a lane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Stable lane-internal rule id, e.g. `"DISCARD-001"`.
    rule: &'static str,
    /// Repository-relative path that produced the finding.
    path: String,
    /// 1-indexed line number in `path`. `0` if the finding is file-level.
    line: u32,
    /// Human-readable message.
    message: String,
}

impl Finding {
    /// Construct a new [`Finding`] from its rule, path, line, and message.
    #[must_use]
    pub fn new(
        rule: &'static str,
        path: impl Into<String>,
        line: u32,
        message: impl Into<String>,
    ) -> Self {
        Self { rule, path: path.into(), line, message: message.into() }
    }

    /// Borrow the rule id (e.g. `"DISCARD-001"`).
    #[must_use]
    pub const fn rule(&self) -> &'static str {
        self.rule
    }

    /// Borrow the repository-relative path.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// 1-indexed line number. `0` means the finding is file-level.
    #[must_use]
    pub const fn line(&self) -> u32 {
        self.line
    }

    /// Borrow the human-readable message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Lane output: collected findings plus summary counters.
#[derive(Debug, Default, Clone)]
pub struct LaneReport {
    /// All findings accumulated so far.
    findings: Vec<Finding>,
    /// Total items scanned by the lane.
    scanned: u32,
    /// Items accepted as clean.
    passed: u32,
}

impl LaneReport {
    /// Construct an empty report.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the report carries zero findings.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }

    /// Append a finding to the report.
    pub fn push(&mut self, finding: Finding) {
        self.findings.push(finding);
    }

    /// Borrow the list of findings.
    #[must_use]
    pub fn findings(&self) -> &[Finding] {
        &self.findings
    }

    /// Number of findings currently in the report.
    #[must_use]
    pub fn finding_count(&self) -> usize {
        self.findings.len()
    }

    /// Mark one item as having passed; saturates at `u32::MAX`.
    pub const fn record_pass(&mut self) {
        self.passed = self.passed.saturating_add(1);
    }

    /// Mark one item as having been scanned; saturates at `u32::MAX`.
    pub const fn record_scan(&mut self) {
        self.scanned = self.scanned.saturating_add(1);
    }

    /// Stable `path:line: rule -- message` line for each finding.
    #[must_use]
    pub fn render(&self) -> String {
        self.findings.iter().fold(String::new(), |mut out, f| {
            match writeln!(out, "{}:{}: {} -- {}", f.path, f.line, f.rule, f.message) {
                Ok(()) | Err(_) => out,
            }
        })
    }
}

/// Typed process/disposition convention used by every lane binary.
///
/// `LaneExit::Clean` and `LaneExit::NotApplicable` both map to process exit
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneExit {
    /// Lane scanned and emitted zero findings; process exit 0.
    Clean,
    /// Lane had no valid subject to judge; process exit 0 (distinct from
    /// [`LaneExit::Clean`] for report/receipt semantics).
    NotApplicable,
    /// Lane emitted at least one finding; process exit 1.
    Violations,
    /// Lane was invoked with bad arguments or config; process exit 2.
    Usage,
    /// Lane could not run because of an upstream or fixture failure;
    /// process exit 3.
    Failure,
}

impl LaneExit {
    /// Stable process exit code. [`LaneExit::NotApplicable`] returns `0`
    /// because a non-applicable lane is a successful process completion.
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::Clean | Self::NotApplicable => 0,
            Self::Violations => 1,
            Self::Usage => 2,
            Self::Failure => 3,
        }
    }
}

/// Small wrapper around `std::process::ExitCode` so bins can `run` and
/// the test harness can `assert_eq!` on the underlying value.
#[must_use]
pub fn exit(code: LaneExit) -> std::process::ExitCode {
    std::process::ExitCode::from(code.as_u8())
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::disallowed_macros,
        clippy::disallowed_methods,
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::missing_panics_doc,
        reason = "Inline tests in the lib are exempt from the strict production deny list per project doctrine."
    )]
    use super::LaneExit;

    #[test]
    fn not_applicable_is_successful_process_exit_with_distinct_disposition() {
        assert_eq!(LaneExit::NotApplicable.as_u8(), 0);
        assert_ne!(LaneExit::NotApplicable, LaneExit::Clean);
    }
}
