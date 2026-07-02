#![expect(
    clippy::redundant_pub_crate,
    reason = "lane entrypoint is called by the private flux_check_package wrapper module"
)]

use std::{
    env,
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
};

use titania_core::TargetProject;
use titania_lanes::{
    CommandIn, Finding, LaneExit, LaneReport, RuleId, RuleIdError, current_target_project, exit,
};

const RULE_REJECTED: &str = "FLUX_REJECTED_001";
const RULE_USAGE: &str = "FLUX_USAGE_001";
const RULE_FLUX_MISSING: &str = "FLUX_MISSING_001";

struct FluxRules {
    rejected: RuleId,
    usage: RuleId,
    missing: RuleId,
}

impl FluxRules {
    /// Construct the rule identifiers used by this lane.
    ///
    /// # Errors
    ///
    /// Returns [`RuleIdError`] when a configured rule identifier is invalid.
    fn new() -> Result<Self, RuleIdError> {
        Ok(Self {
            rejected: RuleId::new(RULE_REJECTED)?,
            usage: RuleId::new(RULE_USAGE)?,
            missing: RuleId::new(RULE_FLUX_MISSING)?,
        })
    }
}

const REJECTED_SELECTORS: &[&str] = &["--lib", "--test", "--tests", "--benches", "--all-targets"];

struct Invocation {
    package: String,
    forwarded: Vec<String>,
}

pub(super) fn main_exit(args: &[String]) -> ExitCode {
    if help_requested(args) {
        return print_help();
    }
    let rules = match FluxRules::new() {
        Ok(rules) => rules,
        Err(error) => {
            return exit_after_stderr_line(
                format_args!("[flux-check-package] rule id configuration error: {error}"),
                LaneExit::Failure,
            );
        }
    };
    let invocation = match parse_invocation(args, &rules) {
        Ok(invocation) => invocation,
        Err(code) => return code,
    };
    let target = match discover_target_project(&rules) {
        Ok(target) => target,
        Err(code) => return code,
    };
    run_flux(&target, &invocation, &rules)
}

fn help_requested(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--help" || arg == "-h")
}

fn print_help() -> ExitCode {
    if write_stderr_line(format_args!(
        "usage: flux-check-package <package> [cargo-flux options]\n\
         Rejects --lib, --test, --tests, --benches, --all-targets and\n\
         shells out to `cargo flux -p <package> --message-format human`."
    ))
    .is_err()
    {
        return exit(LaneExit::Failure);
    }
    exit(LaneExit::Usage)
}

fn exit_after_stderr_line(args: std::fmt::Arguments<'_>, code: LaneExit) -> ExitCode {
    match write_stderr_line(args) {
        Ok(()) => exit(code),
        Err(_) => exit(LaneExit::Failure),
    }
}

/// Parse the package name and forwarded cargo-flux arguments.
///
/// # Errors
///
/// Returns a rendered usage exit code when the package is missing or a
/// forwarded selector is unsupported by the installed cargo-flux path.
fn parse_invocation(args: &[String], rules: &FluxRules) -> Result<Invocation, ExitCode> {
    let Some((package, forwarded)) = args.split_first() else {
        return Err(usage_exit("usage: flux-check-package <package> [cargo-flux options]", rules));
    };
    reject_unsupported_selectors(forwarded, rules)?;
    Ok(Invocation { package: package.clone(), forwarded: forwarded.to_vec() })
}

/// Reject target selectors this wrapper intentionally does not support.
///
/// # Errors
///
/// Returns a rendered usage exit code when any rejected selector is present.
fn reject_unsupported_selectors(args: &[String], rules: &FluxRules) -> Result<(), ExitCode> {
    let mut report = LaneReport::new();
    args.iter().filter(|arg| is_rejected_selector(arg)).for_each(|arg| {
        report.push(Finding::new(
            rules.rejected.clone(),
            "argv",
            0,
            format!("unsupported cargo-flux target selector for installed cargo-flux: {arg}"),
        ));
    });
    if report.is_clean() { Ok(()) } else { Err(render_exit(&report, LaneExit::Usage)) }
}

fn is_rejected_selector(arg: &str) -> bool {
    REJECTED_SELECTORS.contains(&arg)
}

/// Resolve the current target project for command execution.
///
/// # Errors
///
/// Returns a rendered usage exit code when target discovery fails.
fn discover_target_project(rules: &FluxRules) -> Result<TargetProject, ExitCode> {
    current_target_project().map_err(|error| {
        let mut report = LaneReport::new();
        report.push(Finding::new(
            rules.usage.clone(),
            "target",
            0,
            format!("target discovery failed: {error}"),
        ));
        render_exit(&report, LaneExit::Usage)
    })
}

fn usage_exit(message: &str, rules: &FluxRules) -> ExitCode {
    let mut report = LaneReport::new();
    report.push(Finding::new(rules.usage.clone(), "argv", 0, message));
    render_exit(&report, LaneExit::Usage)
}

fn run_flux(target: &TargetProject, invocation: &Invocation, rules: &FluxRules) -> ExitCode {
    let cargo_args = build_cargo_args(invocation);
    let path = rustup_first_path();
    let mut command = match prepare_command(target, path.as_deref()) {
        Ok(command) => command,
        Err(error) => return cargo_missing_exit(&error, rules),
    };
    append_args(&mut command, &cargo_args);
    run_command(&command, rules)
}

fn build_cargo_args(invocation: &Invocation) -> Vec<String> {
    let mut args = vec![
        "flux".to_owned(),
        "-p".to_owned(),
        invocation.package.clone(),
        "--message-format".to_owned(),
        "human".to_owned(),
    ];
    args.extend(invocation.forwarded.iter().cloned());
    args
}

/// Prepare the `cargo` command in the target project.
///
/// # Errors
///
/// Returns a stringified command construction error when the target command
/// cannot be created.
fn prepare_command<'a>(
    target: &'a TargetProject,
    path: Option<&'a str>,
) -> Result<CommandIn<'a>, String> {
    let mut command = CommandIn::new(target, "cargo").map_err(|error| error.to_string())?;
    let _ = command.inherit_env();
    if let Some(path) = path {
        let _ = command.env("PATH", path);
    }
    Ok(command)
}

fn run_command(command: &CommandIn<'_>, rules: &FluxRules) -> ExitCode {
    match command.run_status_raw() {
        Ok(status) => exit(match status.code() {
            Some(0) => LaneExit::Clean,
            Some(1) => LaneExit::Violations,
            Some(2) => LaneExit::Usage,
            Some(_) | None => LaneExit::Failure,
        }),
        Err(error) => cargo_missing_exit(&error.to_string(), rules),
    }
}

fn cargo_missing_exit(error: &str, rules: &FluxRules) -> ExitCode {
    let mut report = LaneReport::new();
    report.push(Finding::new(
        rules.missing.clone(),
        "cargo",
        0,
        format!("cargo flux failed to start: {error}"),
    ));
    render_exit(&report, LaneExit::Failure)
}

fn render_exit(report: &LaneReport, code: LaneExit) -> ExitCode {
    match write_stderr(format_args!("{}", report.render())) {
        Ok(()) => exit(code),
        Err(_) => exit(LaneExit::Failure),
    }
}

fn append_args(command: &mut CommandIn<'_>, args: &[String]) {
    let _ = command.args_strings(args);
}

fn rustup_first_path() -> Option<String> {
    let current = env::var_os("PATH")?;
    let home = env::var_os("HOME")?;
    let cargo_bin = PathBuf::from(home).join(".cargo").join("bin");
    if !cargo_bin.is_dir() {
        return current.into_string().ok();
    }
    let paths = std::iter::once(cargo_bin.clone())
        .chain(env::split_paths(&current).filter(move |path| path != &cargo_bin));
    env::join_paths(paths).ok().and_then(|path| path.into_string().ok())
}

/// Write formatted text to stderr.
///
/// # Errors
///
/// Returns the underlying stderr write error.
fn write_stderr(args: std::fmt::Arguments<'_>) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    stderr.write_fmt(args)
}

/// Write formatted text followed by a newline to stderr.
///
/// # Errors
///
/// Returns the underlying stderr write error.
fn write_stderr_line(args: std::fmt::Arguments<'_>) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    stderr.write_fmt(args)?;
    stderr.write_all(b"\n")
}
