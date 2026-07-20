//! Moon dispatch glue for `Command::Check` (spec §12, §13).
//!
//! `titania-check [--scope <s>]` is contractually required to drive Moon — it
//! is *not* a pure aggregate. This module owns the moon task list per scope,
//! the typed spawn error, and the moon binary resolution (with a
//! `TITANIA_MOON_BIN` override so hermetic tests can stub Moon).

use std::{env, io, path::PathBuf, process::Stdio};

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

/// Hermetic tool-home env vars to inject on the Moon subprocess.
///
/// Detected from `<target_root>/.titania/hermetic/{cargo-home,rustup-home}`
/// (v1-spec §9.5 dev-mode symlink compromise).
///
/// When present, these are set on the Moon subprocess so `PolicyScan` does not
/// emit `BYPASS_ENV_CARGO_HOME` / `BYPASS_ENV_RUSTUP_HOME`. When the process
/// env already points at the hermetic path (e.g. Moon CI set it via the
/// `env:` task block), the value is inherited as-is and not overridden.
#[derive(Debug, Clone, Default)]
pub(crate) struct HermeticEnv {
    cargo_home: Option<PathBuf>,
    rustup_home: Option<PathBuf>,
}

impl HermeticEnv {
    /// Detect hermetic tool homes under `<target_root>/.titania/hermetic/`.
    ///
    /// Returns a [`HermeticEnv`] whose `cargo_home` / `rustup_home` are `Some`
    /// when the corresponding directory (or symlink) exists. Existence is
    /// sufficient for the dev-mode symlink compromise (v1-spec §9.5); the
    /// target is not validated or canonicalized here.
    #[must_use]
    pub(crate) fn detect(target_root: &std::path::Path) -> Self {
        let hermetic_dir = target_root.join(".titania").join("hermetic");
        Self {
            cargo_home: existing_path(&hermetic_dir.join("cargo-home")),
            rustup_home: existing_path(&hermetic_dir.join("rustup-home")),
        }
    }

    /// Return `true` when both hermetic homes were detected.
    #[must_use]
    pub(crate) const fn is_complete(&self) -> bool {
        self.cargo_home.is_some() && self.rustup_home.is_some()
    }

    /// Apply the hermetic env vars to `cmd`, but only when the process env is
    /// not already set to the correct hermetic path. This avoids overriding a
    /// correctly-exported value when titania-check runs under Moon CI (where
    /// the `env:` task block already sets the vars).
    fn apply_to(&self, cmd: &mut std::process::Command) {
        apply_env_var(cmd, "CARGO_HOME", self.cargo_home.as_ref());
        apply_env_var(cmd, "RUSTUP_HOME", self.rustup_home.as_ref());
    }
}

/// Return the path when it exists (as a dir, file, or symlink), else `None`.
fn existing_path(path: &std::path::Path) -> Option<PathBuf> {
    path.exists().then(|| path.to_path_buf())
}

/// Set `name` to `hermetic` on `cmd` unless the process env already holds that
/// exact path.
fn apply_env_var(cmd: &mut std::process::Command, name: &str, hermetic: Option<&PathBuf>) {
    let Some(path) = hermetic else { return };
    if env_already_correct(name, path) {
        return;
    }
    let _ = cmd.env(name, path);
}

/// Return `true` when the process env var `name` already equals `expected`.
fn env_already_correct(name: &str, expected: &std::path::Path) -> bool {
    env::var(name).is_ok_and(|value| std::path::Path::new(&value) == expected)
}

/// Build the ordered list of `moon run` task IDs for a scope.
///
/// Matches the spec §13 gate composition: edit is the seven-lane base; prepush
/// adds test and deny; release adds build; full adds Kani and Mutants.
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
    if includes_full(scope) {
        tasks.extend_from_slice(FULL_TASKS);
    }
    tasks
}

/// Return `true` when `scope` is prepush, release, or full (scopes that
/// include the prepush task set).
const fn includes_prepush(scope: GateScope) -> bool {
    matches!(scope, GateScope::Prepush | GateScope::Release | GateScope::Full)
}

/// Return `true` when `scope` is release or full (scopes that add build).
const fn includes_release(scope: GateScope) -> bool {
    matches!(scope, GateScope::Release | GateScope::Full)
}

/// Return `true` when `scope` is full (the only scope that adds Kani and
/// Mutants).
const fn includes_full(scope: GateScope) -> bool {
    matches!(scope, GateScope::Full)
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

/// Full-scope additional moon task IDs (v1-spec §16 `gate-full` extra deps).
/// Kani and Mutants are intentionally scoped to Full only.
const FULL_TASKS: &[&str] = &[":titania-kani", ":titania-mutants"];

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
/// `hermetic` injects `CARGO_HOME` / `RUSTUP_HOME` on the Moon subprocess when
/// the hermetic dirs exist and the process env does not already point at them
/// (v1-spec §9.5). This closes the standalone `--scope` gap where Moon's
/// `env:` task block is not yet applied.
///
/// # Errors
/// - [`MoonSpawnError::NotFound`] when the moon binary is absent.
/// - [`MoonSpawnError::Failed`] for any other spawn/wait IO error.
pub(crate) fn spawn(
    binary: &str,
    tasks: &[&str],
    hermetic: &HermeticEnv,
) -> Result<(), MoonSpawnError> {
    use wait_timeout::ChildExt;

    let mut command = std::process::Command::new(binary);
    let _ = command.arg("run");
    let _ = command.args(tasks);
    let _ = command.stdin(Stdio::null());
    let _ = command.stdout(Stdio::null());
    let _ = command.stderr(Stdio::inherit());
    hermetic.apply_to(&mut command);

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

/// Run each Moon task sequentially via separate `moon run` invocations.
///
/// Moon's `run` command accepts a single target per invocation. Each lane
/// task runs independently so one rejecting lane cannot prevent the remaining
/// lanes from writing their artifacts. The exit status is intentionally
/// ignored — lanes that fail write `Failed`/missing artifacts, which the
/// in-process aggregate classifies.
///
/// `hermetic` is applied to every Moon subprocess so all lane children inherit
/// the controlled `CARGO_HOME` / `RUSTUP_HOME` (v1-spec §9.5).
///
/// # Errors
/// Returns [`MoonSpawnError`] when any Moon subprocess cannot be spawned or
/// exceeds the configured timeout.
pub(crate) fn spawn_all(
    binary: &str,
    tasks: &[&str],
    hermetic: &HermeticEnv,
) -> Result<(), MoonSpawnError> {
    tasks.iter().try_for_each(|task| spawn(binary, &[*task], hermetic))
}

/// Map a moon spawn IO error to its typed Moon spawn variant.
fn map_spawn_error(error: io::Error) -> MoonSpawnError {
    if error.kind() == io::ErrorKind::NotFound {
        MoonSpawnError::NotFound
    } else {
        MoonSpawnError::Failed(error)
    }
}
