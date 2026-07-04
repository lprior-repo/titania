//! Pure cargo-deny JSON normalization into typed `DENY_*` findings.
//!
//! This module converts cargo-deny machine output into lane findings. It does
//! not run cargo-deny and does not read `deny.toml`.

use serde_json::Value;
use titania_core::LaneFailure;

use crate::{Finding, LaneReport, RuleId};

const TOOL: &str = "cargo-deny";
const RULE_ADVISORY: &str = "DENY_ADVISORY";
const RULE_LICENSE: &str = "DENY_LICENSE";
const RULE_BANNED: &str = "DENY_BANNED_CRATE";
const RULE_MULTIPLE: &str = "DENY_MULTIPLE_VERSIONS";
const RULE_UNKNOWN_REGISTRY: &str = "DENY_UNKNOWN_REGISTRY";
const RULE_UNKNOWN_GIT: &str = "DENY_UNKNOWN_GIT";
const RULE_UNKNOWN: &str = "DENY_UNKNOWN";
const RULE_INFRA: &str = "DENY_INFRA_FAILURE";

/// Result of normalizing cargo-deny output.
#[derive(Debug, Clone)]
pub enum DenyNormalization {
    /// Parsed cargo-deny diagnostics as typed findings.
    Findings(LaneReport),
    /// No usable diagnostics were present and the stream itself was malformed.
    SuspiciousFailure(LaneFailure),
    /// cargo-deny itself was unavailable.
    InfraFailure {
        /// Typed failure explaining that cargo-deny could not be invoked.
        failure: LaneFailure,
        /// Human-readable finding mirror for lane-report consumers.
        report: LaneReport,
    },
}

impl DenyNormalization {
    /// Whether the normalized output contains no findings or failure.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        matches!(self, Self::Findings(report) if report.is_clean())
    }

    /// Borrow normalized findings. Suspicious failures contain no code findings.
    #[must_use]
    pub fn findings(&self) -> &[Finding] {
        match self {
            Self::Findings(report) | Self::InfraFailure { report, .. } => report.findings(),
            Self::SuspiciousFailure(_) => &[],
        }
    }

    /// Count normalized findings.
    #[must_use]
    pub fn finding_count(&self) -> usize {
        self.findings().len()
    }

    /// Render normalized findings or failure evidence.
    #[must_use]
    pub fn render(&self) -> String {
        match self {
            Self::Findings(report) | Self::InfraFailure { report, .. } => report.render(),
            Self::SuspiciousFailure(failure) => format!("{TOOL}: {failure:?}\n"),
        }
    }

    /// Borrow failure details when the result represents a tool failure.
    #[must_use]
    pub const fn failure(&self) -> Option<&LaneFailure> {
        match self {
            Self::Findings(_) => None,
            Self::SuspiciousFailure(failure) | Self::InfraFailure { failure, .. } => Some(failure),
        }
    }
}

/// Normalize cargo-deny JSON or JSONL output into typed findings.
#[must_use]
pub fn normalize_deny_json(input: &str) -> DenyNormalization {
    input.lines().fold(NormalizerState::default(), NormalizerState::feed).finish()
}

/// Build the typed result for a missing cargo-deny binary.
#[must_use]
pub fn deny_missing_binary() -> DenyNormalization {
    DenyNormalization::InfraFailure {
        failure: LaneFailure::InfraFailure {
            tool: TOOL.to_owned(),
            reason: String::from("cargo-deny binary is unavailable"),
        },
        report: single_finding_report(RULE_INFRA, TOOL, 0, "cargo-deny binary is unavailable"),
    }
}

#[derive(Debug, Default)]
struct NormalizerState {
    report: LaneReport,
    malformed_lines: usize,
}

impl NormalizerState {
    fn feed(self, line: &str) -> Self {
        match line.trim() {
            "" => self,
            trimmed => self.feed_json(trimmed),
        }
    }

    fn feed_json(mut self, line: &str) -> Self {
        match serde_json::from_str::<Value>(line) {
            Ok(value) => self.record_value(&value),
            Err(_) => self.malformed_lines = self.malformed_lines.saturating_add(1),
        }
        self
    }

    fn record_value(&mut self, value: &Value) {
        self.report
            .extend_finding(DenyDiagnostic::from_value(value).map(DenyDiagnostic::into_finding));
    }

    fn finish(self) -> DenyNormalization {
        suspicious_candidate(self.report.is_clean(), self.malformed_lines).map_or_else(
            || DenyNormalization::Findings(self.report),
            DenyNormalization::SuspiciousFailure,
        )
    }
}

#[derive(Debug)]
struct DenyDiagnostic {
    rule: RuleId,
    path: String,
    line: u32,
    message: String,
}

impl DenyDiagnostic {
    fn from_value(value: &Value) -> Option<Self> {
        let fields = diagnostic_fields(value)?;
        reportable_severity(fields).then_some(())?;
        let message = diagnostic_message(fields);
        let rule = rule_for_diagnostic(fields, &message)?;
        let (path, line) = diagnostic_location(fields);
        Some(Self { rule, path, line, message })
    }

    fn into_finding(self) -> Finding {
        Finding::new(self.rule, self.path, self.line, self.message)
    }
}

fn diagnostic_fields(value: &Value) -> Option<&Value> {
    (value.get("type").and_then(Value::as_str) == Some("diagnostic"))
        .then_some(())
        .and_then(|()| value.get("fields"))
}

fn reportable_severity(fields: &Value) -> bool {
    matches!(fields.get("severity").and_then(Value::as_str), Some("warning" | "error"))
}

fn diagnostic_message(fields: &Value) -> String {
    fields
        .get("message")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .map_or(TOOL, |message| message)
        .to_owned()
}

fn rule_for_diagnostic(fields: &Value, message: &str) -> Option<RuleId> {
    let code = fields.get("code").and_then(Value::as_str);
    RuleId::new(rule_name(code, message)).ok()
}

fn rule_name(code: Option<&str>, message: &str) -> &'static str {
    match code {
        Some("vulnerability" | "notice" | "unmaintained" | "unsound" | "yanked") => RULE_ADVISORY,
        Some("rejected" | "unlicensed") => RULE_LICENSE,
        Some("banned") => RULE_BANNED,
        Some("duplicate") => RULE_MULTIPLE,
        Some("source-not-allowed") => source_rule(message),
        Some(value) if value.starts_with("rustsec-") => RULE_ADVISORY,
        _other => RULE_UNKNOWN,
    }
}

fn source_rule(message: &str) -> &'static str {
    (message.contains("unknown git") || message.contains("git source"))
        .then_some(RULE_UNKNOWN_GIT)
        .map_or(RULE_UNKNOWN_REGISTRY, |rule| rule)
}

fn diagnostic_location(fields: &Value) -> (String, u32) {
    primary_label(fields).map_or_else(default_location, label_location)
}

fn primary_label(fields: &Value) -> Option<&Value> {
    fields.get("labels")?.as_array()?.first()
}

fn default_location() -> (String, u32) {
    (TOOL.to_owned(), 0)
}

fn label_location(label: &Value) -> (String, u32) {
    (label_span(label), label_line(label))
}

fn label_span(label: &Value) -> String {
    label.get("span").and_then(Value::as_str).map_or(TOOL, |value| value).to_owned()
}

fn label_line(label: &Value) -> u32 {
    label
        .get("line")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .map_or(0, |value| value)
}

fn suspicious_candidate(is_clean: bool, malformed_lines: usize) -> Option<LaneFailure> {
    (is_clean && malformed_lines > 0).then(|| LaneFailure::SuspiciousFailure {
        tool: TOOL.to_owned(),
        evidence: format!("malformed cargo-deny JSON lines: {malformed_lines}"),
    })
}

fn single_finding_report(rule: &str, path: &str, line: u32, message: &str) -> LaneReport {
    let mut report = LaneReport::new();
    RuleId::new(rule)
        .ok()
        .map(|rule_id| Finding::new(rule_id, path, line, message.to_owned()))
        .into_iter()
        .for_each(|finding| report.push(finding));
    report
}
