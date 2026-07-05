use crate::usage_message;

const RULE_FMT: &str = "CARGO_FMT_001";
const RULE_COMPILE: &str = "CARGO_COMPILE_001";
const RULE_CLIPPY: &str = "CARGO_CLIPPY_001";
const RULE_TEST: &str = "CARGO_TEST_001";
const RULE_BUILD: &str = "CARGO_BUILD_001";

#[derive(Debug, Clone, Copy)]
pub(crate) struct CargoLaneParseError;

impl std::fmt::Display for CargoLaneParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&usage_message())
    }
}

impl std::error::Error for CargoLaneParseError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CargoLane {
    Fmt,
    Compile,
    Clippy,
    Test,
    Build,
}

impl CargoLane {
    /// Parses a `run-cargo` lane selector.
    ///
    /// # Errors
    ///
    /// Returns a usage message when `raw` has surrounding whitespace or does
    /// not name a supported Cargo lane.
    pub(crate) fn parse(raw: &str) -> Result<Self, CargoLaneParseError> {
        (raw.trim() == raw).then_some(()).ok_or(CargoLaneParseError)?;
        match raw {
            "fmt" => Ok(Self::Fmt),
            "compile" => Ok(Self::Compile),
            "clippy" => Ok(Self::Clippy),
            "test" => Ok(Self::Test),
            "build" => Ok(Self::Build),
            _other => Err(CargoLaneParseError),
        }
    }

    pub(crate) const fn rule(self) -> &'static str {
        match self {
            Self::Fmt => RULE_FMT,
            Self::Compile => RULE_COMPILE,
            Self::Clippy => RULE_CLIPPY,
            Self::Test => RULE_TEST,
            Self::Build => RULE_BUILD,
        }
    }

    pub(crate) const fn path(self) -> &'static str {
        match self {
            Self::Fmt => "cargo fmt",
            Self::Compile => "cargo check",
            Self::Clippy => "cargo clippy",
            Self::Test => "cargo test",
            Self::Build => "cargo build",
        }
    }
}
