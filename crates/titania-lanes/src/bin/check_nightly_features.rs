//! Scans `*.rs` for `#![feature(...)]` attributes and rejects disallowed
//! unstable feature names.
//!
//! Rust re-implementation of the bash lane `scripts/check-nightly-features.sh`. Run via
//! `cargo run --bin check-nightly-features --` from the repository root
//! or via the matching Moon task in `.moon/tasks/all.yml`.
//!
//! ## Behavior parity
//! Two allowed feature sets:
//! - `normal_allowed = ^(try_blocks|portable_simd)$`
//! - `perf_only_allowed = ^(allocator_api|generic_const_exprs)$` — but
//!   only when the file is in a perf scope
//!   (`crates/*/src/perf/*`, `crates/*/src/generated/*`, or `benches/*`)
//!   OR the file contains the `velvet-allow-perf-nightly-feature` marker.
//!
//! Anything else triggers a finding and the lane exits 1 (mapped to
//! `LaneExit::Violations` here).
//!
//! File enumeration mirrors the bash's `rg --files` call: `*.rs` only,
//! excluding the canonical build/VCS/cache paths shared with the
//! source-length lane.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

#[path = "check_nightly_features/boundary.rs"]
/// Exact-path waivers for non-production nightly-feature boundaries.
pub mod boundary;
#[path = "check_nightly_features/collector.rs"]
/// Feature-attribute collector for the nightly-feature lane.
pub mod collector;
#[path = "check_nightly_features/scope.rs"]
/// Perf-scope classification for nightly features.
pub mod scope;

use std::path::{Path, PathBuf};

use titania_core::TargetProject;
use titania_lanes::{
    Finding, LaneExit, LaneReport, RuleId, RuleIdError, current_target_project, exit,
    helpers::{is_excluded_source_path, normalize_slashes},
};

use boundary::is_dylint_boundary_feature;
use collector::collect_features;
use scope::{FeatureScope, ScopeSignals, classify_scope, is_perf_scoped_path};

/// Stable features allowed in any scope.
const NORMAL_ALLOWED: &[&str] = &["try_blocks", "portable_simd"];

/// Features allowed only in perf-scoped paths or files with the marker
/// comment.
const PERF_ONLY_ALLOWED: &[&str] = &["allocator_api", "generic_const_exprs"];

/// Marker string for opt-in perf feature use.
const PERF_MARKER: &str = "velvet-allow-perf-nightly-feature";

const NIGHTLY_FEATURE_DISALLOWED_RULE: &str = "NIGHTLY_FEATURE_001";
const NIGHTLY_FEATURE_PERF_SCOPE_RULE: &str = "NIGHTLY_FEATURE_002";

fn main() -> std::process::ExitCode {
    let target = match target_or_exit() {
        Ok(target) => target,
        Err(code) => return code,
    };
    let rules = match rules_or_exit() {
        Ok(rules) => rules,
        Err(code) => return code,
    };
    let mut report = LaneReport::new();
    for file in &collect_source_files(target.as_std_path()) {
        scan_file(file, &rules, &mut report);
    }
    if write_stderr_raw(format_args!("{}", report.render())).is_err() {
        return exit(LaneExit::Failure);
    }
    if report.is_clean() {
        exit_after_io(
            write_stderr_line(format_args!(
                "[check-nightly-features] no disallowed feature attributes"
            )),
            LaneExit::Clean,
        )
    } else {
        exit(LaneExit::Violations)
    }
}

/// Resolve the current target project or return the process exit code to use.
///
/// # Errors
///
/// Returns `Err(exit_code)` after writing a diagnostic when target discovery
/// fails.
fn target_or_exit() -> Result<TargetProject, std::process::ExitCode> {
    current_target_project().map_err(|error| {
        failure_after_stderr_line(format_args!(
            "[check-nightly-features] cannot resolve target project: {error}"
        ))
    })
}

/// Build nightly-feature rule identifiers or return the process exit code.
///
/// # Errors
///
/// Returns `Err(exit_code)` after writing a diagnostic when a rule id is
/// invalid.
fn rules_or_exit() -> Result<NightlyRules, std::process::ExitCode> {
    NightlyRules::new().map_err(|error| {
        failure_after_stderr_line(format_args!(
            "[check-nightly-features] rule id configuration error: {error}"
        ))
    })
}

struct NightlyRules {
    disallowed: RuleId,
    perf_scope: RuleId,
}

impl NightlyRules {
    /// Build rule identifiers for nightly-feature findings.
    ///
    /// # Errors
    ///
    /// Returns the invalid rule-id error if one of the configured rule ids
    /// violates the shared rule-id format.
    fn new() -> Result<Self, RuleIdError> {
        Ok(Self {
            disallowed: RuleId::new(NIGHTLY_FEATURE_DISALLOWED_RULE)?,
            perf_scope: RuleId::new(NIGHTLY_FEATURE_PERF_SCOPE_RULE)?,
        })
    }
}

fn collect_source_files(root: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    walk(root, &mut out);
    out.sort();
    out
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    entries.flatten().for_each(|entry| visit_entry(entry.path(), out));
}

fn visit_entry(path: PathBuf, out: &mut Vec<PathBuf>) {
    if is_excluded_entry(&path) {
        return;
    }
    if path.is_dir() {
        walk(&path, out);
    } else if path.extension().is_some_and(|e| e == "rs") {
        out.push(path);
    }
}

/// Apply the canonical shared exclusion to a walked path. The walker
/// starts at the project root, so strip the leading `./` before handing
/// the relative path to `is_excluded_source_path`.
fn is_excluded_entry(path: &Path) -> bool {
    let normalized = normalize_slashes(path);
    let rel = normalized.strip_prefix("./").map_or(normalized.as_str(), |rel| rel);
    is_excluded_source_path(rel)
}

fn scan_file(path: &Path, rules: &NightlyRules, report: &mut LaneReport) {
    report.record_scan();
    let Ok(content) = std::fs::read_to_string(path) else {
        return;
    };

    let display = path.display().to_string();
    let is_perf_scoped = is_perf_scoped_path(&display);
    let has_marker = content.contains(PERF_MARKER);
    let scope =
        classify_scope(ScopeSignals { perf_scoped: is_perf_scoped, marker_opt_in: has_marker });

    for (line_no, names, line_no_for_message) in collect_features(&content) {
        scan_feature_names(FeatureNameScan {
            display: &display,
            feature_line: line_no,
            names,
            report_line: line_no_for_message,
            scope,
            rules,
            report,
        });
    }
}

struct FeatureNameScan<'a> {
    display: &'a str,
    feature_line: u32,
    names: Vec<String>,
    report_line: u32,
    scope: FeatureScope,
    rules: &'a NightlyRules,
    report: &'a mut LaneReport,
}

fn scan_feature_names(scan: FeatureNameScan<'_>) {
    let FeatureNameScan { display, feature_line, names, report_line, scope, rules, report } = scan;
    for name in names.iter().map(|name| name.trim()).filter(|name| !name.is_empty()) {
        check_feature(FeatureCheck {
            file: display,
            feature_line,
            name,
            scope,
            rules,
            report_line,
            report,
        });
    }
}

struct FeatureCheck<'a> {
    file: &'a str,
    feature_line: u32,
    name: &'a str,
    scope: FeatureScope,
    rules: &'a NightlyRules,
    report_line: u32,
    report: &'a mut LaneReport,
}

fn check_feature(check: FeatureCheck<'_>) {
    if NORMAL_ALLOWED.contains(&check.name) || is_dylint_boundary_feature(check.file, check.name) {
        return;
    }
    if PERF_ONLY_ALLOWED.contains(&check.name) {
        push_perf_scope_finding(check);
        return;
    }
    let FeatureCheck { file, feature_line, name, rules, report_line, report, .. } = check;
    report.push(Finding::new(
        rules.disallowed.clone(),
        file.to_owned(),
        report_line,
        format!("disallowed unstable feature `{name}` (line {feature_line})"),
    ));
}

fn push_perf_scope_finding(check: FeatureCheck<'_>) {
    let FeatureCheck { file, feature_line, name, scope, rules, report_line, report } = check;
    if scope != FeatureScope::Normal {
        return;
    }
    report.push(Finding::new(
        rules.perf_scope.clone(),
        file.to_owned(),
        report_line,
        format!("perf-only unstable feature `{name}` outside approved scope (line {feature_line})"),
    ));
}

fn failure_after_stderr_line(args: std::fmt::Arguments<'_>) -> std::process::ExitCode {
    exit_after_io(write_stderr_line(args), LaneExit::Failure)
}

fn exit_after_io(result: std::io::Result<()>, success: LaneExit) -> std::process::ExitCode {
    match result {
        Ok(()) => exit(success),
        Err(_error) => exit(LaneExit::Failure),
    }
}

/// Write one formatted line to stderr.
///
/// # Errors
///
/// Returns the underlying stderr write error.
fn write_stderr_line(args: std::fmt::Arguments<'_>) -> std::io::Result<()> {
    use std::io::Write as _;

    let mut stderr = std::io::stderr().lock();
    stderr.write_fmt(args)?;
    stderr.write_all(b"\n")
}

/// Write raw formatted text to stderr.
///
/// # Errors
///
/// Returns the underlying stderr write error.
fn write_stderr_raw(args: std::fmt::Arguments<'_>) -> std::io::Result<()> {
    use std::io::Write as _;

    let mut stderr = std::io::stderr().lock();
    stderr.write_fmt(args)
}

#[cfg(test)]
#[path = "check_nightly_features/tests.rs"]
mod tests;
