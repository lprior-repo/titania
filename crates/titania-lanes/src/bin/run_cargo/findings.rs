use serde_json::Value;
use titania_lanes::{Finding, LaneReport, RuleId};

use crate::lane::CargoLane;

pub(crate) fn collect_findings(
    lane: CargoLane,
    rule: &RuleId,
    stdout: &str,
    stderr: &str,
    report: &mut LaneReport,
) {
    match lane {
        CargoLane::Fmt => collect_fmt_findings(rule, stdout, stderr, report),
        CargoLane::Clippy => collect_clippy_findings(rule, stdout, stderr, report),
        CargoLane::Test => collect_test_findings(rule, stdout, stderr, report),
        CargoLane::Compile | CargoLane::Build => {
            collect_error_lines(rule, lane.path(), stderr, report);
        }
    }
}

fn collect_fmt_findings<'a>(
    rule: &RuleId,
    stdout: &'a str,
    stderr: &'a str,
    report: &mut LaneReport,
) {
    let mut collector = FmtCollector::new(rule);
    stdout.lines().chain(stderr.lines()).for_each(|line| collector.feed(line, report));
}

enum FmtLine<'a> {
    Diff(&'a str),
    Hunk,
    Ignore,
}

struct FmtCollector<'rule, 'line> {
    rule: &'rule RuleId,
    path: &'line str,
    saw_diff_header: bool,
}

impl<'rule, 'line> FmtCollector<'rule, 'line> {
    const fn new(rule: &'rule RuleId) -> Self {
        Self { rule, path: "cargo fmt", saw_diff_header: false }
    }

    fn feed(&mut self, line: &'line str, report: &mut LaneReport) {
        match classify_fmt_line(line, self.saw_diff_header) {
            FmtLine::Diff(rest) => self.record_diff(rest, report),
            FmtLine::Hunk => self.record_hunk(report),
            FmtLine::Ignore => (),
        }
    }

    fn record_diff(&mut self, rest: &'line str, report: &mut LaneReport) {
        self.path = strip_diff_path_suffix(rest);
        self.saw_diff_header = true;
        self.record_hunk(report);
    }

    fn record_hunk(&self, report: &mut LaneReport) {
        report.push(Finding::new(self.rule.clone(), self.path, 0, "rustfmt diff hunk"));
    }
}

fn strip_diff_path_suffix(rest: &str) -> &str {
    rest.strip_suffix(':').map_or(rest, |stripped| stripped)
}

fn classify_fmt_line(line: &str, saw_diff_header: bool) -> FmtLine<'_> {
    line.strip_prefix("Diff in ")
        .map_or_else(|| classify_fmt_hunk(line, saw_diff_header), FmtLine::Diff)
}

fn classify_fmt_hunk(line: &str, saw_diff_header: bool) -> FmtLine<'_> {
    if line.starts_with("@@") && !saw_diff_header { FmtLine::Hunk } else { FmtLine::Ignore }
}

fn collect_clippy_findings(rule: &RuleId, stdout: &str, stderr: &str, report: &mut LaneReport) {
    stdout
        .lines()
        .filter_map(clippy_diagnostic_from_line)
        .for_each(|diagnostic| push_clippy_diagnostic(rule, diagnostic, report));
    collect_error_lines_when_clean(rule, "cargo clippy", stderr, report);
}

struct ClippyDiagnostic {
    path: String,
    line_no: u32,
    text: String,
}

impl ClippyDiagnostic {
    fn from_message(message: &Value) -> Self {
        let text = message_text(message);
        let (path, line_no) = message_location(message);
        Self { path, line_no, text }
    }
}

fn clippy_diagnostic_from_line(line: &str) -> Option<ClippyDiagnostic> {
    let value = serde_json::from_str::<Value>(line).ok()?;
    clippy_diagnostic_from_value(&value)
}

fn clippy_diagnostic_from_value(value: &Value) -> Option<ClippyDiagnostic> {
    let message = value.get("message")?;
    is_compiler_message(value)
        .then_some(message)
        .filter(|candidate| has_reportable_level(candidate))
        .map(ClippyDiagnostic::from_message)
}

fn is_compiler_message(value: &Value) -> bool {
    value.get("reason").and_then(Value::as_str) == Some("compiler-message")
}

fn has_reportable_level(message: &Value) -> bool {
    matches!(message.get("level").and_then(Value::as_str), Some("warning" | "error"))
}

fn push_clippy_diagnostic(rule: &RuleId, diagnostic: ClippyDiagnostic, report: &mut LaneReport) {
    report.push(Finding::new(rule.clone(), diagnostic.path, diagnostic.line_no, diagnostic.text));
}

fn collect_error_lines_when_clean(rule: &RuleId, path: &str, text: &str, report: &mut LaneReport) {
    if report.is_clean() {
        collect_error_lines(rule, path, text, report);
    }
}

fn collect_test_findings(rule: &RuleId, stdout: &str, stderr: &str, report: &mut LaneReport) {
    stdout
        .lines()
        .chain(stderr.lines())
        .filter_map(failed_test_message)
        .for_each(|message| push_test_finding(rule, message, report));
}

fn failed_test_message(line: &str) -> Option<String> {
    failed_test_name(line).map(|name| format!("test failed: {name}"))
}

fn push_test_finding(rule: &RuleId, message: String, report: &mut LaneReport) {
    report.push(Finding::new(rule.clone(), "cargo test", 0, message));
}

fn collect_error_lines(rule: &RuleId, path: &str, text: &str, report: &mut LaneReport) {
    text.lines()
        .filter(|line| is_error_line(line))
        .for_each(|line| push_error_line(rule, path, line, report));
}

fn is_error_line(line: &str) -> bool {
    line.starts_with("error[") || line.starts_with("error:")
}

fn push_error_line(rule: &RuleId, path: &str, line: &str, report: &mut LaneReport) {
    report.push(Finding::new(rule.clone(), path, 0, line.to_owned()));
}

fn failed_test_name(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("test ")?;
    rest.strip_suffix(" ... FAILED")
}

fn message_text(message: &Value) -> String {
    let text = message
        .get("rendered")
        .and_then(Value::as_str)
        .or_else(|| message.get("message").and_then(Value::as_str))
        .map_or("cargo clippy diagnostic", |value| value);
    text.trim().to_owned()
}

fn message_location(message: &Value) -> (String, u32) {
    primary_span(message).map_or_else(default_clippy_location, span_location)
}

fn primary_span(message: &Value) -> Option<&Value> {
    message.get("spans")?.as_array()?.iter().find(|span| is_primary_span(span))
}

fn is_primary_span(span: &Value) -> bool {
    span.get("is_primary") == Some(&Value::Bool(true))
}

fn default_clippy_location() -> (String, u32) {
    (String::from("cargo clippy"), 0)
}

fn span_location(span: &Value) -> (String, u32) {
    (span_file_name(span), span_line_no(span))
}

fn span_file_name(span: &Value) -> String {
    span.get("file_name").and_then(Value::as_str).map_or("cargo clippy", |value| value).to_owned()
}

fn span_line_no(span: &Value) -> u32 {
    span.get("line_start")
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
        .map_or(0, |value| value)
}
