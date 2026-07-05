mod scan;
mod self_test;
#[cfg(test)]
mod tests;
mod vcs;

use std::{
    env,
    io::{self, Write as _},
};

use titania_core::TargetProject;
use titania_lanes::{
    Finding, LaneExit, LaneReport, RuleId, RuleIdError, current_target_project, exit,
};

const RULE_TEST_INTEGRITY: &str = "TEST_INTEGRITY_001";
const TEST_INTEGRITY_DEL_RULE: &str = "TEST_INTEGRITY_DEL_001";
const TEST_INTEGRITY_IGNORE_RULE: &str = "TEST_INTEGRITY_IGNORE_001";
const TEST_INTEGRITY_COMPILE_RULE: &str = "TEST_INTEGRITY_COMPILE_001";
const TEST_INTEGRITY_DECL_RULE: &str = "TEST_INTEGRITY_DECL_001";
const TEST_INTEGRITY_WEAK_RULE: &str = "TEST_INTEGRITY_WEAK_001";

type IntegrityFinding = (String, String, String);
type TestDeclaration = (String, String);
type ChangedFile = (String, String);
type ChangedFiles = Vec<ChangedFile>;

#[derive(Debug)]
pub(super) struct TestIntegrityError(String);

impl std::fmt::Display for TestIntegrityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for TestIntegrityError {}

impl From<String> for TestIntegrityError {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&'static str> for TestIntegrityError {
    fn from(value: &'static str) -> Self {
        Self(value.to_owned())
    }
}

struct TestIntegrityRules {
    test_integrity: RuleId,
    del: RuleId,
    ignore: RuleId,
    compile: RuleId,
    decl: RuleId,
    weak: RuleId,
}

impl TestIntegrityRules {
    /// Build rule identifiers for test-integrity findings.
    ///
    /// # Errors
    ///
    /// Returns the invalid rule-id error if one of the configured rule ids
    /// violates the shared rule-id format.
    fn new() -> Result<Self, RuleIdError> {
        Ok(Self {
            test_integrity: RuleId::new(RULE_TEST_INTEGRITY)?,
            del: RuleId::new(TEST_INTEGRITY_DEL_RULE)?,
            ignore: RuleId::new(TEST_INTEGRITY_IGNORE_RULE)?,
            compile: RuleId::new(TEST_INTEGRITY_COMPILE_RULE)?,
            decl: RuleId::new(TEST_INTEGRITY_DECL_RULE)?,
            weak: RuleId::new(TEST_INTEGRITY_WEAK_RULE)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Vcs {
    Git,
    Jj,
}

#[derive(Debug, Clone, Copy)]
struct RootInfo {
    vcs: Vcs,
}

/// Run the test-integrity lane and return its process exit code.
#[must_use]
pub fn main_exit() -> std::process::ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        return exit_after_stderr(
            &write_stderr_line(format_args!(
                "usage: check-test-integrity [--self-test] [--base <rev>]\n\
                 Validates that changes since <rev> do not delete tests, weaken\n\
                 assertions, or add #[ignore] / compile-only replacements."
            )),
            LaneExit::Usage,
        );
    }
    if args.iter().any(|arg| arg == "--self-test") {
        return exit(self_test::run());
    }
    let rules = match TestIntegrityRules::new() {
        Ok(rules) => rules,
        Err(error) => {
            return exit_after_stderr(
                &write_stderr_line(format_args!(
                    "[check-test-integrity] rule id configuration error: {error}"
                )),
                LaneExit::Failure,
            );
        }
    };
    exit(run_for_args(&args, &rules))
}

/// Write formatted text to stderr.
///
/// # Errors
///
/// Returns the underlying stderr write error.
fn write_stderr(args: std::fmt::Arguments<'_>) -> io::Result<()> {
    io::stderr().lock().write_fmt(args)
}

/// Write one formatted line to stderr.
///
/// # Errors
///
/// Returns the underlying stderr write error.
fn write_stderr_line(args: std::fmt::Arguments<'_>) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    stderr.write_fmt(args)?;
    stderr.write_all(b"\n")
}

fn exit_after_stderr(result: &io::Result<()>, success: LaneExit) -> std::process::ExitCode {
    exit(lane_after_stderr(result, success))
}

const fn lane_after_stderr(result: &io::Result<()>, success: LaneExit) -> LaneExit {
    match result {
        Ok(()) => success,
        Err(_) => LaneExit::Failure,
    }
}

fn run_for_args(args: &[String], rules: &TestIntegrityRules) -> LaneExit {
    let target = match resolve_target_project() {
        Ok(target) => target,
        Err(code) => return code,
    };
    let root = match resolve_vcs_root(&target) {
        Ok(info) => info,
        Err(code) => return code,
    };
    let base = base_argument_or_default(args, &target, root.vcs);
    check_result_to_exit(check(&target, &base, root.vcs, rules))
}

/// Resolve the target project used by the lane.
///
/// # Errors
///
/// Returns the mapped lane exit code when target discovery fails.
fn resolve_target_project() -> Result<TargetProject, LaneExit> {
    current_target_project().map_err(|error| {
        lane_after_stderr(
            &write_stderr_line(format_args!(
                "test integrity: ERROR cannot resolve target project: {error}"
            )),
            LaneExit::Usage,
        )
    })
}

/// Resolve the repository VCS root metadata.
///
/// # Errors
///
/// Returns the mapped lane exit code when neither supported VCS can resolve a root.
fn resolve_vcs_root(target: &TargetProject) -> Result<RootInfo, LaneExit> {
    vcs::root_dir(target).map_err(|error| {
        lane_after_stderr(
            &write_stderr_line(format_args!("test integrity: ERROR {error}")),
            LaneExit::Failure,
        )
    })
}

fn check_result_to_exit(result: Result<i32, TestIntegrityError>) -> LaneExit {
    match result {
        Ok(0) => LaneExit::Clean,
        Ok(_) => LaneExit::Violations,
        Err(error) => lane_after_stderr(
            &write_stderr_line(format_args!("test integrity: ERROR {error}")),
            LaneExit::Usage,
        ),
    }
}

/// Check changed tests against the selected base revision.
///
/// # Errors
///
/// Returns VCS, diff loading, base validation, or stderr rendering errors.
fn check(
    target: &TargetProject,
    base: &str,
    vcs: Vcs,
    rules: &TestIntegrityRules,
) -> Result<i32, TestIntegrityError> {
    vcs::validate_base_revision(target, base, vcs)?;
    let mut findings = deleted_file_findings(&vcs::changed_files(target, base, vcs)?);
    findings.extend(scan::scan_diff(&vcs::diff_text(target, base, vcs)?));
    if findings.is_empty() {
        write_stderr_line(format_args!("test integrity: PASS base={base}"))
            .map_err(|error| format!("stderr write failed: {error}"))?;
        Ok(0)
    } else {
        render_findings(&findings, rules)
            .map_err(|error| format!("stderr write failed: {error}"))?;
        Ok(1)
    }
}

fn deleted_file_findings(entries: &[ChangedFile]) -> Vec<IntegrityFinding> {
    entries
        .iter()
        .filter(|(status, path)| status.starts_with('D') && scan::is_test_path(path))
        .map(|(_status, path)| {
            (
                "DeletedTestFile".to_owned(),
                path.clone(),
                "deleted file contained tests or test assertions".to_owned(),
            )
        })
        .collect()
}

/// Render integrity findings to stderr.
///
/// # Errors
///
/// Returns the first stderr write error.
fn render_findings(findings: &[IntegrityFinding], rules: &TestIntegrityRules) -> io::Result<()> {
    write_stderr_line(format_args!("test integrity: FAIL"))?;
    let report = findings.iter().fold(LaneReport::new(), |mut report, (kind, path, detail)| {
        push_finding(&mut report, kind, path, detail, rules);
        report
    });
    write_stderr(format_args!("{}", report.render()))?;
    write_stderr_line(format_args!(
        "Add equal-or-stronger replacement coverage or bead-linked justification."
    ))
}

fn push_finding(
    report: &mut LaneReport,
    kind: &str,
    path: &str,
    detail: &str,
    rules: &TestIntegrityRules,
) {
    let rule = match kind {
        "DeletedTestFile" => &rules.del,
        "IgnoredOrSkippedTest" => &rules.ignore,
        "CompileOnlyReplacement" => &rules.compile,
        "DeletedTestDeclaration" => &rules.decl,
        "WeakenedAssertion" => &rules.weak,
        _ => &rules.test_integrity,
    };
    report.push(Finding::new(rule.clone(), path.to_owned(), 0, format!("{kind}: {detail}")));
}

fn argument_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2).find_map(|window| {
        let first = window.first()?;
        let second = window.get(1)?;
        (first == flag).then(|| second.clone())
    })
}

fn base_argument_or_default(args: &[String], target: &TargetProject, vcs: Vcs) -> String {
    let Some(base) = argument_value(args, "--base") else {
        return vcs::default_base(target, vcs);
    };
    base
}
