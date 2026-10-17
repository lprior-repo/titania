use serde::{Deserialize, Serialize};

use crate::{error::GateScopeError, lane::Lane};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateScope {
    Edit,
    Prepush,
    Release,
    Full,
}

const EDIT_LANES: &[Lane] = &[
    Lane::Fmt,
    Lane::Compile,
    Lane::Clippy,
    Lane::AstGrep,
    Lane::Dylint,
    Lane::PanicScan,
    Lane::PolicyScan,
];
const FULL_LANES: &[Lane] = &[
    Lane::Fmt,
    Lane::Compile,
    Lane::Clippy,
    Lane::AstGrep,
    Lane::Dylint,
    Lane::PanicScan,
    Lane::PolicyScan,
    Lane::Test,
    Lane::Deny,
    Lane::Build,
    Lane::Kani,
    Lane::Mutants,
];

const PREPUSH_LANES: &[Lane] = &[
    Lane::Fmt,
    Lane::Compile,
    Lane::Clippy,
    Lane::AstGrep,
    Lane::Dylint,
    Lane::PanicScan,
    Lane::PolicyScan,
    Lane::Test,
    Lane::Deny,
];

const RELEASE_LANES: &[Lane] = &[
    Lane::Fmt,
    Lane::Compile,
    Lane::Clippy,
    Lane::AstGrep,
    Lane::Dylint,
    Lane::PanicScan,
    Lane::PolicyScan,
    Lane::Test,
    Lane::Deny,
    Lane::Build,
];

impl GateScope {
    #[must_use]
    pub const fn lanes(self) -> &'static [Lane] {
        match self {
            Self::Edit => EDIT_LANES,
            Self::Prepush => PREPUSH_LANES,
            Self::Release => RELEASE_LANES,
            Self::Full => FULL_LANES,
        }
    }
}

impl core::str::FromStr for GateScope {
    type Err = GateScopeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "edit" => Ok(Self::Edit),
            "prepush" => Ok(Self::Prepush),
            "release" => Ok(Self::Release),
            "full" => Ok(Self::Full),
            _ => Err(GateScopeError::UnknownScope(s.to_owned())),
        }
    }
}
