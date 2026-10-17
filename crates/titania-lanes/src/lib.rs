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
pub mod proof_registry;

pub mod run_lane;
pub mod source_line;
pub mod trust_binding;

pub use command::{CommandBudget, CommandIn, CommandOutput, EnvPolicy, LaneError, OutputStream};
pub use discover::{CurrentTargetError, discover_target, target_project_from_path, try_from_path};
pub use run_lane::run_lane_sources::{SourceWalkError, SourceWalkResource, collect_rust_sources};
pub use source_line::{SourceLine, SourceLineState};
pub use titania_core::{RuleId, RuleIdError};

pub fn current_target_project() -> Result<TargetProject, CurrentTargetError> {
    crate::discover::current_target_project()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    rule: RuleId,
    path: String,
    line: u32,
    message: String,
    repair: RepairHint,
}
impl Finding {
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

    #[must_use]
    pub fn with_repair(mut self, repair: RepairHint) -> Self {
        self.repair = repair;
        self
    }

    #[must_use]
    pub const fn repair(&self) -> &RepairHint {
        &self.repair
    }

    #[must_use]
    pub const fn rule(&self) -> &RuleId {
        &self.rule
    }

    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub const fn line(&self) -> u32 {
        self.line
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    fn rendered_line(&self) -> String {
        format!("{}:{}: {} -- {}\n", self.path, self.line, self.rule.as_str(), self.message)
    }
}

#[derive(Debug, Default, Clone)]
pub struct LaneReport {
    findings: Vec<Finding>,
    scanned: u32,
    passed: u32,
}

impl LaneReport {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }

    pub fn push(&mut self, finding: Finding) {
        self.findings.push(finding);
    }

    pub fn extend_finding(&mut self, findings: impl IntoIterator<Item = Finding>) {
        self.findings.extend(findings);
    }

    #[must_use]
    pub fn findings(&self) -> &[Finding] {
        &self.findings
    }

    #[must_use]
    pub fn finding_count(&self) -> usize {
        self.findings.len()
    }

    pub const fn record_pass(&mut self) {
        self.passed = self.passed.saturating_add(1);
    }

    pub const fn record_scan(&mut self) {
        self.scanned = self.scanned.saturating_add(1);
    }

    #[must_use]
    pub fn render(&self) -> String {
        self.findings.iter().map(Finding::rendered_line).collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneExit {
    Clean,
    NotApplicable,
    Violations,
    Usage,
    Failure,
}

impl LaneExit {
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

    #[test]
    fn as_u8_matches_v1_spec_section_12_exit_codes() {
        assert_eq!(LaneExit::Clean.as_u8(), 0);
        assert_eq!(LaneExit::NotApplicable.as_u8(), 0);
        assert_eq!(LaneExit::Violations.as_u8(), 1);
        assert_eq!(LaneExit::Usage.as_u8(), 3);
        assert_eq!(LaneExit::Failure.as_u8(), 4);
    }
}
