use titania_core::{OutcomeError, TargetProjectError};

mod args;
mod outcome;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CargoLane {
    Fmt,
    Compile,
    Clippy,
    Test,
    Build,
}

impl CargoLane {
    pub(super) fn parse(raw: &str) -> Result<Self, CargoLaneParseError> {
        (raw.trim() == raw).then_some(()).ok_or(CargoLaneParseError::Usage)?;
        match raw {
            "fmt" => Ok(Self::Fmt),
            "compile" => Ok(Self::Compile),
            "clippy" => Ok(Self::Clippy),
            "test" => Ok(Self::Test),
            "build" => Ok(Self::Build),
            _other => Err(CargoLaneParseError::Usage),
        }
    }

    pub(super) const fn tool_name(self) -> &'static str {
        match self {
            Self::Fmt => "cargo-fmt",
            Self::Compile => "cargo-check",
            Self::Clippy => "cargo-clippy",
            Self::Test => "cargo-test",
            Self::Build => "cargo-build",
        }
    }
}

#[derive(Debug)]
pub(super) enum CargoLaneParseError {
    Usage,
}

#[derive(Debug)]
pub(super) enum RunCargoError {
    Usage(String),
    Target(TargetProjectError),
    Command(crate::LaneError),
    CurrentDir(std::io::Error),
    Outcome(OutcomeError),
    ToolVersion(String),
    Policy(String),
}

pub(super) const fn args_for_lane(lane: CargoLane) -> &'static [&'static str] {
    args::args_for_lane(lane)
}

pub(super) fn clean_outcome(
    target: &titania_core::TargetProject,
    lane: CargoLane,
    extra_args: &[String],
    output: &crate::CommandOutput,
) -> Result<titania_core::LaneOutcome, RunCargoError> {
    outcome::clean_outcome(target, lane, extra_args, output)
}

pub(super) fn findings_outcome(
    lane: CargoLane,
    report: &crate::LaneReport,
) -> titania_core::LaneOutcome {
    outcome::findings_outcome(lane, report)
}

pub(super) fn process_termination(
    status: std::process::ExitStatus,
) -> titania_core::ProcessTermination {
    outcome::process_termination(status)
}
