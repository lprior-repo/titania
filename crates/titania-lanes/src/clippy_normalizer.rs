//! Pure Clippy JSONL normalization into typed lane findings.
//!
//! The runner owns subprocess execution. This module owns only data conversion
//! from Cargo/Clippy JSON lines into stable `CLIPPY_*` rule identifiers.

use serde_json::Value;
use titania_core::LaneFailure;

use crate::{Finding, LaneReport, RuleId};

const TOOL: &str = "cargo clippy";
const CLIPPY_UNKNOWN: &str = "CLIPPY_UNKNOWN";

/// Result of normalizing one Clippy JSONL stream.
#[derive(Debug, Clone)]
pub enum ClippyNormalization {
    /// Parsed Clippy diagnostics as typed findings.
    Findings(LaneReport),
    /// No usable diagnostics were present and the stream itself was malformed.
    SuspiciousFailure(LaneFailure),
}

impl ClippyNormalization {
    /// Whether the normalized stream contains no findings or failure.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        matches!(self, Self::Findings(report) if report.is_clean())
    }

    /// Borrow normalized findings. Suspicious failures contain no code findings.
    #[must_use]
    pub fn findings(&self) -> &[Finding] {
        match self {
            Self::Findings(report) => report.findings(),
            Self::SuspiciousFailure(_) => &[],
        }
    }

    /// Count normalized findings.
    #[must_use]
    pub fn finding_count(&self) -> usize {
        self.findings().len()
    }

    /// Render normalized findings or suspicious failure evidence.
    #[must_use]
    pub fn render(&self) -> String {
        match self {
            Self::Findings(report) => report.render(),
            Self::SuspiciousFailure(failure) => format!("{TOOL}: {failure:?}\n"),
        }
    }

    /// Borrow suspicious failure details when normalization failed suspiciously.
    #[must_use]
    pub const fn failure(&self) -> Option<&LaneFailure> {
        match self {
            Self::Findings(_) => None,
            Self::SuspiciousFailure(failure) => Some(failure),
        }
    }
}

/// Normalize Cargo/Clippy JSONL output into typed findings.
#[must_use]
pub fn normalize_clippy_jsonl(input: &str) -> ClippyNormalization {
    input.lines().fold(NormalizerState::default(), NormalizerState::feed).finish()
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
        self.report.extend_finding(
            ClippyDiagnostic::from_value(value).map(ClippyDiagnostic::into_finding),
        );
    }

    fn finish(self) -> ClippyNormalization {
        suspicious_candidate(self.report.is_clean(), self.malformed_lines).map_or_else(
            || ClippyNormalization::Findings(self.report),
            ClippyNormalization::SuspiciousFailure,
        )
    }
}

#[derive(Debug)]
struct ClippyDiagnostic {
    rule: RuleId,
    path: String,
    line: u32,
    message: String,
}

impl ClippyDiagnostic {
    fn from_value(value: &Value) -> Option<Self> {
        let message = compiler_message(value)?;
        reportable_level(message).then_some(())?;
        let lint_name = lint_code(message)?;
        let rule = rule_for_lint(lint_name, message)?;
        let (path, line_no) = message_location(message);
        Some(Self { rule, path, line: line_no, message: message_text(message, lint_name) })
    }

    fn into_finding(self) -> Finding {
        Finding::new(self.rule, self.path, self.line, self.message)
    }
}

fn compiler_message(value: &Value) -> Option<&Value> {
    is_compiler_message(value)
        .then_some(())
        .and_then(|()| value.get("message").or_else(|| value.get("compiler_message")))
}

fn is_compiler_message(value: &Value) -> bool {
    value.get("reason").and_then(Value::as_str) == Some("compiler-message")
}

fn reportable_level(message: &Value) -> bool {
    matches!(diagnostic_level(message), Some("warning" | "error"))
}

fn diagnostic_level(message: &Value) -> Option<&str> {
    message
        .get("level")
        .and_then(Value::as_str)
        .or_else(|| message.pointer("/code/severity").and_then(Value::as_str))
}

fn lint_code(message: &Value) -> Option<&str> {
    message.pointer("/code/code").and_then(Value::as_str)
}

fn rule_for_lint(raw: &str, message: &Value) -> Option<RuleId> {
    let lint = normalized_lint_name(raw);
    is_unknown_lint_message(message)
        .then(|| RuleId::new(CLIPPY_UNKNOWN).ok())
        .flatten()
        .or_else(|| typed_rule(&lint))
        .or_else(|| RuleId::new(CLIPPY_UNKNOWN).ok())
}

fn is_unknown_lint_message(message: &Value) -> bool {
    message_text_raw(message).contains("unknown lint")
}

fn normalized_lint_name(raw: &str) -> String {
    raw.strip_prefix("clippy::")
        .map_or(raw, |stripped| stripped)
        .chars()
        .map(|ch| if ch == '-' { '_' } else { ch })
        .collect()
}

fn typed_rule(lint: &str) -> Option<RuleId> {
    RuleId::new(&format!("CLIPPY_{}", lint.to_ascii_uppercase())).ok()
}

fn message_location(message: &Value) -> (String, u32) {
    primary_span(message).map_or_else(default_location, span_location)
}

fn primary_span(message: &Value) -> Option<&Value> {
    message.get("spans")?.as_array()?.iter().find(|span| is_primary_span(span))
}

fn is_primary_span(span: &Value) -> bool {
    span.get("is_primary") == Some(&Value::Bool(true))
}

fn default_location() -> (String, u32) {
    (TOOL.to_owned(), 0)
}

fn span_location(span: &Value) -> (String, u32) {
    (span_file_name(span), span_line(span))
}

fn span_file_name(span: &Value) -> String {
    span.get("file_name").and_then(Value::as_str).map_or(TOOL, |value| value).to_owned()
}

fn span_line(span: &Value) -> u32 {
    span.get("line_start")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .map_or(0, |value| value)
}

fn message_text(message: &Value, lint: &str) -> String {
    let base = message_text_raw(message);
    if base.contains(lint) { base.to_owned() } else { format!("{base}\n[clippy lint: {lint}]") }
}

fn message_text_raw(message: &Value) -> &str {
    message
        .get("rendered")
        .and_then(Value::as_str)
        .or_else(|| message.get("message").and_then(Value::as_str))
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map_or(TOOL, |text| text)
}

fn suspicious_candidate(is_clean: bool, malformed_lines: usize) -> Option<LaneFailure> {
    (is_clean && malformed_lines > 0).then(|| suspicious_failure(malformed_lines))
}

fn suspicious_failure(malformed_lines: usize) -> LaneFailure {
    LaneFailure::SuspiciousFailure {
        tool: TOOL.to_owned(),
        evidence: format!("malformed clippy JSON lines: {malformed_lines}"),
    }
}
