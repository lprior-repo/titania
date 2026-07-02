use std::{
    collections::BTreeSet,
    fs,
    io::{self, ErrorKind, Write},
    path::PathBuf,
    process::ExitCode,
};

use titania_core::TargetProject;
use titania_lanes::{Finding, LaneExit, LaneReport, RuleId, current_target_project, exit};

const SRC: TargetRelativePath =
    TargetRelativePath::new("crates/vb_core/src/proof_kernels/step_state.rs");
const RULE_STEPSTATE: &str = "STEPSTATE_MATRIX";

type StateSet = BTreeSet<String>;

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

struct StepStateFacts {
    variants: StateSet,
    transitions: StateSet,
    is_terminal: StateSet,
    terminal_states: StateSet,
    non_terminal_fn: StateSet,
}

impl StepStateFacts {
    fn parse(text: &str) -> Self {
        let variants = extract_enum_variants(text, "StepState");
        let transitions = extract_block_after(text, "const VALID_TRANSITIONS", "];")
            .map_or_else(StateSet::new, |block| collect_stepstate_refs(&block));
        let is_terminal = find_function_body(text, "is_terminal")
            .map_or_else(StateSet::new, |block| collect_stepstate_refs(&block));
        let terminal_states = find_function_body(text, "terminal_states")
            .map_or_else(StateSet::new, |block| collect_stepstate_refs(&block));
        let non_terminal_fn = find_function_body(text, "non_terminal_states")
            .map_or_else(StateSet::new, |block| collect_stepstate_refs(&block));
        Self { variants, transitions, is_terminal, terminal_states, non_terminal_fn }
    }

    fn non_terminal_derived(&self) -> StateSet {
        self.variants.iter().filter(|v| !self.is_terminal.contains(*v)).cloned().collect()
    }
}

fn main_exit() -> ExitCode {
    let target = match current_target_project() {
        Ok(target) => target,
        Err(error) => {
            return exit_after_stderr_line(
                format_args!("[check-stepstate-matrix] target discovery failed: {error}"),
                LaneExit::Usage,
            );
        }
    };
    let rule = match RuleId::new(RULE_STEPSTATE) {
        Ok(rule) => rule,
        Err(error) => {
            return exit_after_stderr_line(
                format_args!("[check-stepstate-matrix] rule id configuration error: {error}"),
                LaneExit::Failure,
            );
        }
    };
    let mut report = LaneReport::new();
    run(&target, &rule, &mut report);
    print_and_exit(&report)
}

fn run(target: &TargetProject, rule: &RuleId, report: &mut LaneReport) {
    let Some(text) = read_source(target, rule, report) else {
        return;
    };
    let facts = StepStateFacts::parse(&text);
    check_transition_coverage(&facts.variants, &facts.transitions, rule, report);
    check_terminal_consistency(&facts, rule, report);
    check_non_terminal_consistency(&facts, rule, report);
}

fn read_source(target: &TargetProject, rule: &RuleId, report: &mut LaneReport) -> Option<String> {
    let path = SRC.in_target(target);
    match fs::read_to_string(&path) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == ErrorKind::NotFound => {
            record_absent_source(rule, report, target);
            None
        }
        Err(error) => {
            report.push(Finding::new(
                rule.clone(),
                SRC.as_str(),
                0,
                format!("source not readable: {:?}", error.kind()),
            ));
            None
        }
    }
}

fn check_transition_coverage(
    variants: &StateSet,
    transitions: &StateSet,
    rule: &RuleId,
    report: &mut LaneReport,
) {
    variants.iter().filter(|v| !transitions.contains(*v)).for_each(|v| {
        report.push(Finding::new(
            rule.clone(),
            SRC.as_str(),
            0,
            format!("variant {v} missing from VALID_TRANSITIONS"),
        ));
    });
    transitions.iter().filter(|t| !variants.contains(*t)).for_each(|t| {
        report.push(Finding::new(
            rule.clone(),
            SRC.as_str(),
            0,
            format!("phantom state {t} in VALID_TRANSITIONS"),
        ));
    });
    if variants.len() != transitions.len() {
        report.push(Finding::new(
            rule.clone(),
            SRC.as_str(),
            0,
            transition_count_message(variants, transitions),
        ));
    }
}

fn transition_count_message(variants: &StateSet, transitions: &StateSet) -> String {
    format!("variant count ({}) != transition state count ({})", variants.len(), transitions.len())
}

fn check_terminal_consistency(facts: &StepStateFacts, rule: &RuleId, report: &mut LaneReport) {
    if facts.is_terminal != facts.terminal_states {
        report.push(Finding::new(
            rule.clone(),
            SRC.as_str(),
            0,
            format!(
                "is_terminal/terminal_states inconsistent: {:?} vs {:?}",
                facts.is_terminal, facts.terminal_states
            ),
        ));
    }
}

fn check_non_terminal_consistency(facts: &StepStateFacts, rule: &RuleId, report: &mut LaneReport) {
    let non_terminal_derived = facts.non_terminal_derived();
    if non_terminal_derived != facts.non_terminal_fn {
        report.push(Finding::new(
            rule.clone(),
            SRC.as_str(),
            0,
            format!(
                "non_terminal_states inconsistent: {:?} vs {:?}",
                non_terminal_derived, facts.non_terminal_fn
            ),
        ));
    }
}

fn print_and_exit(report: &LaneReport) -> ExitCode {
    let rendered = report.render();
    if !rendered.is_empty() && write_stderr(format_args!("{rendered}")).is_err() {
        return exit(LaneExit::Failure);
    }
    if report.is_clean() { exit(LaneExit::Clean) } else { exit(LaneExit::Violations) }
}

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/check_stepstate_matrix/parser.rs"));

fn record_absent_source(rule: &RuleId, report: &mut LaneReport, target: &TargetProject) {
    if write_stderr_line(format_args!(
        "[check-stepstate-matrix] not applicable: {} is absent under {}; skipping StepState matrix lane",
        SRC.as_str(),
        target
    ))
    .is_err()
    {
        report.push(Finding::new(
            rule.clone(),
            SRC.as_str(),
            0,
            "stderr write failed while reporting StepState source absence",
        ));
    }
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

fn exit_after_stderr_line(args: std::fmt::Arguments<'_>, code: LaneExit) -> ExitCode {
    match write_stderr_line(args) {
        Ok(()) => exit(code),
        Err(_) => exit(LaneExit::Failure),
    }
}
