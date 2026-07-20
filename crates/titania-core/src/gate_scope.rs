//! Gate scopes — composite sets of lanes for CI/CD triggers.
//!
//! Four scope tiers define which lanes run:
//!
//! | Scope     | Lanes                                                               |
//! |-----------|---------------------------------------------------------------------|
//! | `Edit`    | Fmt, Compile, Clippy, AstGrep, Dylint, PanicScan, PolicyScan         |
//! | `Prepush` | Edit lanes + Test, Deny                                              |
//! | `Release` | Prepush lanes + Build                                                |
//! | `Full`    | Release lanes + Kani, Mutants                                        |

use serde::{Deserialize, Serialize};

use crate::{error::GateScopeError, lane::Lane};

/// Composite gate defining which lanes to run.
///
/// Total enum — every production match site is exhaustive so the compiler
/// catches drift whenever a new variant lands. Adding a future scope is a
/// breaking change that requires touching every match site on purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateScope {
    /// Edit-time gate — fastest feedback loop.
    Edit,
    /// Pre-push gate — adds test and supply-chain checks.
    Prepush,
    /// Release gate — adds the full release build.
    Release,
    /// Full gate — adds Kani bounded model-check and cargo mutants.
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
