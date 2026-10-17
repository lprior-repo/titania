use super::run_cargo::{
    CargoLane, RunCargoError, args_for_lane, clean_outcome, findings_outcome, process_termination,
};
use crate::{LaneExit, discover::discover_target};
use titania_core::{Lane, LaneFailure, LaneOutcome, TargetProject};

pub(super) enum CargoRun {
    Unsupported,
    Complete(LaneOutcome),
    Error { exit: LaneExit, message: String },
}

pub(super) fn run(lane: Lane) -> CargoRun {
    let Some(lane_name) = cargo_lane_name(lane) else {
        return CargoRun::Unsupported;
    };
    match run_checked(lane_name) {
        Ok(outcome) => CargoRun::Complete(outcome),
        Err(error) => CargoRun::Error { exit: error_exit(&error), message: error_message(error) },
    }
}

const fn cargo_lane_name(lane: Lane) -> Option<&'static str> {
    match lane {
        Lane::Fmt => Some("fmt"),
        Lane::Compile => Some("compile"),
        Lane::Clippy => Some("clippy"),
        Lane::Test => Some("test"),
        Lane::Build => Some("build"),
        Lane::AstGrep
        | Lane::Dylint
        | Lane::PanicScan
        | Lane::PolicyScan
        | Lane::Deny
        | Lane::Kani
        | Lane::Mutants => None,
    }
}

const fn error_exit(error: &RunCargoError) -> LaneExit {
    match error {
        RunCargoError::Usage(_) | RunCargoError::Target(_) => LaneExit::Usage,
        RunCargoError::Policy(_) => LaneExit::Violations,
        _other => LaneExit::Failure,
    }
}

fn error_message(error: RunCargoError) -> String {
    match error {
        RunCargoError::Usage(message)
        | RunCargoError::ToolVersion(message)
        | RunCargoError::Policy(message) => message,
        RunCargoError::Target(error) => format!("target discovery failed: {error}"),
        RunCargoError::Command(error) => format!("cargo execution failed: {error}"),
        RunCargoError::CurrentDir(error) => format!("cannot read current directory: {error}"),
        RunCargoError::Outcome(error) => format!("lane outcome construction failed: {error}"),
    }
}

fn usage_message() -> String {
    String::from("usage: run-lane <fmt|compile|clippy|test|build>")
}

fn run_checked(lane_name: &str) -> Result<LaneOutcome, RunCargoError> {
    let cargo_lane = CargoLane::parse(lane_name)
        .map_err(|_parse_error| RunCargoError::Usage(usage_message()))?;
    let cwd = std::env::current_dir().map_err(RunCargoError::CurrentDir)?;
    let target = discover_target(&cwd).map_err(RunCargoError::Target)?;
    run_lane(&target, cargo_lane, &[])
}

fn run_lane(
    target: &TargetProject,
    lane: CargoLane,
    extra_args: &[String],
) -> Result<LaneOutcome, RunCargoError> {
    crate::policy_scan::invocation::validate_extra_args(extra_args)
        .map_err(|error| RunCargoError::Policy(error.to_string()))?;
    let output = cargo_output(target, lane, extra_args).map_err(RunCargoError::Command)?;
    if lane == CargoLane::Clippy {
        return normalize_clippy_output(target, lane, extra_args, &output);
    }
    lane_outcome(target, lane, extra_args, &output)
}

fn lane_outcome(
    target: &TargetProject,
    lane: CargoLane,
    extra_args: &[String],
    output: &crate::CommandOutput,
) -> Result<LaneOutcome, RunCargoError> {
    if output.success() {
        return clean_outcome(target, lane, extra_args, output);
    }
    Ok(LaneOutcome::Failed { failure: tool_failure(lane, output) })
}

fn tool_failure(lane: CargoLane, output: &crate::CommandOutput) -> LaneFailure {
    LaneFailure::Tool {
        tool: String::from(lane.tool_name()),
        termination: process_termination(output.status()),
    }
}

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
            Ok(LaneOutcome::Failed { failure: tool_failure(lane, output) })
        }
        crate::clippy_normalizer::ClippyNormalization::Findings(_) => {
            clean_outcome(target, lane, extra_args, output)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CargoLane, RunCargoError, normalize_clippy_output, run_lane};
    use crate::{command::mock_output, discover::try_from_path};
    use std::error::Error;
    use titania_core::{LaneOutcome, Location, TargetProject};

    fn make_target() -> Result<(tempfile::TempDir, TargetProject), Box<dyn Error>> {
        let dir = tempfile::tempdir().map_err(std::io::Error::other)?;
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
        )
        .map_err(std::io::Error::other)?;
        let target = try_from_path(dir.path()).map_err(std::io::Error::other)?;
        Ok((dir, target))
    }

    fn serialized_span_line_start(location: &Location) -> Result<u64, Box<dyn Error>> {
        let value = serde_json::to_value(location)?;
        let line_start = value
            .get("Span")
            .and_then(|span| span.get("line_start"))
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| std::io::Error::other(format!("expected serialized Span: {value}")))?;
        Ok(line_start)
    }

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
            assert!(
                findings.iter().any(|f| f
                    .location()
                    .span_file()
                    .is_some_and(|path| path.as_str() == "src/lib.rs")),
                "clippy findings must preserve source span locations; got: {findings:?}"
            );
            let span_line_starts = findings
                .iter()
                .map(|finding| serialized_span_line_start(finding.location()))
                .collect::<Result<Vec<_>, _>>()?;
            assert_eq!(
                span_line_starts,
                vec![3, 7],
                "clippy findings must preserve exact serialized Location::Span line_start values from fixture"
            );
        }
        Ok(())
    }

    #[test]
    fn run_lane_rejects_cap_lints_before_spawn() -> Result<(), Box<dyn Error>> {
        let (_dir, target) = make_target()?;
        let extra = [String::from("--cap-lints"), String::from("allow")];
        match run_lane(&target, CargoLane::Clippy, &extra) {
            Err(RunCargoError::Policy(message)) => {
                assert!(
                    message.contains("--cap-lints"),
                    "policy error must name the ceiling token; got {message:?}"
                );
                Ok(())
            }
            Err(other) => {
                Err(std::io::Error::other(format!("expected RunCargoError::Policy, got {other:?}"))
                    .into())
            }
            Ok(outcome) => Err(std::io::Error::other(format!(
                "ceiling extra args must not produce an outcome; got {outcome:?}"
            ))
            .into()),
        }
    }

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
