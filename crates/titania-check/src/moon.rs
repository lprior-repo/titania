//! Moon dispatch glue for `Command::Check` (spec §12, §13).
//!
//! `titania-check [--scope <s>]` is contractually required to drive Moon — it
//! is *not* a pure aggregate. This module owns the moon task list per scope,
//! the typed spawn error, and the moon binary resolution (with a
//! `TITANIA_MOON_BIN` override so hermetic tests can stub Moon).

use std::{env, io, process::Stdio};

use titania_core::GateScope;

/// Install hint surfaced when Moon is missing. Spec §14 lists Moon v2+ as a
/// hard prerequisite; v1-spec §12 says the check command runs lanes via Moon.
pub(crate) const MISSING_INSTALL_HINT: &str = concat!(
    "Moon CI/CD is required to run scoped lanes (v1-spec §14). ",
    "Install Moon: https://moonrepo.dev/moon/install"
);

/// Environment variable used to override the moon binary path.
///
/// Hermetic tests point this at a stub script that exits 0 (or records its
/// argv) so `Command::Check` can be exercised without a real Moon install.
const MOON_BIN_ENV: &str = "TITANIA_MOON_BIN";

/// Default moon binary name resolved through `PATH`.
const DEFAULT_MOON_BIN: &str = "moon";

/// Resolve the moon binary path, honoring the `TITANIA_MOON_BIN` override.
#[must_use]
pub(crate) fn binary_path() -> String {
    if let Ok(value) = env::var(MOON_BIN_ENV) {
        return value;
    }
    String::from(DEFAULT_MOON_BIN)
}

/// Build the ordered list of `moon run` task IDs for a scope.
///
/// Matches the spec §13 gate composition: edit is the seven-lane base; prepush
/// adds test and deny; release adds build.
#[must_use]
pub(crate) fn tasks_for_scope(scope: GateScope) -> Vec<&'static str> {
    let mut tasks = Vec::new();
    tasks.extend_from_slice(EDIT_TASKS);
    if includes_prepush(scope) {
        tasks.extend_from_slice(PREPUSH_TASKS);
    }
    if includes_release(scope) {
        tasks.extend_from_slice(RELEASE_TASKS);
    }
    tasks
}

/// Return `true` when `scope` is prepush or release (scopes that include the
/// prepush task set).
const fn includes_prepush(scope: GateScope) -> bool {
    matches!(scope, GateScope::Prepush | GateScope::Release)
}

/// Return `true` when `scope` is release (the only scope that adds build).
const fn includes_release(scope: GateScope) -> bool {
    matches!(scope, GateScope::Release)
}

/// Edit-scope moon task IDs (spec §13 `gate-edit` deps).
const EDIT_TASKS: &[&str] = &[
    ":titania-fmt",
    ":titania-compile",
    ":titania-clippy",
    ":titania-ast-grep",
    ":titania-dylint",
    ":titania-panic-scan",
    ":titania-policy-scan",
];

/// Prepush-scope additional moon task IDs (spec §13 `gate-prepush` extra deps).
const PREPUSH_TASKS: &[&str] = &[":titania-test", ":titania-deny"];

/// Release-scope additional moon task IDs (spec §13 `gate-release` extra deps).
const RELEASE_TASKS: &[&str] = &[":titania-build"];

const TIMEOUT_ENV: &str = "TITANIA_MOON_TIMEOUT_SECS";
const DEFAULT_TIMEOUT_SECS: u64 = 600;
const MAX_TIMEOUT_SECS: u64 = 24 * 60 * 60;

/// Resolve and clamp the wallclock timeout for a moon spawn (seconds).
/// Reads `TITANIA_MOON_TIMEOUT_SECS` if present; otherwise returns the default.
#[must_use]
pub(crate) fn timeout_seconds() -> u64 {
    let parsed = env::var(TIMEOUT_ENV)
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|v| *v > 0 && *v <= MAX_TIMEOUT_SECS);
    parsed.map_or(DEFAULT_TIMEOUT_SECS, std::convert::identity)
}

/// Typed failure mode for the moon spawn.
#[derive(Debug)]
pub(crate) enum MoonSpawnError {
    /// The moon binary could not be found on `PATH` (or at the overridden
    /// path). Surfaces to the user as `InputError(3)`.
    NotFound,
    /// The moon subprocess exceeded the wallclock timeout.
    /// Surfaces as `InternalError(>=4)`.
    TimedOut {
        /// Configured timeout in seconds.
        seconds: u64,
    },
    /// Any other spawn/wait IO failure. Surfaces as `InternalError(>=4)`.
    Failed(io::Error),
}

/// Spawn `moon run <tasks...>` with stderr inherited so the user sees lane
/// progress.
///
/// Moon's stdout is discarded (null) so it cannot pollute the aggregate report
/// later written to stdout. The exit status is intentionally ignored: lanes
/// that fail write `Failed`/missing artifacts, which the in-process aggregate
/// classifies.
///
/// # Errors
/// - [`MoonSpawnError::NotFound`] when the moon binary is absent.
/// - [`MoonSpawnError::Failed`] for any other spawn/wait IO error.
pub(crate) fn spawn(binary: &str, tasks: &[&str]) -> Result<(), MoonSpawnError> {
    use wait_timeout::ChildExt;

    let mut command = std::process::Command::new(binary);
    let _ = command.arg("run");
    let _ = command.args(tasks);
    let _ = command.stdin(Stdio::null());
    let _ = command.stdout(Stdio::null());
    let _ = command.stderr(Stdio::inherit());

    let mut child = command.spawn().map_err(map_spawn_error)?;
    let seconds = timeout_seconds();
    match child.wait_timeout(std::time::Duration::from_secs(seconds)) {
        Ok(Some(_status)) => Ok(()),
        Ok(None) => {
            drop(child.kill());
            drop(child.wait());
            Err(MoonSpawnError::TimedOut { seconds })
        }
        Err(error) => {
            drop(child.kill());
            drop(child.wait());
            Err(MoonSpawnError::Failed(error))
        }
    }
}

/// Map a moon spawn IO error to its typed Moon spawn variant.
fn map_spawn_error(error: io::Error) -> MoonSpawnError {
    if error.kind() == io::ErrorKind::NotFound {
        MoonSpawnError::NotFound
    } else {
        MoonSpawnError::Failed(error)
    }
}
