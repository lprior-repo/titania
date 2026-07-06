//! Library wrapper for Cargo-backed v1 lanes.
//!
//! The public `titania-check run-lane` dispatcher uses this module to execute
//! cargo-native lanes without depending on the compatibility `run-cargo` binary.

use super::run_cargo::{
    CargoLane, RunCargoError, args_for_lane, clean_outcome, findings_outcome, process_termination,
};
use crate::{LaneExit, LaneReport};
use titania_core::{Lane, LaneFailure, LaneOutcome, TargetProject, discover_target};

/// Cargo-backed lane dispatch result.
pub(super) enum CargoRun {
    Unsupported,
    Complete(LaneOutcome),
    Error { exit: LaneExit, message: String },
}

/// Run the Cargo-backed implementation for known Cargo lanes.
pub(super) fn run(lane: Lane) -> CargoRun {
    let Some(lane_name) = cargo_lane_name(lane) else {
        return CargoRun::Unsupported;
    };
    match run_checked(lane_name) {
        Ok(outcome) => CargoRun::Complete(outcome),
        Err(error) => CargoRun::Error { exit: error_exit(&error), message: error_message(error) },
    }
}

/// Map an outer [`Lane`] to its cargo sub-lane name.
const fn cargo_lane_name(lane: Lane) -> Option<&'static str> {
    match lane {
        Lane::Fmt => Some("fmt"),
        Lane::Compile => Some("compile"),
        Lane::Clippy => Some("clippy"),
        Lane::Test => Some("test"),
        Lane::Build => Some("build"),
        Lane::AstGrep | Lane::Dylint | Lane::PanicScan | Lane::PolicyScan | Lane::Deny => None,
    }
}

const fn error_exit(error: &RunCargoError) -> LaneExit {
    match error {
        RunCargoError::Usage(_) | RunCargoError::Target(_) => LaneExit::Usage,
        _other => LaneExit::Failure,
    }
}

fn error_message(error: RunCargoError) -> String {
    match error {
        RunCargoError::Usage(message) | RunCargoError::ToolVersion(message) => message,
        RunCargoError::Target(error) => format!("target discovery failed: {error}"),
        RunCargoError::Command(error) => format!("cargo execution failed: {error}"),
        RunCargoError::CurrentDir(error) => format!("cannot read current directory: {error}"),
        RunCargoError::RuleId(error) => format!("rule id configuration error: {error}"),
        RunCargoError::Outcome(error) => format!("lane outcome construction failed: {error}"),
    }
}

fn usage_message() -> String {
    String::from("usage: run-lane <fmt|compile|clippy|test|build>")
}

/// Resolve target state and run the selected Cargo lane.
///
/// # Errors
/// Returns [`RunCargoError`] when the sub-command is unrecognised, the CWD
/// cannot be read, no Cargo target project is discoverable, the rule ID is
/// invalid, or the lane execution fails.
fn run_checked(lane_name: &str) -> Result<LaneOutcome, RunCargoError> {
    let cargo_lane = CargoLane::parse(lane_name)
        .map_err(|_parse_error| RunCargoError::Usage(usage_message()))?;
    let cwd = std::env::current_dir().map_err(RunCargoError::CurrentDir)?;
    let target = discover_target(&cwd).map_err(RunCargoError::Target)?;
    let rule = crate::RuleId::new(cargo_lane.rule()).map_err(RunCargoError::RuleId)?;
    run_lane(&target, cargo_lane, &rule, &[])
}

/// Execute a single cargo lane and build its [`LaneOutcome`].
///
/// # Errors
/// Returns [`RunCargoError::Command`] on cargo execution failure, or any
/// outcome-construction error from [`lane_outcome`].
fn run_lane(
    target: &TargetProject,
    lane: CargoLane,
    rule: &crate::RuleId,
    extra_args: &[String],
) -> Result<LaneOutcome, RunCargoError> {
    let output = cargo_output(target, lane, extra_args).map_err(RunCargoError::Command)?;
    if lane == CargoLane::Clippy {
        return normalize_clippy_output(target, lane, extra_args, &output);
    }
    let mut report = LaneReport::new();
    report.record_scan();
    record_command_result(&output, lane, rule, &mut report);
    lane_outcome(target, lane, extra_args, &output, &report)
}

fn record_command_result(
    output: &crate::CommandOutput,
    lane: CargoLane,
    rule: &crate::RuleId,
    report: &mut LaneReport,
) {
    if output.success() && report.is_clean() {
        report.record_pass();
    }
    if !output.success() {
        report.push(crate::Finding::new(rule.clone(), lane.path(), 0, output_message(output)));
    }
}

fn output_message(output: &crate::CommandOutput) -> String {
    let stdout_result = output.stdout_str();
    let stderr_result = output.stderr_str();
    let stdout = output_text(&stdout_result);
    let stderr = output_text(&stderr_result);
    stderr
        .lines()
        .chain(stdout.lines())
        .find(|line| !line.trim().is_empty())
        .map_or("cargo command failed without output", |line| line)
        .to_owned()
}

const fn output_text<'a>(result: &'a Result<&'a str, crate::LaneError>) -> &'a str {
    match result {
        Ok(text) => text,
        Err(_) => "<non-UTF-8>",
    }
}

/// Dispatch to clean / failure / findings path based on output status.
///
/// # Errors
/// Returns [`RunCargoError::Outcome`] when clean-outcome construction fails.
fn lane_outcome(
    target: &TargetProject,
    lane: CargoLane,
    extra_args: &[String],
    output: &crate::CommandOutput,
    report: &LaneReport,
) -> Result<LaneOutcome, RunCargoError> {
    if output.success() && report.is_clean() {
        return clean_outcome(target, lane, extra_args, output);
    }
    if report.is_clean() {
        return Ok(LaneOutcome::Failed { failure: tool_failure(output) });
    }
    Ok(findings_outcome(lane, report))
}

fn tool_failure(output: &crate::CommandOutput) -> LaneFailure {
    LaneFailure::Tool {
        tool: String::from("cargo"),
        termination: process_termination(output.status()),
    }
}

/// Run cargo for *lane* with the appropriate flags.
///
/// # Errors
/// Returns [`crate::LaneError`] on command construction or execution failure.
fn cargo_output(
    target: &TargetProject,
    lane: CargoLane,
    extra_args: &[String],
) -> Result<crate::CommandOutput, crate::LaneError> {
    let mut command = crate::CommandIn::new(target, "cargo")?;
    let _ = command.inherit_env();
    let _ = command.args(args_for_lane(lane));
    let _ = command.args_strings(extra_args);
    command.run_capture_raw()
}
/// Normalize clippy JSONL output into a typed lane outcome.
///
/// # Errors
/// Returns [`RunCargoError`] when the clippy JSONL cannot be parsed or the
/// lane execution fails unexpectedly.
fn normalize_clippy_output(
    target: &TargetProject,
    lane: CargoLane,
    extra_args: &[String],
    output: &crate::CommandOutput,
) -> Result<LaneOutcome, RunCargoError> {
    let stdout = output.stdout_str().map_or("", |text| text);
    let normalized = crate::clippy_normalizer::normalize_clippy_jsonl(stdout);
    match normalized {
        crate::clippy_normalizer::ClippyNormalization::SuspiciousFailure(failure) => {
            Ok(LaneOutcome::Failed { failure })
        }
        crate::clippy_normalizer::ClippyNormalization::Findings(report) if !report.is_clean() => {
            Ok(findings_outcome(lane, &report))
        }
        crate::clippy_normalizer::ClippyNormalization::Findings(_) if !output.success() => {
            Ok(LaneOutcome::Failed { failure: tool_failure(output) })
        }
        crate::clippy_normalizer::ClippyNormalization::Findings(_) => {
            clean_outcome(target, lane, extra_args, output)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CargoLane, normalize_clippy_output};
    use crate::command::mock_output;
    use std::error::Error;
    use titania_core::{LaneOutcome, TargetProject};

    fn make_target() -> Result<(tempfile::TempDir, TargetProject), Box<dyn Error>> {
        let dir = tempfile::tempdir().map_err(std::io::Error::other)?;
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
        )
        .map_err(std::io::Error::other)?;
        let target = TargetProject::try_from_path(dir.path()).map_err(std::io::Error::other)?;
        Ok((dir, target))
    }

    /// Regression: nonzero clippy exit + empty stdout → Failed, not Clean.
    #[test]
    fn nonzero_exit_empty_stdout_becomes_failed_not_clean() -> Result<(), Box<dyn Error>> {
        let (_dir, target) = make_target()?;
        let output = mock_output(1, b"", b"")?;
        let outcome = normalize_clippy_output(&target, CargoLane::Clippy, &[], &output)
            .map_err(|e| std::io::Error::other(format!("{e:?}")))?;
        assert!(
            matches!(outcome, LaneOutcome::Failed { .. }),
            "nonzero exit + empty stdout should yield Failed, got {outcome:?}"
        );
        Ok(())
    }

    /// Nonzero clippy exit + clean JSON (no findings) → Failed.
    #[test]
    fn nonzero_exit_clean_json_becomes_failed_not_clean() -> Result<(), Box<dyn Error>> {
        let (_dir, target) = make_target()?;
        let output = mock_output(1, b"", b"")?;
        let outcome = normalize_clippy_output(&target, CargoLane::Clippy, &[], &output)
            .map_err(|e| std::io::Error::other(format!("{e:?}")))?;
        assert!(
            matches!(outcome, LaneOutcome::Failed { .. }),
            "nonzero exit + clean JSON should yield Failed, got {outcome:?}"
        );
        Ok(())
    }

    /// Nonzero clippy exit + valid JSON with unwrap findings → CLIPPY_UNWRAP_USED.
    #[test]
    fn nonzero_exit_with_unwrap_json_becomes_findings() -> Result<(), Box<dyn Error>> {
        let (_dir, target) = make_target()?;
        let fixture = include_str!("../tests/fixtures/clippy/unwrap.jsonl");
        let output = mock_output(1, fixture.as_bytes(), b"")?;
        let outcome = normalize_clippy_output(&target, CargoLane::Clippy, &[], &output)
            .map_err(|e| std::io::Error::other(format!("{e:?}")))?;
        assert!(
            matches!(outcome, LaneOutcome::Findings { .. }),
            "nonzero exit + unwrap JSON should yield Findings, got {outcome:?}"
        );
        if let LaneOutcome::Findings { findings, .. } = outcome {
            assert!(!findings.is_empty(), "report should have findings");
            assert!(
                findings.iter().any(|f| f.rule_id().to_string() == "CLIPPY_UNWRAP_USED"),
                "should contain CLIPPY_UNWRAP_USED finding; got: {findings:?}"
            );
        }
        Ok(())
    }

    /// Zero exit + clean JSON → Clean (sanity check, existing behavior preserved).
    #[test]
    fn zero_exit_clean_json_becomes_clean() -> Result<(), Box<dyn Error>> {
        let (_dir, target) = make_target()?;
        let output = mock_output(0, b"", b"")?;
        let outcome = normalize_clippy_output(&target, CargoLane::Clippy, &[], &output)
            .map_err(|e| std::io::Error::other(format!("{e:?}")))?;
        assert!(
            matches!(outcome, LaneOutcome::Clean { .. }),
            "zero exit + clean JSON should yield Clean, got {outcome:?}"
        );
        Ok(())
    }
}
