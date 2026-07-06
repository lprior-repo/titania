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

use std::{env, io, path::Path};

use thiserror::Error;
use titania_core::{TargetProject, TargetProjectError, discover_target};

pub mod artifact_writer;
pub mod ast_grep_lane;
pub mod clippy_normalizer;
pub mod command;
pub mod deny_normalizer;
pub mod dylint_lane;
pub mod helpers;
pub mod policy_scan;
pub mod run_lane;
pub mod source_line;

pub use command::{CommandBudget, CommandIn, CommandOutput, EnvPolicy, LaneError, OutputStream};
pub use run_lane::run_lane_sources::{SourceWalkError, collect_rust_sources};
pub use source_line::{SourceLine, SourceLineState};
pub use titania_core::{RuleId, RuleIdError};

/// Errors produced while resolving the target project from the process CWD.
#[derive(Debug, Error)]
pub enum CurrentTargetError {
    /// The process current working directory could not be read.
    #[error("cannot read current directory")]
    CurrentDir(#[source] io::Error),
    /// No valid Cargo target project could be resolved from the CWD.
    #[error(transparent)]
    Target(#[from] TargetProjectError),
}

/// Construct a [`TargetProject`] from an arbitrary filesystem path.
///
/// Walks ancestors from the given path, reads manifests, and selects the
/// nearest workspace root (or single-package root). This is the pure core
/// of target-project resolution — it accepts any `&Path` and returns a
/// validated `TargetProject` or a typed error.
///
/// # Errors
/// Returns a [`TargetProjectError`] when the path cannot be resolved to
/// a valid Cargo target project.
///
/// # Pure core
/// This function performs no I/O beyond filesystem reads for manifest
/// discovery. It accepts `&Path` to allow callers to pass pre-validated
/// paths from other layers without requiring CWD resolution.
pub fn target_project_from_path(cwd: &Path) -> Result<TargetProject, TargetProjectError> {
    discover_target(cwd)
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
    target_project_from_path(&cwd).map_err(CurrentTargetError::Target)
}
/// One typed finding produced by a lane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Stable lane-internal rule id (validated [`RuleId`]).
    rule: RuleId,
    /// Repository-relative path that produced the finding.
    path: String,
    /// 1-indexed line number in `path`. `0` if the finding is file-level.
    line: u32,
    /// Human-readable message.
    message: String,
}

impl Finding {
    /// Construct a finding from a validated rule id, a repository-relative
    /// path, a 1-indexed line number (`0` for file-level), and a message.
    #[must_use]
    pub fn new(
        rule: RuleId,
        path: impl Into<String>,
        line: u32,
        message: impl Into<String>,
    ) -> Self {
        Self { rule, path: path.into(), line, message: message.into() }
    }

    /// Stable lane-internal rule id (validated [`RuleId`]).
    #[must_use]
    pub const fn rule(&self) -> &RuleId {
        &self.rule
    }

    /// Repository-relative path that produced the finding.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// 1-indexed line number; `0` for a file-level finding.
    #[must_use]
    pub const fn line(&self) -> u32 {
        self.line
    }

    /// Human-readable finding message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    fn rendered_line(&self) -> String {
        format!("{}:{}: {} -- {}\n", self.path, self.line, self.rule.as_str(), self.message)
    }
}

/// Lane output: collected findings plus summary counters.
#[derive(Debug, Default, Clone)]
pub struct LaneReport {
    findings: Vec<Finding>,
    scanned: u32,
    passed: u32,
}

impl LaneReport {
    /// Construct an empty report.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `true` when no findings have been recorded.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }

    /// Record a single typed finding.
    pub fn push(&mut self, finding: Finding) {
        self.findings.push(finding);
    }

    /// Append an iterator of findings to the report.
    pub fn extend_finding(&mut self, findings: impl IntoIterator<Item = Finding>) {
        self.findings.extend(findings);
    }

    /// Borrow the recorded findings.
    #[must_use]
    pub fn findings(&self) -> &[Finding] {
        &self.findings
    }

    /// Number of findings recorded.
    #[must_use]
    pub fn finding_count(&self) -> usize {
        self.findings.len()
    }

    /// Record one accepted (passed) item, saturating at `u32::MAX`.
    pub const fn record_pass(&mut self) {
        self.passed = self.passed.saturating_add(1);
    }

    /// Record one scanned item, saturating at `u32::MAX`.
    pub const fn record_scan(&mut self) {
        self.scanned = self.scanned.saturating_add(1);
    }

    /// Stable `path:line: rule -- message` line for each finding.
    #[must_use]
    pub fn render(&self) -> String {
        self.findings.iter().map(Finding::rendered_line).collect()
    }
}

/// Typed process/disposition convention used by every lane binary.
///
/// `LaneExit::Clean` and `LaneExit::NotApplicable` both map to process exit
/// code `0`, but they remain distinct lane/report dispositions: CI process
/// success differs from the receipt/report meaning that a lane had no valid
/// subject to judge. Other codes are `1` = violations, `2` = usage/config
/// error, `3` = upstream dependency missing or fixture self-test failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneExit {
    /// Lane ran and reported no violations.
    Clean,
    /// Lane had no valid subject to judge; maps to process exit code `0`.
    NotApplicable,
    /// Lane ran and reported one or more violations.
    Violations,
    /// Lane exited with a usage/argument/config error.
    Usage,
    /// Lane failed to run (infrastructure or internal error).
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
    use super::LaneExit;

    #[test]
    fn not_applicable_is_successful_process_exit_with_distinct_disposition() {
        assert_eq!(LaneExit::NotApplicable.as_u8(), 0);
        assert_ne!(LaneExit::NotApplicable, LaneExit::Clean);
    }
}
