//! Cargo lane argument and outcome construction.
//!
//! Splits the cargo lane implementation into focused submodules so that
//! `run_cargo_lane.rs` stays within the source-length threshold.

use titania_core::{OutcomeError, TargetProjectError};

mod args;
mod outcome;

/// Which cargo sub-lane to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CargoLane {
    /// `cargo fmt --check` formatting lane.
    Fmt,
    /// `cargo check` compilation lane.
    Compile,
    /// `cargo clippy` source lint lane.
    Clippy,
    /// `cargo test` behavior lane.
    Test,
    /// `cargo build --release` build lane.
    Build,
}

impl CargoLane {
    /// Parse a raw sub-command string into a [`CargoLane`].
    ///
    /// # Errors
    /// Returns [`CargoLaneParseError::Usage`] when *raw* is not a recognised
    /// sub-command name.
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

    /// Lane-specific tool name for `LaneFailure::Tool` per v1-spec §11.2.
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

/// Cargo lane parse errors.
#[derive(Debug)]
pub(super) enum CargoLaneParseError {
    /// The supplied sub-command name was not recognised.
    Usage,
}

/// Errors from a cargo lane execution pipeline.
#[derive(Debug)]
pub(super) enum RunCargoError {
    /// CLI usage or lane-selection error.
    Usage(String),
    /// Target project discovery failed.
    Target(TargetProjectError),
    /// Cargo command execution failed.
    Command(crate::LaneError),
    /// Process current directory could not be read.
    CurrentDir(std::io::Error),
    /// Typed outcome construction failed.
    Outcome(OutcomeError),
    /// Cargo version probe produced invalid evidence.
    ToolVersion(String),
}

/// Return the static cargo sub-command arguments for *lane*.
pub(super) const fn args_for_lane(lane: CargoLane) -> &'static [&'static str] {
    args::args_for_lane(lane)
}

/// Build clean-lane evidence including command argv and tool version.
///
/// # Errors
/// Returns [`RunCargoError::Outcome`] when command or lane evidence
/// construction fails, or [`RunCargoError::Command`] when the tool
/// version command cannot run.
pub(super) fn clean_outcome(
    target: &titania_core::TargetProject,
    lane: CargoLane,
    extra_args: &[String],
    output: &crate::CommandOutput,
) -> Result<titania_core::LaneOutcome, RunCargoError> {
    outcome::clean_outcome(target, lane, extra_args, output)
}

/// Turn a report with findings into a [`titania_core::LaneOutcome::Findings`].
pub(super) fn findings_outcome(
    lane: CargoLane,
    report: &crate::LaneReport,
) -> titania_core::LaneOutcome {
    outcome::findings_outcome(lane, report)
}

/// Convert a raw [`std::process::ExitStatus`] into a [`titania_core::ProcessTermination`].
pub(super) fn process_termination(
    status: std::process::ExitStatus,
) -> titania_core::ProcessTermination {
    outcome::process_termination(status)
}
