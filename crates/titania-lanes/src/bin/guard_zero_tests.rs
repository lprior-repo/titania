//! Fail-closed wrapper: refuses cargo test runs that executed zero applicable tests.
//!
//! Rust re-implementation of the bash lane in
//! `velvet-ballistics/scripts/guard-zero-tests.sh`. Run via
//! `cargo run --bin guard_zero_tests -- -- <cargo-test-args...>` from the
//! repository root or via the matching Moon task in `.moon/tasks/all.yml`.
//!
//! Exit codes: 0 = >0 applicable tests executed, 1 = zero tests or parse
//! failure, 2 = usage error.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use std::{
    env,
    io::{self, Write},
};

use titania_lanes::{CommandIn, CommandOutput, LaneExit, current_target_project, exit};

const USAGE: &str = "usage: guard_zero_tests [--] <cargo-test-args>\n  \
     exit 0: >0 applicable tests executed\n  \
     exit 1: zero applicable tests or parse failure\n  \
     exit 2: usage error";

fn main() -> std::process::ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return exit_after_stderr_line(format_args!("{USAGE}"), LaneExit::Clean);
    }
    match parse_lane_input(args) {
        LaneInput::MissingCommand => missing_command_exit(),
        LaneInput::Run(cmd_args) => run_command_exit(&cmd_args),
    }
}

fn missing_command_exit() -> std::process::ExitCode {
    match write_stderr_line(format_args!("guard-zero-tests: no command supplied")) {
        Ok(()) => exit_after_stderr_line(format_args!("{USAGE}"), LaneExit::Usage),
        Err(_) => exit(LaneExit::Failure),
    }
}

fn run_command_exit(cmd_args: &[String]) -> std::process::ExitCode {
    let target = match current_target_project() {
        Ok(target) => target,
        Err(err) => {
            return exit_after_stderr_line(
                format_args!("[guard-zero-tests] cannot resolve target project: {err}"),
                LaneExit::Usage,
            );
        }
    };
    lane_result_exit(run_lane(&target, cmd_args))
}

fn lane_result_exit(result: Result<(), String>) -> std::process::ExitCode {
    match result {
        Ok(()) => exit(LaneExit::Clean),
        Err(err) => {
            exit_after_stderr_line(format_args!("[guard-zero-tests] {err}"), LaneExit::Violations)
        }
    }
}

enum LaneInput {
    Run(Vec<String>),
    MissingCommand,
}
enum TestEvidence {
    Applicable(u32),
    NotApplicable,
}

type ParsedCommand<'a> = (&'a str, &'a [String]);
type LineCountParser = fn(&str) -> Option<u32>;

fn parse_lane_input(args: Vec<String>) -> LaneInput {
    let cmd: Vec<String> = match args.iter().position(|arg| arg == "--") {
        Some(separator) => args.into_iter().skip(separator.saturating_add(1)).collect(),
        None => args,
    };
    if cmd.is_empty() { LaneInput::MissingCommand } else { LaneInput::Run(cmd) }
}

/// # Errors
///
/// Returns an error when the command cannot be parsed, fails to run, fails, or
/// reports no applicable tests.
fn run_lane(target: &titania_core::TargetProject, cmd_args: &[String]) -> Result<(), String> {
    write_stderr_line(format_args!("[guard-zero-tests] running: {}", cmd_args.join(" ")))
        .map_err(|err| stderr_error(&err))?;
    let (program, passthrough) = parse_command_args(cmd_args)?;
    let output = run_test_command(target, program, passthrough)?;
    let combined = combine_output(output.stdout(), output.stderr());
    reject_failed_command(&output, &combined)?;
    report_test_evidence(parse_test_count(&combined)?)
}

/// # Errors
///
/// Returns an error when no program argument is present.
fn parse_command_args(cmd_args: &[String]) -> Result<ParsedCommand<'_>, String> {
    cmd_args
        .split_first()
        .map(|(program, passthrough)| (program.as_str(), passthrough))
        .ok_or_else(|| "guard-zero-tests: empty command".to_string())
}

/// # Errors
///
/// Returns an error when the command cannot be prepared or spawned.
fn run_test_command<'a>(
    target: &'a titania_core::TargetProject,
    program: &'a str,
    passthrough: &'a [String],
) -> Result<CommandOutput, String> {
    let mut command =
        CommandIn::new(target, program).map_err(|e| format!("failed to spawn {program}: {e}"))?;
    let _ = command.inherit_env();
    let _ = command.args_strings(passthrough);
    command.run_capture_raw().map_err(|e| format!("failed to spawn {program}: {e}"))
}

/// # Errors
///
/// Returns an error when the captured command status was non-zero or signaled.
fn reject_failed_command(output: &CommandOutput, combined: &str) -> Result<(), String> {
    match output.status().code() {
        Some(0) => Ok(()),
        Some(code) => reject_nonzero_command(code, combined),
        None => reject_signaled_command(combined),
    }
}

/// # Errors
///
/// Returns an error after reporting a non-zero command status.
fn reject_nonzero_command(code: i32, combined: &str) -> Result<(), String> {
    write_stderr_line(format_args!(
        "[guard-zero-tests] cargo test exited {code} - treating as tooling failure"
    ))
    .map_err(|err| stderr_error(&err))?;
    if let Some(n) = extract_applicable_count(combined) {
        write_stderr_line(format_args!(
            "[guard-zero-tests] applicable test count: {n} (cargo failed with exit {code})"
        ))
        .map_err(|err| stderr_error(&err))?;
    }
    write_stderr_line(format_args!("{combined}")).map_err(|err| stderr_error(&err))?;
    Err(format!("command exited with status {code}"))
}

/// # Errors
///
/// Returns an error after reporting a signaled command termination.
fn reject_signaled_command(combined: &str) -> Result<(), String> {
    write_stderr_line(format_args!("[guard-zero-tests] command terminated by signal"))
        .map_err(|err| stderr_error(&err))?;
    write_stderr_line(format_args!("{combined}")).map_err(|err| stderr_error(&err))?;
    Err("command terminated by signal".to_string())
}

/// # Errors
///
/// Returns an error when cargo output contains no recognized test count.
fn parse_test_count(combined: &str) -> Result<u32, String> {
    extract_applicable_count(combined).map_or_else(|| missing_test_count(combined), Ok)
}

/// # Errors
///
/// Returns an error when reporting fails or zero applicable tests ran.
fn report_test_evidence(count: u32) -> Result<(), String> {
    match classify_evidence(count) {
        TestEvidence::Applicable(count) => {
            write_stderr_line(format_args!(
                "[guard-zero-tests] PASS: {count} applicable tests executed"
            ))
            .map_err(|err| stderr_error(&err))?;
            Ok(())
        }
        TestEvidence::NotApplicable => {
            write_stderr_line(format_args!(
                "[guard-zero-tests] FAIL: command completed but executed zero applicable tests"
            ))
            .map_err(|err| stderr_error(&err))?;
            Err("zero applicable tests executed".to_string())
        }
    }
}

/// # Errors
///
/// Returns an error when writing the parse failure report fails.
fn missing_test_count(combined: &str) -> Result<u32, String> {
    write_stderr_line(format_args!(
        "[guard-zero-tests] FAIL: could not parse test count from cargo output."
    ))
    .map_err(|err| stderr_error(&err))?;
    write_stderr_line(format_args!("[guard-zero-tests] Raw output:\n{combined}"))
        .map_err(|err| stderr_error(&err))?;
    Err("could not parse test count from cargo output".to_string())
}

const fn classify_evidence(count: u32) -> TestEvidence {
    if count == 0 { TestEvidence::NotApplicable } else { TestEvidence::Applicable(count) }
}

fn combine_output(stdout: &[u8], stderr: &[u8]) -> String {
    let mut combined = String::new();
    if let Ok(s) = std::str::from_utf8(stdout) {
        combined.push_str(s);
    }
    if let Ok(s) = std::str::from_utf8(stderr) {
        combined.push_str(s);
    }
    combined
}

/// Try four patterns the bash script handled, in order, returning a summed
/// non-negative `u32` from the first pattern family present.
fn extract_applicable_count(text: &str) -> Option<u32> {
    extract_running_n(text)
        .or_else(|| extract_libtest_passed(text))
        .or_else(|| extract_cargo_test_passed(text))
        .or_else(|| extract_cargo_test_filtered(text))
}

/// Format 1: lines that look like `running 5 tests` / `running 0 tests`.
fn extract_running_n(text: &str) -> Option<u32> {
    sum_line_counts(text, running_line_count)
}

fn running_line_count(line: &str) -> Option<u32> {
    let trimmed = line.trim_start();
    if !(trimmed.starts_with("running ")
        && (trimmed.contains(" test") || trimmed.starts_with("running 0")))
    {
        return None;
    }
    let after = trimmed.strip_prefix("running ")?;
    let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// Format 2: `test result: ok. 5 passed; 0 failed; ...`.
fn extract_libtest_passed(text: &str) -> Option<u32> {
    sum_line_counts(text, libtest_passed_count)
}

fn libtest_passed_count(line: &str) -> Option<u32> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("test result:") {
        return None;
    }
    let after_passed = trimmed.split("passed").next()?;
    let token = after_passed.split(|c: char| !c.is_ascii_digit()).rev().find(|t| !t.is_empty())?;
    token.parse().ok()
}

/// Format 3: `cargo test: 5 passed (1 suite, 0.08s)`.
fn extract_cargo_test_passed(text: &str) -> Option<u32> {
    sum_line_counts(text, cargo_test_passed_count)
}

fn cargo_test_passed_count(line: &str) -> Option<u32> {
    let trimmed = line.trim_start();
    if !(trimmed.starts_with("cargo test:") && trimmed.contains("passed")) {
        return None;
    }
    let after = trimmed.strip_prefix("cargo test:")?;
    let token = after.split(|c: char| !c.is_ascii_digit()).find(|t| !t.is_empty())?;
    token.parse().ok()
}

/// Format 4 is covered by [`cargo_test_passed_count`].
fn extract_cargo_test_filtered(text: &str) -> Option<u32> {
    sum_line_counts(text, cargo_test_filtered_count)
}

fn cargo_test_filtered_count(line: &str) -> Option<u32> {
    let trimmed = line.trim_start();
    if is_cargo_test_filtered_line(trimmed) { cargo_test_passed_count(trimmed) } else { None }
}

fn is_cargo_test_filtered_line(trimmed: &str) -> bool {
    trimmed.starts_with("cargo test:")
        && trimmed.contains("passed")
        && trimmed.contains("filtered out")
}

fn sum_line_counts(text: &str, parse: LineCountParser) -> Option<u32> {
    let (seen, total) = text
        .lines()
        .filter_map(parse)
        .fold((false, 0_u32), |(_, total), count| (true, total.saturating_add(count)));
    if seen { Some(total) } else { None }
}

fn exit_after_stderr_line(args: std::fmt::Arguments<'_>, code: LaneExit) -> std::process::ExitCode {
    match write_stderr_line(args) {
        Ok(()) => exit(code),
        Err(_) => exit(LaneExit::Failure),
    }
}

/// # Errors
///
/// Returns an error when stderr cannot be written.
fn write_stderr_line(args: std::fmt::Arguments<'_>) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    stderr.write_fmt(args)?;
    stderr.write_all(b"\n")
}

fn stderr_error(err: &io::Error) -> String {
    format!("failed to write stderr: {err}")
}
