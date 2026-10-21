use core::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::error::LaneError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Lane {
    Fmt,
    Compile,
    Clippy,
    AstGrep,
    Dylint,
    PanicScan,
    PolicyScan,
    Test,
    Deny,
    Build,
    Kani,
    Mutants,
}

impl Lane {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Fmt => "Fmt",
            Self::Compile => "Compile",
            Self::Clippy => "Clippy",
            Self::AstGrep => "AstGrep",
            Self::Dylint => "Dylint",
            Self::PanicScan => "PanicScan",
            Self::PolicyScan => "PolicyScan",
            Self::Test => "Test",
            Self::Deny => "Deny",
            Self::Build => "Build",
            Self::Kani => "Kani",
            Self::Mutants => "Mutants",
        }
    }

    #[must_use]
    pub const fn depends_on_compile(self) -> bool {
        matches!(self, Self::Clippy | Self::Test | Self::Build)
    }

    #[must_use]
    pub const fn file_stem(self) -> &'static str {
        match self {
            Self::Fmt => "fmt",
            Self::Compile => "compile",
            Self::Clippy => "clippy",
            Self::AstGrep => "ast-grep",
            Self::Dylint => "dylint",
            Self::PanicScan => "panic-scan",
            Self::PolicyScan => "policy-scan",
            Self::Test => "test",
            Self::Deny => "deny",
            Self::Build => "build",
            Self::Kani => "kani",
            Self::Mutants => "mutants",
        }
    }
}

impl fmt::Display for Lane {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Lane {
    type Err = LaneError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Fmt" => Ok(Self::Fmt),
            "Compile" => Ok(Self::Compile),
            "Clippy" => Ok(Self::Clippy),
            "AstGrep" => Ok(Self::AstGrep),
            "Dylint" => Ok(Self::Dylint),
            "PanicScan" => Ok(Self::PanicScan),
            "PolicyScan" => Ok(Self::PolicyScan),
            "Test" => Ok(Self::Test),
            "Deny" => Ok(Self::Deny),
            "Build" => Ok(Self::Build),
            "Kani" => Ok(Self::Kani),
            "Mutants" => Ok(Self::Mutants),
            _ => Err(LaneError::UnknownLane(s.to_owned())),
        }
    }
}
