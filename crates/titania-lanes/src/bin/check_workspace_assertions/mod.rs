mod checks;
mod toml_scan;

use std::{
    io::{self, Write as _},
    path::Path,
};

use titania_core::TargetProject;
use titania_lanes::{
    Finding, LaneExit, LaneReport, RuleId, RuleIdError, current_target_project, exit,
};
const RULE_INVALID_INVOCATION: &str = "WS_INVOCATION_001";
const RULE_MEMBERS: &str = "WS_MEMBERS_001";
const RULE_CRATE_NAME: &str = "WS_CRATE_NAME_001";
const RULE_FORBIDDEN_FEATURE: &str = "WS_FORBIDDEN_FEATURE_001";
const RULE_FORBIDDEN_DEP: &str = "WS_FORBIDDEN_DEP_001";
const RULE_GENERATED_BOUNDARY: &str = "WS_GENERATED_BOUNDARY_001";
const RULE_UNREADABLE: &str = "WS_UNREADABLE_001";

const FORBIDDEN_FEATURE_NAMES: &[&str] =
    &["json", "serde-json", "generated", "maxperf", "velvet-ballistics", "velvet_ballistics"];

struct WsRules {
    invalid_invocation: RuleId,
    members: RuleId,
    crate_name: RuleId,
    forbidden_feature: RuleId,
    forbidden_dep: RuleId,
    generated_boundary: RuleId,
    unreadable: RuleId,
}

impl WsRules {
    /// Build the workspace assertion rule identifiers.
    ///
    /// # Errors
    ///
    /// Returns [`RuleIdError`] when any hard-coded rule identifier is not a
    /// valid lane rule id.
    fn new() -> Result<Self, RuleIdError> {
        Ok(Self {
            invalid_invocation: RuleId::new(RULE_INVALID_INVOCATION)?,
            members: RuleId::new(RULE_MEMBERS)?,
            crate_name: RuleId::new(RULE_CRATE_NAME)?,
            forbidden_feature: RuleId::new(RULE_FORBIDDEN_FEATURE)?,
            forbidden_dep: RuleId::new(RULE_FORBIDDEN_DEP)?,
            generated_boundary: RuleId::new(RULE_GENERATED_BOUNDARY)?,
            unreadable: RuleId::new(RULE_UNREADABLE)?,
        })
    }
}

/// Run the workspace assertion lane and convert the lane result to a process
/// exit code.
#[must_use]
pub fn main_exit() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        return usage_exit();
    }
    let target = match target_project_or_exit() {
        Ok(target) => target,
        Err(code) => return code,
    };
    let rules = match rules_or_exit() {
        Ok(rules) => rules,
        Err(code) => return code,
    };
    exit(run(&target, &rules))
}

fn usage_exit() -> std::process::ExitCode {
    exit_after_stderr(
        &write_stderr_line(format_args!(
            "usage: check-workspace-assertions\n\
             Validates the Cargo workspace shape (members, package names,\n\
             forbidden features, generated-boundary files). The target\n\
             project is discovered by walking up from the process CWD."
        )),
        LaneExit::Usage,
    )
}

/// Resolve the target project or return the process exit code to use.
///
/// # Errors
///
/// Returns `Err(exit_code)` after writing a diagnostic when target discovery
/// fails.
fn target_project_or_exit() -> Result<TargetProject, std::process::ExitCode> {
    current_target_project().map_err(|error| {
        exit_after_stderr(
            &write_stderr_line(format_args!(
                "InvalidInvocation: cannot resolve target project: {error}"
            )),
            LaneExit::Usage,
        )
    })
}

/// Build workspace assertion rule identifiers or return the process exit code.
///
/// # Errors
///
/// Returns `Err(exit_code)` after writing a diagnostic when a rule id is
/// invalid.
fn rules_or_exit() -> Result<WsRules, std::process::ExitCode> {
    WsRules::new().map_err(|error| {
        exit_after_stderr(
            &write_stderr_line(format_args!(
                "[check-workspace-assertions] rule id configuration error: {error}"
            )),
            LaneExit::Failure,
        )
    })
}

/// Write formatted data to stderr without appending a newline.
///
/// # Errors
///
/// Returns the underlying [`io::Error`] when stderr cannot be written.
fn write_stderr(args: std::fmt::Arguments<'_>) -> io::Result<()> {
    io::stderr().lock().write_fmt(args)
}

/// Write formatted data to stderr and append a newline.
///
/// # Errors
///
/// Returns the underlying [`io::Error`] when either formatted data or the
/// trailing newline cannot be written to stderr.
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

fn run(target: &TargetProject, rules: &WsRules) -> LaneExit {
    let root = target.as_std_path();
    if !is_workspace_root(root) {
        return invalid_workspace_exit(rules);
    }

    let mut report = LaneReport::new();
    let members = checks::discover_members(root);
    checks::check_workspace_members(root, rules, &mut report);
    checks::check_crate_names(root, &members, rules, &mut report);
    checks::check_forbidden_dependencies(root, &members, rules, &mut report);
    checks::check_generated_boundaries(root, rules, &mut report);

    if write_stderr(format_args!("{}", report.render())).is_err() {
        return LaneExit::Failure;
    }
    if report.is_clean() {
        lane_after_stderr(
            &write_stderr_line(format_args!("workspace assertions: PASS")),
            LaneExit::Clean,
        )
    } else {
        LaneExit::Violations
    }
}

fn is_workspace_root(root: &Path) -> bool {
    root.join("Cargo.toml").exists() && root.join("crates").exists()
}

fn invalid_workspace_exit(rules: &WsRules) -> LaneExit {
    let mut report = LaneReport::new();
    report.push(Finding::new(
        rules.invalid_invocation.clone(),
        "Cargo.toml",
        0,
        "InvalidInvocation: target project is not a Cargo workspace root",
    ));
    if write_stderr(format_args!("{}", report.render())).is_err() {
        LaneExit::Failure
    } else {
        LaneExit::Usage
    }
}

#[cfg(test)]
mod tests {
    use super::FORBIDDEN_FEATURE_NAMES;

    #[test]
    fn forbidden_features_does_not_contain_cargo_or_unrelated() {
        assert!(!FORBIDDEN_FEATURE_NAMES.contains(&"serde"));
        assert!(FORBIDDEN_FEATURE_NAMES.contains(&"json"));
    }
}
