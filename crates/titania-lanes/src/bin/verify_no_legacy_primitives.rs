//! Rejects 'parallel' / 'aggregate' inside `STEP_PRIMITIVES` + `ALLOWED_STEP_FIELDS` constants.
//!
//! Rust re-implementation of the bash lane `scripts/verify-no-legacy-primitives.sh`. Run via
//! `cargo run --bin verify-no-legacy-primitives --` from the repository root or via the
//! matching Moon task in `.moon/tasks/all.yml`.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use std::{
    fs,
    io::{self, ErrorKind, Write},
    path::PathBuf,
};

use titania_core::TargetProject;
use titania_lanes::{Finding, LaneExit, LaneReport, RuleId, current_target_project, exit};

const FORBIDDEN: &[&str] = &["\"parallel\"", "\"aggregate\""];
const LEGACY_PRIM_RULE: &str = "LEGACY_PRIM";
const SOURCES: &[TargetRelativePath] = &[
    TargetRelativePath::new("crates/vb_validate/src/schema.rs"),
    TargetRelativePath::new("crates/vb_validate/src/schema_fields.rs"),
];

#[derive(Clone, Copy)]
struct TargetRelativePath {
    value: &'static str,
}

impl TargetRelativePath {
    const fn new(value: &'static str) -> Self {
        Self { value }
    }

    const fn as_str(self) -> &'static str {
        self.value
    }

    fn in_target(self, target: &TargetProject) -> PathBuf {
        target.as_std_path().join(self.value)
    }
}

fn main() -> std::process::ExitCode {
    let target = match current_target_project() {
        Ok(target) => target,
        Err(error) => {
            return exit_after_stderr_line(
                format_args!("[verify-no-legacy-primitives] target discovery failed: {error}"),
                LaneExit::Failure,
            );
        }
    };
    let mut report = LaneReport::new();
    let rule = match RuleId::new(LEGACY_PRIM_RULE) {
        Ok(rule) => rule,
        Err(error) => {
            return exit_after_stderr_line(
                format_args!("[verify-no-legacy-primitives] rule id configuration error: {error}"),
                LaneExit::Failure,
            );
        }
    };
    run(&target, &rule, &mut report);
    print_and_exit(&report)
}

fn run(target: &TargetProject, rule: &RuleId, report: &mut LaneReport) {
    let findings: Vec<Finding> =
        SOURCES.iter().flat_map(|rel| check_file(target, *rel, rule)).collect();
    report.extend_finding(findings);
}

fn print_and_exit(report: &LaneReport) -> std::process::ExitCode {
    let rendered = report.render();
    if !rendered.is_empty() && write_stderr(format_args!("{rendered}")).is_err() {
        return exit(LaneExit::Failure);
    }
    if report.is_clean() { exit(LaneExit::Clean) } else { exit(LaneExit::Violations) }
}

fn check_file(target: &TargetProject, rel: TargetRelativePath, rule: &RuleId) -> Vec<Finding> {
    let path = rel.in_target(target);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return optional_source_absent_findings(rule, rel);
        }
        Err(error) => {
            return vec![Finding::new(
                rule.clone(),
                rel.as_str(),
                0,
                format!("file not readable: {:?}", error.kind()),
            )];
        }
    };
    let mut findings =
        check_const_body(&text, rel, "const STEP_PRIMITIVES", "STEP_PRIMITIVES", rule);
    findings.extend(check_const_body(
        &text,
        rel,
        "const ALLOWED_STEP_FIELDS",
        "ALLOWED_STEP_FIELDS",
        rule,
    ));
    findings
}

fn check_const_body(
    text: &str,
    rel: TargetRelativePath,
    marker: &str,
    label: &str,
    rule: &RuleId,
) -> Vec<Finding> {
    let Some(body) = extract_const_body(text, marker) else {
        return optional_constant_absent_findings(rule, rel, label);
    };
    FORBIDDEN
        .iter()
        .filter(|bad| body.contains(**bad))
        .map(|bad| Finding::new(rule.clone(), rel.as_str(), 0, format!("{label} contains {bad}")))
        .collect()
}

fn extract_const_body<'a>(text: &'a str, marker: &str) -> Option<&'a str> {
    let start = text.find(marker)?;
    let after_marker = text.get(start..)?;
    let equals_offset = after_marker.find('=')?;
    let scan_start = start.saturating_add(equals_offset);
    let scan = text.get(scan_start..)?;
    let (open_pos, open, close) = scan.char_indices().find_map(|(offset, ch)| match ch {
        '[' => Some((scan_start.saturating_add(offset), '[', ']')),
        '{' => Some((scan_start.saturating_add(offset), '{', '}')),
        _ => None,
    })?;
    extract_balanced(text, start, open_pos, open, close)
}

fn extract_balanced(
    text: &str,
    start: usize,
    open_pos: usize,
    open: char,
    close: char,
) -> Option<&str> {
    let tail = text.get(open_pos..)?;
    let mut depth: i32 = 0;
    let context = BalanceContext { text, start, open_pos, open, close };
    tail.char_indices().find_map(|(offset, ch)| context.extract_end(offset, ch, &mut depth))
}

struct BalanceContext<'a> {
    text: &'a str,
    start: usize,
    open_pos: usize,
    open: char,
    close: char,
}

impl<'a> BalanceContext<'a> {
    fn extract_end(&self, offset: usize, ch: char, depth: &mut i32) -> Option<&'a str> {
        match balance_step(ch, self.open, self.close, depth) {
            BalanceStep::Continue => None,
            BalanceStep::Finished => self.extract_slice(offset, ch),
        }
    }

    fn extract_slice(&self, offset: usize, ch: char) -> Option<&'a str> {
        let end = self.open_pos.saturating_add(offset).saturating_add(ch.len_utf8());
        self.text.get(self.start..end)
    }
}

#[derive(Clone, Copy)]
enum BalanceStep {
    Continue,
    Finished,
}

const fn balance_step(ch: char, open: char, close: char, depth: &mut i32) -> BalanceStep {
    match (ch == open, ch == close) {
        (true, _) => {
            *depth = depth.saturating_add(1);
            BalanceStep::Continue
        }
        (false, true) => close_balance(depth),
        (false, false) => BalanceStep::Continue,
    }
}

const fn close_balance(depth: &mut i32) -> BalanceStep {
    *depth = depth.saturating_sub(1);
    match *depth {
        0_i32 => BalanceStep::Finished,
        _ => BalanceStep::Continue,
    }
}

fn optional_source_absent_findings(rule: &RuleId, rel: TargetRelativePath) -> Vec<Finding> {
    let write_result = write_stderr_line(format_args!(
        "[verify-no-legacy-primitives] not applicable: {} absent; skipping optional vb_validate primitive source",
        rel.as_str()
    ));
    if write_result.is_ok() {
        return Vec::new();
    }
    vec![Finding::new(
        rule.clone(),
        rel.as_str(),
        0,
        "stderr write failed while reporting optional source absence",
    )]
}

fn optional_constant_absent_findings(
    rule: &RuleId,
    rel: TargetRelativePath,
    label: &str,
) -> Vec<Finding> {
    let write_result = write_stderr_line(format_args!(
        "[verify-no-legacy-primitives] clean: {} has no {}; skipping optional constant",
        rel.as_str(),
        label
    ));
    if write_result.is_ok() {
        return Vec::new();
    }
    vec![Finding::new(
        rule.clone(),
        rel.as_str(),
        0,
        "stderr write failed while reporting optional constant absence",
    )]
}

/// Write formatted text to stderr without adding a newline.
///
/// # Errors
///
/// Returns an [`io::Error`] when stderr cannot be written.
fn write_stderr(args: std::fmt::Arguments<'_>) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    stderr.write_fmt(args)
}

/// Write formatted text to stderr followed by a newline.
///
/// # Errors
///
/// Returns an [`io::Error`] when stderr cannot be written.
fn write_stderr_line(args: std::fmt::Arguments<'_>) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    stderr.write_fmt(args)?;
    stderr.write_all(b"\n")
}

fn exit_after_stderr_line(args: std::fmt::Arguments<'_>, code: LaneExit) -> std::process::ExitCode {
    match write_stderr_line(args) {
        Ok(()) => exit(code),
        Err(_) => exit(LaneExit::Failure),
    }
}
