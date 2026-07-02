use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use titania_lanes::{
    Finding, LaneExit, LaneReport, RuleId, RuleIdError, current_target_project, exit,
    helpers::relative_path,
};

const VERIFICATION_DIR: &str = "verification/verus";
const FIXTURE_SMOKE_MARKER: &str = "titania-verus-binding: fixture-smoke";
const FORMAL_SETUP_SMOKE_FILE: &str = "verification/verus/formal_setup_smoke.rs";
const BINDING_RULE: &str = "VERUS_BINDING";
const VACUUM_RULE: &str = "VERUS_VACUUM";
const SCAN_ERROR_RULE: &str = "SCAN_ERROR";

struct VerusBindingRules {
    binding: RuleId,
    vacuum: RuleId,
    scan_error: RuleId,
}

impl VerusBindingRules {
    /// Build the rule identifiers used by this lane.
    ///
    /// # Errors
    ///
    /// Returns [`RuleIdError`] when any static rule id fails validation.
    fn new() -> Result<Self, RuleIdError> {
        Ok(Self {
            binding: RuleId::new(BINDING_RULE)?,
            vacuum: RuleId::new(VACUUM_RULE)?,
            scan_error: RuleId::new(SCAN_ERROR_RULE)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Binding {
    Strong,
    Weak,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ProofScan {
    Binding(Binding),
    NotApplicable(NotApplicableReason),
    Vacuum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NotApplicableReason {
    FixtureSmoke,
    NoVerusDirectory,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct BindingSummary {
    strong: u32,
    weak: u32,
    not_applicable: u32,
    vacuum: u32,
}

impl BindingSummary {
    const fn record_binding(&mut self, binding: &Binding) {
        match binding {
            Binding::Strong => self.strong = self.strong.saturating_add(1),
            Binding::Weak => self.weak = self.weak.saturating_add(1),
        }
    }

    const fn record_not_applicable(&mut self, _reason: &NotApplicableReason) {
        self.not_applicable = self.not_applicable.saturating_add(1);
    }

    const fn record_vacuum(&mut self) {
        self.vacuum = self.vacuum.saturating_add(1);
    }
}

fn main_exit() -> ExitCode {
    let target = match current_target_project() {
        Ok(target) => target,
        Err(error) => {
            return exit_after_stderr_line(
                format_args!("[check-verus-production-binding] target discovery failed: {error}"),
                LaneExit::Usage,
            );
        }
    };
    let rules = match VerusBindingRules::new() {
        Ok(rules) => rules,
        Err(error) => {
            return exit_after_stderr_line(
                format_args!(
                    "[check-verus-production-binding] rule id configuration error: {error}"
                ),
                LaneExit::Failure,
            );
        }
    };
    let mut report = LaneReport::new();
    let summary = run(target.as_std_path(), &rules, &mut report);
    if print_summary(&summary).is_err() {
        return exit(LaneExit::Failure);
    }
    print_and_exit(&report)
}

fn run(root: &Path, rules: &VerusBindingRules, report: &mut LaneReport) -> BindingSummary {
    let mut summary = BindingSummary::default();
    let files = candidate_proof_files(root, rules, report, &mut summary);
    let scans: Vec<(String, ProofScan)> =
        files.iter().filter_map(|path| scan_candidate(root, path, rules, report)).collect();
    report.extend_finding(scans.iter().filter_map(|(rel, scan)| finding_for(rel, scan, rules)));
    scans.iter().fold(summary, |mut summary, (_, scan)| {
        record_summary(scan, &mut summary);
        summary
    })
}

fn candidate_proof_files(
    root: &Path,
    rules: &VerusBindingRules,
    report: &mut LaneReport,
    summary: &mut BindingSummary,
) -> Vec<PathBuf> {
    let dir = root.join(VERIFICATION_DIR);
    let read = match std::fs::read_dir(&dir) {
        Ok(read) => read,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            summary.record_not_applicable(&NotApplicableReason::NoVerusDirectory);
            return Vec::new();
        }
        Err(error) => {
            report.push(Finding::new(
                rules.scan_error.clone(),
                VERIFICATION_DIR,
                0,
                format!("cannot read verification dir: {error}"),
            ));
            return Vec::new();
        }
    };
    read.filter_map(|entry| entry_path(entry, rules, report))
        .filter(|path| is_candidate_path(root, path))
        .collect()
}

fn entry_path(
    entry: std::io::Result<std::fs::DirEntry>,
    rules: &VerusBindingRules,
    report: &mut LaneReport,
) -> Option<PathBuf> {
    match entry {
        Ok(entry) => Some(entry.path()),
        Err(error) => {
            report.push(Finding::new(
                rules.scan_error.clone(),
                VERIFICATION_DIR,
                0,
                format!("cannot read verification entry: {error}"),
            ));
            None
        }
    }
}

fn is_candidate_path(root: &Path, path: &Path) -> bool {
    path.is_file()
        && path.extension().and_then(|e| e.to_str()) == Some("rs")
        && !is_skipped_rel(&relative_path(root, path))
}

fn is_skipped_rel(rel: &str) -> bool {
    rel.ends_with("extern_.rs") || rel.contains("extern_") || rel.contains("production_inner/")
}

fn scan_candidate(
    root: &Path,
    path: &Path,
    rules: &VerusBindingRules,
    report: &mut LaneReport,
) -> Option<(String, ProofScan)> {
    let rel = relative_path(root, path);
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => {
            report.push(Finding::new(
                rules.scan_error.clone(),
                rel,
                0,
                format!("cannot read proof file: {error}"),
            ));
            return None;
        }
    };
    has_proof_fn(&text).then(|| (rel.clone(), classify(&rel, &text)))
}

fn finding_for(rel: &str, scan: &ProofScan, rules: &VerusBindingRules) -> Option<Finding> {
    match scan {
        ProofScan::Binding(binding) => {
            Some(Finding::new(rules.binding.clone(), rel, 0, binding_message(binding)))
        }
        ProofScan::Vacuum => {
            Some(Finding::new(rules.vacuum.clone(), rel, 0, "VACUUM no production binding"))
        }
        ProofScan::NotApplicable(_) => None,
    }
}

const fn record_summary(scan: &ProofScan, summary: &mut BindingSummary) {
    match scan {
        ProofScan::Binding(binding) => summary.record_binding(binding),
        ProofScan::NotApplicable(reason) => summary.record_not_applicable(reason),
        ProofScan::Vacuum => summary.record_vacuum(),
    }
}

const fn binding_message(binding: &Binding) -> &'static str {
    match binding {
        Binding::Strong => "STRONG direct crates/ binding",
        Binding::Weak => "WEAK production_inner/ mirror",
    }
}

/// Write the classification counts to stderr.
///
/// # Errors
///
/// Returns an [`io::Error`] when stderr cannot be written.
fn print_summary(summary: &BindingSummary) -> io::Result<()> {
    write_stderr_line(format_args!(
        "STRONG: {}, WEAK: {}, NOT_APPLICABLE: {}, VACUUM: {}",
        summary.strong, summary.weak, summary.not_applicable, summary.vacuum
    ))
}

fn print_and_exit(report: &LaneReport) -> ExitCode {
    let rendered = report.render();
    if !rendered.is_empty() && write_stderr(format_args!("{rendered}")).is_err() {
        return exit(LaneExit::Failure);
    }
    let has_blocking = report
        .findings()
        .iter()
        .any(|f| f.rule().as_str() == VACUUM_RULE || f.rule().as_str() == SCAN_ERROR_RULE);
    if has_blocking { exit(LaneExit::Violations) } else { exit(LaneExit::Clean) }
}

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/bin/check_verus_production_binding/classification.rs"
));

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

fn exit_after_stderr_line(args: std::fmt::Arguments<'_>, code: LaneExit) -> ExitCode {
    match write_stderr_line(args) {
        Ok(()) => exit(code),
        Err(_) => exit(LaneExit::Failure),
    }
}
