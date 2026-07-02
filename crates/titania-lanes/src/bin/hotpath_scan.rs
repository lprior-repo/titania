//! Rejects HashMap/IndexMap/mpsc tokens on hot paths outside allowlist.
//!
//! Rust re-implementation of the bash lane in
//! `velvet-ballistics/scripts/hotpath-scan.sh`. Run via
//! `cargo run --bin hotpath_scan --` from the repository root or via the
//! matching Moon task in `.moon/tasks/all.yml`.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

#[path = "hotpath_scan/allow.rs"]
/// Hotpath allowlist parsing and validation.
pub mod allow;
#[path = "hotpath_scan/scan.rs"]
/// Recursive hotpath token scanner.
pub mod scan;

use std::io::Write as _;

use titania_core::TargetProject;
use titania_lanes::{LaneExit, LaneReport, RuleId, RuleIdError, current_target_project, exit};

const HOT_ROOTS: &[&str] =
    &["crates/vb_core/src", "crates/vb_runtime/src", "crates/vb_storage/src", "crates/vb_ipc/src"];
const ALLOW_RULE: &str = "ALLOW_ROW";
const HOTPATH_RULE: &str = "HOTPATH_TOKEN";

/// Typed rule identifiers used by hotpath findings.
#[derive(Debug)]
pub struct HotpathRules {
    allow: RuleId,
    hotpath: RuleId,
}

impl HotpathRules {
    /// Build typed rule identifiers for hotpath findings.
    ///
    /// # Errors
    ///
    /// Returns the invalid rule-id error if a configured rule identifier does
    /// not satisfy the shared rule-id grammar.
    fn new() -> Result<Self, RuleIdError> {
        Ok(Self { allow: RuleId::new(ALLOW_RULE)?, hotpath: RuleId::new(HOTPATH_RULE)? })
    }
}

const TOKENS: &[&str] = &[
    "HashMap",
    "IndexMap",
    "IndexSet",
    "BTreeMap",
    "std::sync::mpsc",
    "mpsc::channel",
    "channel(",
];
const COLD_TOKENS: &[&str] = &[
    "diagnostic",
    "diagnostics",
    "fixture",
    "fixtures",
    "harness",
    "kani",
    "loom",
    "proof",
    "property",
    "proptest",
    "proptests",
    "support",
    "test",
    "tests",
    "verification",
];
const ALLOW_FILE: &str = "scripts/hotpath-scan.allow";

fn main() -> std::process::ExitCode {
    let target = match current_target_project() {
        Ok(target) => target,
        Err(error) => {
            return failure_after_stderr_line(format_args!(
                "[hotpath-scan] target discovery failed: {error}"
            ));
        }
    };
    let rules = match HotpathRules::new() {
        Ok(rules) => rules,
        Err(error) => {
            return failure_after_stderr_line(format_args!(
                "[hotpath-scan] rule id configuration error: {error}"
            ));
        }
    };
    let mut report = LaneReport::new();
    run(&target, &rules, &mut report);
    print_and_exit(&report)
}

fn failure_after_stderr_line(args: std::fmt::Arguments<'_>) -> std::process::ExitCode {
    match write_stderr_line(args) {
        Ok(()) | Err(_) => exit(LaneExit::Failure),
    }
}

fn run(target: &TargetProject, rules: &HotpathRules, report: &mut LaneReport) {
    let root = target.as_std_path();
    let allow = allow::load_allow(root, rules, report);
    for dir in HOT_ROOTS.iter().map(|hot| root.join(hot)).filter(|dir| dir.is_dir()) {
        scan::scan_dir(&dir, root, &allow, rules, report);
    }
}

fn print_and_exit(report: &LaneReport) -> std::process::ExitCode {
    let rendered = report.render();
    if !rendered.is_empty() && write_stderr(&rendered).is_err() {
        return exit(LaneExit::Failure);
    }
    if report.is_clean() { exit(LaneExit::Clean) } else { exit(LaneExit::Violations) }
}

/// Writes raw text to stderr.
///
/// # Errors
///
/// Returns the stderr write error when output fails.
fn write_stderr(text: &str) -> std::io::Result<()> {
    let mut stderr = std::io::stderr().lock();
    stderr.write_all(text.as_bytes())
}

/// Writes a diagnostic line to stderr.
///
/// # Errors
///
/// Returns the stderr write error when formatting or newline output fails.
fn write_stderr_line(args: std::fmt::Arguments<'_>) -> std::io::Result<()> {
    let mut stderr = std::io::stderr().lock();
    stderr.write_fmt(args)?;
    stderr.write_all(b"\n")
}
