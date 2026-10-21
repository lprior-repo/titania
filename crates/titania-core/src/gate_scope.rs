use serde::{Deserialize, Serialize};

use crate::{error::GateScopeError, lane::Lane};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum GateScope {
    Edit,
    Prepush,
    Release,
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
    pub const fn name(self) -> &'static str {
        match self {
            Self::Edit => "edit",
            Self::Prepush => "prepush",
            Self::Release => "release",
        }
    }

    #[must_use]
    pub const fn lanes(self) -> &'static [Lane] {
        match self {
            Self::Edit => EDIT_LANES,
            Self::Prepush => PREPUSH_LANES,
            Self::Release => RELEASE_LANES,
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
            _ => Err(GateScopeError::UnknownScope(s.to_owned())),
        }
    }
}
