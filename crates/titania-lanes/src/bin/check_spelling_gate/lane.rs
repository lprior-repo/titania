use std::{
    io::Write as _,
    path::{Path, PathBuf},
    process::ExitCode,
};

use titania_lanes::{Finding, LaneExit, LaneReport, RuleId, current_target_project, exit};

/// Candidate wrong-spelling token. A [`SpellingRule`] makes the lane
/// applicable only when this differs from the canonical spelling.
const WRONG_SPELLING: &str = "velvet-ballistics";
const CANONICAL_SPELLING: &str = "velvet-ballistics";
const SPELLING_RULE: &str = "SPELLING_GATE_001";

/// File extensions we scan (matches the bash `--include` list).
const SCAN_EXTENSIONS: &[&str] = &["rs", "toml", "yaml", "yml", "md", "sh", "py"];

const EXCLUDED_SUBSTRINGS: &[&str] = &[
    "/.beads/",
    "/.jj/",
    "/.evidence/",
    "/evidence/",
    "/target/",
    "/target_nosccache/",
    "/target_debug_clean/",
    "/target_clean/",
    "/tests/",
    "/benches/",
    "/naming_scan/",
    "/vb-",
    "/femdation-vb-",
    "/go-skill-",
    "/holzman-workspace-",
    "/pick5-",
];

pub(crate) fn main_exit() -> ExitCode {
    let rule = match spelling_rule() {
        Ok(rule) => rule,
        Err(code) => return code,
    };
    let target = match current_target_project() {
        Ok(target) => target,
        Err(error) => {
            return exit_after_stderr_line(
                format_args!("[check-spelling-gate] cannot resolve target project: {error}"),
                LaneExit::Usage,
            );
        }
    };
    run_for_root(target.as_std_path(), &rule)
}

/// Build the active spelling rule from lane constants.
///
/// # Errors
///
/// Returns an exit code when the constants describe an invalid or
/// not-applicable spelling rule.
fn spelling_rule() -> Result<SpellingRule<'static>, ExitCode> {
    SpellingRule::parse(WRONG_SPELLING, CANONICAL_SPELLING).map_err(|error| match error {
        SpellingRuleError::IdenticalTerms => not_applicable_rule_exit(),
        SpellingRuleError::EmptyTerm => invalid_rule_exit(),
    })
}

fn not_applicable_rule_exit() -> ExitCode {
    if write_stderr_line(format_args!(
        "NotApplicable: spelling rule has identical wrong/canonical terms"
    ))
    .is_err()
    {
        return exit(LaneExit::Failure);
    }
    exit(LaneExit::NotApplicable)
}

fn invalid_rule_exit() -> ExitCode {
    if write_stderr_line(format_args!("InvalidInvocation: spelling rule contains an empty term"))
        .is_err()
    {
        return exit(LaneExit::Failure);
    }
    exit(LaneExit::Usage)
}

fn run_for_root(root: &Path, rule: &SpellingRule<'_>) -> ExitCode {
    if write_stderr_line(format_args!("=== Spelling Gate: {} vs {} ===", rule.bad(), rule.good()))
        .is_err()
    {
        return exit(LaneExit::Failure);
    }
    let rule_id = match RuleId::new(SPELLING_RULE) {
        Ok(rule_id) => rule_id,
        Err(error) => {
            return exit_after_stderr_line(
                format_args!("[check-spelling-gate] rule id configuration error: {error}"),
                LaneExit::Failure,
            );
        }
    };
    let mut report = LaneReport::new();
    let findings: Vec<Finding> =
        collect_files(root).iter().flat_map(|file| scan_file(file, rule, &rule_id)).collect();
    report.extend_finding(findings);
    if write_stderr_line(format_args!(
        "=== Spelling Gate complete: {} violations ===",
        report.finding_count()
    ))
    .is_err()
    {
        return exit(LaneExit::Failure);
    }
    if write_stderr(&report.render()).is_err() {
        return exit(LaneExit::Failure);
    }
    if report.is_clean() { exit(LaneExit::Clean) } else { spelling_violations_exit(rule) }
}

fn spelling_violations_exit(rule: &SpellingRule<'_>) -> ExitCode {
    if write_spelling_violation_help(rule).is_err() {
        return exit(LaneExit::Failure);
    }
    exit(LaneExit::Violations)
}

/// Write remediation help for spelling violations.
///
/// # Errors
///
/// Returns the underlying stderr write error.
fn write_spelling_violation_help(rule: &SpellingRule<'_>) -> std::io::Result<()> {
    write_stderr_line(format_args!("Hint: The canonical spelling is '{}'.", rule.good()))?;
    write_stderr_line(format_args!("Allowlisted path patterns (excluded entirely):"))?;
    write_stderr_line(format_args!("  - .beads/ (bead artifacts and CI output)"))?;
    write_stderr_line(format_args!("  - .jj/ (JJ internal state)"))?;
    write_stderr_line(format_args!("  - target/ (build artifacts)"))?;
    write_stderr_line(format_args!("  - tests/ and benches/ (test/bench clippy is not strict)"))?;
    write_stderr_line(format_args!("  - velvet-ballistics-MASTER.md (master contract file)"))?;
    write_stderr_line(format_args!("Allowlisted content patterns:"))?;
    write_stderr_line(format_args!("  - velvet-ballistics-MASTER.md (reference to master file)"))?;
    write_stderr_line(format_args!("  - source checkout path migration artifacts"))?;
    write_stderr_line(format_args!("  - FORBIDDEN_FEATURE_NAMES (spelling used as forbid-tag)"))?;
    write_stderr_line(format_args!("  - explicit rule statements"))
}

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/check_spelling_gate/paths.rs"));

fn is_content_allowed(line: &str, rule: &SpellingRule<'_>) -> bool {
    let bad = rule.bad();
    line.contains("velvet-ballistics-MASTER.md")
        || (line.contains("/home/") && line.contains(bad))
        || line.contains("FORBIDDEN_FEATURE_NAMES")
        || line.contains("is invalid")
        || (line.contains("dolthub.com/") && line.contains(bad))
        || line.contains("velvet-ballistics/v2")
}

fn scan_file(file: &Path, rule: &SpellingRule<'_>, rule_id: &RuleId) -> Vec<Finding> {
    if is_path_excluded(file) {
        return Vec::new();
    }
    let Ok(content) = std::fs::read_to_string(file) else {
        return Vec::new();
    };
    let display = file.display().to_string();
    content
        .lines()
        .enumerate()
        .flat_map(|(idx, line)| scan_line(line, idx, &display, rule, rule_id))
        .collect()
}

fn scan_line(
    line: &str,
    idx: usize,
    display: &str,
    rule: &SpellingRule<'_>,
    rule_id: &RuleId,
) -> Vec<Finding> {
    if !line.contains(rule.bad()) || is_content_allowed(line, rule) {
        return Vec::new();
    }
    let line_no = u32::try_from(idx.saturating_add(1)).map_or(u32::MAX, core::convert::identity);
    vec![Finding::new(
        rule_id.clone(),
        display,
        line_no,
        format!("wrong spelling '{}' (use '{}')", rule.bad(), rule.good()),
    )]
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SpellingRule<'a> {
    bad: &'a str,
    good: &'a str,
}

impl<'a> SpellingRule<'a> {
    /// Parse and validate a wrong/canonical spelling pair.
    ///
    /// # Errors
    ///
    /// Returns [`SpellingRuleError::EmptyTerm`] when either term is blank and
    /// [`SpellingRuleError::IdenticalTerms`] when the terms are equal.
    fn parse(bad: &'a str, good: &'a str) -> Result<Self, SpellingRuleError> {
        spelling_rule_error(bad, good).map_or(Ok(Self { bad, good }), Err)
    }

    const fn bad(&self) -> &str {
        self.bad
    }

    const fn good(&self) -> &str {
        self.good
    }
}

fn spelling_rule_error(bad: &str, good: &str) -> Option<SpellingRuleError> {
    if bad.trim().is_empty() || good.trim().is_empty() {
        return Some(SpellingRuleError::EmptyTerm);
    }
    if bad == good {
        return Some(SpellingRuleError::IdenticalTerms);
    }
    None
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SpellingRuleError {
    EmptyTerm,
    IdenticalTerms,
}

/// Write raw text to stderr.
///
/// # Errors
///
/// Returns the underlying stderr write error.
fn write_stderr(text: &str) -> std::io::Result<()> {
    let mut stderr = std::io::stderr().lock();
    stderr.write_all(text.as_bytes())
}

/// Write formatted text followed by a newline to stderr.
///
/// # Errors
///
/// Returns the underlying stderr write error.
fn write_stderr_line(args: std::fmt::Arguments<'_>) -> std::io::Result<()> {
    let mut stderr = std::io::stderr().lock();
    stderr.write_fmt(args)?;
    stderr.write_all(b"\n")
}

fn exit_after_stderr_line(args: std::fmt::Arguments<'_>, code: LaneExit) -> ExitCode {
    match write_stderr_line(args) {
        Ok(()) => exit(code),
        Err(_) => exit(LaneExit::Failure),
    }
}
