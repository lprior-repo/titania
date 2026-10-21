use std::{env, io, path::PathBuf, process::Stdio};

use titania_core::GateScope;

pub(crate) const MISSING_INSTALL_HINT: &str = concat!(
    "Moon CI/CD is required to run scoped lanes (v1-spec §14). ",
    "Install Moon: https://moonrepo.dev/moon/install"
);

const MOON_BIN_ENV: &str = "TITANIA_MOON_BIN";

const DEFAULT_MOON_BIN: &str = "moon";

#[must_use]
pub(crate) fn binary_path() -> String {
    if let Ok(value) = env::var(MOON_BIN_ENV) {
        return value;
    }
    String::from(DEFAULT_MOON_BIN)
}

#[derive(Debug, Clone, Default)]
pub(crate) struct HermeticEnv {
    cargo_home: Option<PathBuf>,
    rustup_home: Option<PathBuf>,
}

impl HermeticEnv {
    #[must_use]
    pub(crate) fn detect(target_root: &std::path::Path) -> Self {
        let hermetic_dir = target_root.join(".titania").join("hermetic");
        Self {
            cargo_home: existing_path(&hermetic_dir.join("cargo-home")),
            rustup_home: existing_path(&hermetic_dir.join("rustup-home")),
        }
    }

    #[must_use]
    pub(crate) const fn is_complete(&self) -> bool {
        self.cargo_home.is_some() && self.rustup_home.is_some()
    }

    fn apply_to(&self, cmd: &mut std::process::Command) {
        apply_env_var(cmd, "CARGO_HOME", self.cargo_home.as_ref());
        apply_env_var(cmd, "RUSTUP_HOME", self.rustup_home.as_ref());
    }
}

fn existing_path(path: &std::path::Path) -> Option<PathBuf> {
    if !path.exists() {
        return None;
    }
    if let Ok(canonical) = std::fs::canonicalize(path) {
        return Some(canonical);
    }
    Some(path.to_path_buf())
}

fn apply_env_var(cmd: &mut std::process::Command, name: &str, hermetic: Option<&PathBuf>) {
    let Some(path) = hermetic else { return };
    if env_already_correct(name, path) {
        return;
    }
    let _ = cmd.env(name, path);
}

fn env_already_correct(name: &str, expected: &std::path::Path) -> bool {
    env::var(name).is_ok_and(|value| std::path::Path::new(&value) == expected)
}

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

const fn includes_prepush(scope: GateScope) -> bool {
    matches!(scope, GateScope::Prepush | GateScope::Release | GateScope::Full)
}

const fn includes_release(scope: GateScope) -> bool {
    matches!(scope, GateScope::Release | GateScope::Full)
}

const fn includes_full(scope: GateScope) -> bool {
    matches!(scope, GateScope::Full)
}

const EDIT_TASKS: &[&str] = &[
    ":titania-fmt",
    ":titania-compile",
    ":titania-clippy",
    ":titania-ast-grep",
    ":titania-dylint",
    ":titania-panic-scan",
    ":titania-policy-scan",
];

const PREPUSH_TASKS: &[&str] = &[":titania-test", ":titania-deny"];

const RELEASE_TASKS: &[&str] = &[":titania-build"];

const FULL_TASKS: &[&str] = &[":titania-kani", ":titania-mutants"];

const TIMEOUT_ENV: &str = "TITANIA_MOON_TIMEOUT_SECS";
const DEFAULT_TIMEOUT_SECS: u64 = 600;
const MAX_TIMEOUT_SECS: u64 = 24 * 60 * 60;

#[must_use]
pub(crate) fn timeout_seconds() -> u64 {
    let Ok(raw) = env::var(TIMEOUT_ENV) else {
        return DEFAULT_TIMEOUT_SECS;
    };
    let Ok(parsed) = raw.trim().parse::<u64>() else {
        return DEFAULT_TIMEOUT_SECS;
    };
    if parsed > 0 && parsed <= MAX_TIMEOUT_SECS { parsed } else { DEFAULT_TIMEOUT_SECS }
}

#[derive(Debug)]
pub(crate) enum MoonSpawnError {
    NotFound,
    TimedOut { seconds: u64 },
    Failed(io::Error),
}

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

pub(crate) fn spawn_all(
    binary: &str,
    tasks: &[&str],
    hermetic: &HermeticEnv,
) -> Result<(), MoonSpawnError> {
    tasks.iter().try_for_each(|task| spawn(binary, &[*task], hermetic))
}

fn map_spawn_error(error: io::Error) -> MoonSpawnError {
    if error.kind() == io::ErrorKind::NotFound {
        MoonSpawnError::NotFound
    } else {
        MoonSpawnError::Failed(error)
    }
}
