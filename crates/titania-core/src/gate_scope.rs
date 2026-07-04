//! Gate scopes — composite sets of lanes for CI/CD triggers.
//!
//! Three scope tiers define which lanes run:
//!
//! | Scope   | Lanes                                                          |
//! |---------|----------------------------------------------------------------|
//! | `Edit`      | Fmt, Compile, Clippy, AstGrep, Dylint, PanicScan, PolicyScan      |
//! | `Prepush`   | Edit lanes + Test, Deny                                          |
//! | `Release`   | Prepush lanes + Build                                            |

use serde::{Deserialize, Serialize};

use crate::{error::GateScopeError, lane::Lane};

/// Composite gate defining which lanes to run.
///
/// `#[non_exhaustive]` ensures forward compatibility: v1.5 can add
/// `Full` and v2.5 can add `Deep` without breaking downstream match
/// expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum GateScope {
    /// Edit-time gate — fastest feedback loop.
    Edit,
    /// Pre-push gate — adds test and supply-chain checks.
    Prepush,
    /// Release gate — adds the full release build.
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
    /// Ordered slice of lanes this scope exercises.
    ///
    /// The slice is ordered so that lanes which depend on prior lanes
    /// (e.g. `Test` depends on `Compile`) appear after their prerequisites.
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
