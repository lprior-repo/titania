//! Execution lanes in the titania-check pipeline.
//!
//! Each lane corresponds to a single tool or analysis pass. The set of lanes
//! is fixed for v1 and serialised to `PascalCase` JSON.
//!
//! Construction is total: [`Lane::from_str`] parses a `PascalCase` string into
//! the matching variant, or returns a [`LaneError`].

use core::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::error::LaneError;

/// Execution lane in the titania-check pipeline.
///
/// Each variant names a single tool or analysis pass. Serialized to
/// `PascalCase` JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Lane {
    /// Format-check lane (`cargo fmt --check`).
    Fmt,
    /// Compile-check lane (`cargo check --workspace --frozen`).
    Compile,
    /// Clippy lint lane (`cargo clippy --workspace --lib --bins`).
    Clippy,
    /// Structural ast-grep lane (embedded rules).
    AstGrep,
    /// Type-aware dylint lane (`cargo dylint titania`).
    Dylint,
    /// Panic-macro scan lane (`rg` prefilter).
    PanicScan,
    /// Policy-violation scan lane (native TOML + env scanner).
    PolicyScan,
    /// Test lane (`cargo test --workspace --frozen`).
    Test,
    /// Supply-chain deny lane (`cargo deny check`).
    Deny,
    /// Release build lane (`cargo build --workspace --release`).
    Build,
}

impl Lane {
    /// Uppercase-Pascal display name matching the serde representation.
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
            _ => Err(LaneError::UnknownLane(s.to_owned())),
        }
    }
}
