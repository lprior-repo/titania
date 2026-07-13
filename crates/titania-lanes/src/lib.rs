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

use titania_core::{RepairHint, TargetProject};
pub mod artifact_writer;
pub mod ast_grep_lane;
pub mod clippy_normalizer;
pub mod command;
pub mod deny_normalizer;
pub mod discover;
pub mod dylint_lane;
pub mod helpers;
pub mod policy_scan;

pub mod run_lane;
pub mod source_line;

pub use command::{CommandBudget, CommandIn, CommandOutput, EnvPolicy, LaneError, OutputStream};
pub use discover::{CurrentTargetError, discover_target, target_project_from_path, try_from_path};
pub use run_lane::run_lane_sources::{SourceWalkError, collect_rust_sources};
pub use source_line::{SourceLine, SourceLineState};
pub use titania_core::{RuleId, RuleIdError};

/// Discover the target Rust project from the current working directory.
///
/// Thin re-export over [`discover::current_target_project`] so existing
/// `titania_lanes::current_target_project` callers compile unchanged.
///
/// # Errors
/// Returns [`CurrentTargetError::CurrentDir`] when CWD cannot be read and
/// [`CurrentTargetError::Target`] when no valid Cargo target project can
/// be discovered from that directory.
pub fn current_target_project() -> Result<TargetProject, CurrentTargetError> {
    crate::discover::current_target_project()
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
    /// Typed remediation hint. Auto-populated from the `rule_id` by the
    /// single-source-of-truth catalog in [`titania_core::repair_hint`];
    /// overridden via [`Finding::with_repair`] when a normalizer has
    /// richer context (a precise `Patch` range, crate-name in `from`,
    /// etc.) than the catalog row.
    repair: RepairHint,
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
        let repair = titania_core::RepairHint::for_rule(rule.as_str());
        Self { rule, path: path.into(), line, message: message.into(), repair }
    }

    /// Override the auto-populated remediation hint. Use this when a
    /// normalizer has richer context than the catalog (e.g. a precise
    /// `Patch` with a line-exact `TextRange`, or a `ReplaceDependency`
    /// whose `from` carries the banned crate name).
    #[must_use]
    pub fn with_repair(mut self, repair: RepairHint) -> Self {
        self.repair = repair;
        self
    }

    /// Borrow the typed remediation hint.
    #[must_use]
    pub const fn repair(&self) -> &RepairHint {
        &self.repair
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
/// subject to judge. Per v1-spec §12: exit `1` is Reject (violations and/or
/// gate failures), `2` is `PolicyError`, `3` is `InputError` (usage/config),
/// `4`+ is Internal error (infrastructure or internal failure).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneExit {
    /// Lane ran and reported no violations.
    Clean,
    /// Lane had no valid subject to judge; maps to process exit code `0`.
    NotApplicable,
    /// Lane ran and reported one or more violations, or the lane's tool
    /// terminated abnormally (gate failure). Per v1-spec §12, both are
    /// Reject (exit 1).
    Violations,
    /// Lane exited with a usage/argument/config error. Per v1-spec §12,
    /// this is `InputError` (exit `3`).
    Usage,
    /// Lane failed to run (infrastructure or internal error). Per v1-spec
    /// §12, this is Internal error (exit `>=4`).
    Failure,
}

impl LaneExit {
    /// Stable process exit code per v1-spec §12. [`LaneExit::NotApplicable`]
    /// returns `0` because a non-applicable lane is a successful process
    /// completion.
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::Clean | Self::NotApplicable => 0,
            Self::Violations => 1,
            Self::Usage => 3,
            Self::Failure => 4,
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

    /// Per v1-spec §12: 0 = Pass, 1 = Reject, 2 = PolicyError,
    /// 3 = InputError, >=4 = Internal error.
    #[test]
    fn as_u8_matches_v1_spec_section_12_exit_codes() {
        assert_eq!(LaneExit::Clean.as_u8(), 0);
        assert_eq!(LaneExit::NotApplicable.as_u8(), 0);
        assert_eq!(LaneExit::Violations.as_u8(), 1);
        assert_eq!(LaneExit::Usage.as_u8(), 3);
        assert_eq!(LaneExit::Failure.as_u8(), 4);
    }
}
