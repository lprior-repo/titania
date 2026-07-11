//! Integration test for the template-prepush smoke (bead tn-rld.4).
//!
//! Generates a fresh workspace from the Titania cargo-generate skeleton into an
//! isolated temp directory, then runs `titania-check --scope prepush --emit json`
//! inside it.  Assertions check that:
//!
//! 1. The generated workspace actually exists with the expected files.
//! 2. `titania-check --scope prepush --emit json` succeeds.
//! 3. The JSON report is a v1 `Pass` with lane evidence.
//!
//! Selective acceptance filter:
//! `cargo test -p titania-check template_prepush`

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

use wait_timeout::ChildExt;
/// Return the root directory used by `generate_workspace` to host the
/// cargo-generate scaffold output.
///
/// Honors `TITANIA_TEST_TMPDIR` (CI / restricted environments) and defaults
/// to `$XDG_CACHE_HOME/titania-check-tests`, then `$HOME/.cache/titania-check-tests`,
/// then `std::env::temp_dir()`.
///
/// **Why not `/tmp`?** This machine's `/tmp` is a 62G tmpfs with both block
/// and inode quotas. `cargo generate` writes the generated workspace + lock
/// files + inner `cargo metadata` cache there; with the tmpfs at 80% use
/// the next allocation trips `os error 122 (EDQUOT)`. The home filesystem
/// is btrfs with no inode cap and 990G free, so test artifacts land cleanly.
fn test_workspace_dir() -> PathBuf {
    if let Ok(p) = std::env::var("TITANIA_TEST_TMPDIR") {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME") {
        return PathBuf::from(xdg).join("titania-check-tests");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".cache").join("titania-check-tests");
    }
    std::env::temp_dir()
}

/// Run a single external command, capturing stdout / stderr / exit code.
fn run_cmd<C, A, I>(cmd: C, args: I, cwd: Option<&std::path::Path>) -> CmdResult
where
    C: AsRef<std::ffi::OsStr>,
    A: AsRef<std::ffi::OsStr>,
    I: IntoIterator<Item = A>,
{
    let mut cmd = Command::new(&cmd);
    let _ = cmd.args(args);
    if let Some(dir) = cwd {
        let _ = cmd.current_dir(dir);
    }
    match cmd.output() {
        Ok(output) => CmdResult {
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            timed_out: false,
        },
        Err(e) => CmdResult {
            exit_code: None,
            stdout: String::new(),
            stderr: e.to_string(),
            timed_out: false,
        },
    }
}

fn run_command_with_timeout(command: &mut Command, timeout: Duration) -> CmdResult {
    let _ = command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return CmdResult {
                exit_code: None,
                stdout: String::new(),
                stderr: error.to_string(),
                timed_out: false,
            };
        }
    };

    match child.wait_timeout(timeout) {
        Ok(Some(_status)) => output_from_child(child, false),
        Ok(None) => {
            drop(child.kill());
            output_from_child(child, true)
        }
        Err(error) => CmdResult {
            exit_code: None,
            stdout: String::new(),
            stderr: error.to_string(),
            timed_out: false,
        },
    }
}

fn output_from_child(child: std::process::Child, timed_out: bool) -> CmdResult {
    match child.wait_with_output() {
        Ok(output) => CmdResult {
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            timed_out,
        },
        Err(error) => CmdResult {
            exit_code: None,
            stdout: String::new(),
            stderr: error.to_string(),
            timed_out,
        },
    }
}

#[derive(Debug)]
struct CmdResult {
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    timed_out: bool,
}

impl CmdResult {
    fn succeeded(&self) -> bool {
        self.exit_code == Some(0) && !self.timed_out
    }
}

/// Locate the titania-check binary.
///
/// Search order:
/// 1. `CARGO_TARGET_DIR` / debug / titania-check  (if set)
/// 2. `<worktree>/target/debug/titania-check` relative to the crate root
fn find_titania_check() -> PathBuf {
    let binary_name = format!("titania-check{}", std::env::consts::EXE_SUFFIX);
    // Derive the target/debug directory from the test binary's location.
    // Test binary: target/debug/deps/template_prepush-XXXX
    // Target binary: target/debug/titania-check[.exe]
    let mut exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    _ = exe.pop();
    _ = exe.pop();
    let target_debug = exe;
    let mut p = target_debug.join(&binary_name);
    if !p.exists() {
        if let Ok(dir) = std::env::var("CARGO_TARGET_DIR") {
            p = PathBuf::from(&dir).join("debug").join(&binary_name);
        }
    }
    p
}

/// Build and locate the local `titania-check` binary used to emulate an install.
fn ensure_titania_check_binary() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.ancestors().nth(2).unwrap();
    let result = run_cmd("cargo", ["build", "-p", "titania-check"], Some(workspace_root));
    assert!(
        result.succeeded(),
        "cargo build -p titania-check failed:\nstderr: {}\nstdout: {}",
        result.stderr,
        result.stdout
    );

    let check_bin = find_titania_check();
    assert!(
        check_bin.exists(),
        "titania-check binary not found at {check_bin:?} after cargo build -p titania-check"
    );
    check_bin
}

/// Build a PATH value that makes the just-built `titania-check` binary visible
/// to generated Moon lane tasks.
fn path_with_titania_check(check_bin: &std::path::Path) -> std::ffi::OsString {
    let check_dir = check_bin.parent().expect("titania-check binary must have a parent dir");
    let current_path = std::env::var_os("PATH").unwrap_or_default();
    std::env::join_paths(
        std::iter::once(check_dir.to_path_buf()).chain(std::env::split_paths(&current_path)),
    )
    .expect("PATH with titania-check binary dir must be valid")
}

/// Ensure the local Dylint cdylib exists in cargo-dylint's `--lib-path`
/// filename format and return its path.
fn ensure_dylint_library(_check_bin: &std::path::Path) -> PathBuf {
    let workspace = test_workspace_root();
    let result = run_cmd("cargo", ["dylint", "list", "--all"], Some(&workspace));
    assert!(
        result.succeeded(),
        "cargo dylint list --all failed while preparing template smoke Dylint library:\nstderr: {}\nstdout: {}",
        result.stderr,
        result.stdout
    );
    let target_env = std::env::var_os("CARGO_TARGET_DIR");
    let target_dir = resolve_cargo_target_dir(&workspace, target_env.as_deref());
    let library_root = target_dir.join("dylint").join("libraries");
    find_titania_dylint_library(&library_root).unwrap_or_else(|| {
        panic!("cargo dylint list --all did not create libtitania_dylint under {library_root:?}")
    })
}

fn test_workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("titania-check crate must live under <workspace>/crates/titania-check")
        .to_path_buf()
}

fn resolve_cargo_target_dir(
    workspace: &Path,
    cargo_target_dir: Option<&std::ffi::OsStr>,
) -> PathBuf {
    let Some(value) = cargo_target_dir else {
        return workspace.join("target");
    };
    let target_dir = PathBuf::from(value);
    if target_dir.is_absolute() { target_dir } else { workspace.join(target_dir) }
}

fn find_titania_dylint_library(root: &Path) -> Option<PathBuf> {
    std::fs::read_dir(root).ok()?.filter_map(Result::ok).find_map(|entry| {
        let path = entry.path();
        if path.is_dir() {
            return find_titania_dylint_library(&path);
        }
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                name.starts_with(&format!("{}titania_dylint@", std::env::consts::DLL_PREFIX))
                    && name.ends_with(std::env::consts::DLL_SUFFIX)
            })
            .then_some(path)
    })
}

/// Preserve one host variable when building an otherwise clean child env.
fn preserve_env(command: &mut Command, key: &str) {
    if let Some(value) = std::env::var_os(key) {
        let _ = command.env(key, value);
    }
}

/// Build a minimal environment for nested generated-workspace Moon execution.
fn configure_clean_child_env(
    command: &mut Command,
    check_bin: &std::path::Path,
    dylint_lib: &std::path::Path,
) {
    let _ = command.env_clear();
    let _ = command.env("PATH", path_with_titania_check(check_bin));
    let _ = command.env("TITANIA_DYLINT_LIB", dylint_lib);
    let _ = command.env("TITANIA_MOON_BIN", "moon");
    [
        "HOME",
        "USER",
        "LOGNAME",
        "USERPROFILE",
        "HOMEDRIVE",
        "HOMEPATH",
        "SystemRoot",
        "APPDATA",
        "LOCALAPPDATA",
        "CARGO_HOME",
        "CARGO_TARGET_DIR",
        "RUSTUP_HOME",
        "SCCACHE_DIR",
        "SSL_CERT_FILE",
        "SSL_CERT_DIR",
        "XDG_CACHE_HOME",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "TMPDIR",
        "TMP",
        "TEMP",
    ]
    .iter()
    .for_each(|key| preserve_env(command, key));
}

/// Generate a fresh workspace from the Titania template into an isolated temp
/// directory and return its path.  Panics on failure.
fn generate_workspace(name: &str) -> PathBuf {
    let template_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .join("titania")
        .join("template");

    // Unique name using nanosecond timestamp to avoid collision across runs.
    let ns = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let unique_name = format!("{name}-{ns}");
    let tmp_dir = {
        let parent = test_workspace_dir();
        std::fs::create_dir_all(&parent).expect("create test workspace dir");
        tempfile::Builder::new()
            .prefix("titania-prepush-")
            .tempdir_in(&parent)
            .expect("create tempdir")
            .keep()
    };
    let dest = tmp_dir.join(&unique_name);

    // Forward TMPDIR to the cargo-generate child so its inner
    // `cargo metadata` invocation also routes through the redirected
    // tmp_dir and avoids the /tmp tmpfs EDQUOT.
    // Use a builder chain so the unused-result lint from -D unused-results
    // (workspace AGENTS deny list) doesn't fire on `Command::env`'s
    // `&mut Self` return type. Store the eventual error via let _ = ...
    // just to thread the compiler; the methods return &mut Self which the
    // ignore pattern satisfies.
    let mut cmd = std::process::Command::new("cargo");
    let _ = cmd
        .arg("generate")
        .arg("--path")
        .arg(template_root.to_string_lossy().as_ref())
        .arg("--name")
        .arg(&unique_name);
    // Forward TMPDIR + CARGO_TARGET_DIR so the cargo-generate subprocess
    // AND its inner cargo (which writes build artifacts and `.fingerprint/`
    // files to TMPDIR by default) both route through the redirected tmpdir
    // instead of /tmp, where the 62G tmpfs keeps tripping EDQUOT (os 122).
    // cargo's default tmpdir lives at ${TMPDIR}/cargo-installXXXXXX/, so
    // moving TMPDIR alone wasn't enough — the spawned cargo also needs
    // its own target dir pinned off /tmp.
    let cargo_target = tmp_dir.join("cargo-target");
    std::fs::create_dir_all(&cargo_target).expect("create cargo target dir");
    // TMPDIR redirects cargo's `std::env::temp_dir()` for build artifacts;
    // CARGO_TARGET_DIR redirects the entire target dir; both are needed
    // because the /tmp tmpfs on this machine hits EDQUOT.
    let _ = cmd.env("TMPDIR", &tmp_dir);
    let _ = cmd.env("CARGO_TARGET_DIR", &cargo_target);
    let _ = cmd.env("CARGO_BUILD_JOBS", "1");
    let _ = cmd.current_dir(&tmp_dir);
    // CARGO_TARGET_DIR is preserved through the keep-list in
    // configure_clean_child_env, so the moon→cargo chain inherits it.
    // No unsafe block needed in this workspace; we add it to the
    // keep-list at module load order below.
    let result = run_command_with_timeout(&mut cmd, Duration::from_secs(120));

    assert!(
        result.succeeded(),
        "cargo generate failed:\nstderr: {}\nstdout: {}",
        result.stderr,
        result.stdout
    );

    // cargo generate creates <tmp_dir>/<name>/
    assert!(dest.exists(), "generated workspace does not exist at {dest:?}");

    // Verify key files are present
    let cargo_toml = dest.join("Cargo.toml");
    let deny_toml = dest.join("deny.toml");
    assert!(cargo_toml.exists(), "Cargo.toml missing from generated workspace at {cargo_toml:?}");
    assert!(deny_toml.exists(), "deny.toml missing from generated workspace at {deny_toml:?}");

    dest
}

/// Run `titania-check --scope prepush --emit json` in the given directory
/// and return the stdout (the JSON report).
fn run_prepush_check(workspace_dir: &std::path::Path) -> String {
    let check_bin = ensure_titania_check_binary();
    let dylint_lib = ensure_dylint_library(&check_bin);

    let mut command = Command::new(&check_bin);
    let _ = command.args(["--scope", "prepush", "--emit", "json"]).current_dir(workspace_dir);
    configure_clean_child_env(&mut command, &check_bin, &dylint_lib);

    let output = command
        .output()
        .map(|out| CmdResult {
            exit_code: out.status.code(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            timed_out: false,
        })
        .unwrap_or_else(|e| CmdResult {
            exit_code: None,
            stdout: String::new(),
            stderr: e.to_string(),
            timed_out: false,
        });
    let result = output;

    assert!(
        result.succeeded(),
        "titania-check prepush on generated template must pass:\nstderr: {}\nstdout: {}",
        result.stderr,
        result.stdout,
    );

    assert!(!result.stdout.is_empty(), "titania-check produced no stdout — JSON report is empty");

    // Validate it is valid JSON with expected top-level fields
    let json: serde_json::Value =
        serde_json::from_str(&result.stdout).expect("titania-check stdout is not valid JSON");

    assert!(
        json.get("variant").is_some(),
        "JSON report missing 'variant' field: {}",
        result.stdout
    );
    assert!(
        json.get("per_lane").is_some(),
        "JSON report missing 'per_lane' field: {}",
        result.stdout
    );

    result.stdout
}

fn run_moon_prepush_gate(workspace_dir: &std::path::Path) -> CmdResult {
    let check_bin = ensure_titania_check_binary();
    let dylint_lib = ensure_dylint_library(&check_bin);

    let mut command = Command::new("moon");
    let _ = command.args(["run", "titania:gate-prepush"]).current_dir(workspace_dir);
    configure_clean_child_env(&mut command, &check_bin, &dylint_lib);

    run_command_with_timeout(&mut command, Duration::from_secs(300))
}

const EXPECTED_PREPUSH_ARTIFACTS: &[&str] = &[
    "ast-grep.json",
    "clippy.json",
    "compile.json",
    "deny.json",
    "dylint.json",
    "fmt.json",
    "panic-scan.json",
    "policy-scan.json",
    "test.json",
];

fn assert_expected_prepush_artifacts(workspace_dir: &std::path::Path) {
    let prepush_dir = workspace_dir.join(".titania").join("out").join("prepush");
    assert!(prepush_dir.is_dir(), "prepush artifact directory missing at {prepush_dir:?}");

    let actual_files = std::fs::read_dir(&prepush_dir)
        .expect("prepush artifact directory must be readable")
        .map(|entry| {
            entry
                .expect("prepush artifact entry must be readable")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect::<BTreeSet<_>>();
    let expected_files =
        EXPECTED_PREPUSH_ARTIFACTS.iter().map(|name| (*name).to_owned()).collect::<BTreeSet<_>>();
    assert_eq!(
        actual_files, expected_files,
        "moon gate-prepush must produce exactly the expected prepush lane artifacts"
    );

    for artifact in EXPECTED_PREPUSH_ARTIFACTS {
        let path = prepush_dir.join(artifact);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("prepush artifact {path:?} must be readable: {error}"));
        let json: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|error| {
            panic!("prepush artifact {path:?} must be valid JSON: {error}")
        });
        assert!(
            json.is_object(),
            "prepush artifact {path:?} must serialize a JSON object; got {json}"
        );
    }
}

#[test]
fn template_prepush_generated_workspace_smoke() {
    let workspace = generate_workspace("tn-rld-4-smoke");
    let json_report = run_prepush_check(&workspace);

    // The report should pass out of the box per v1 DoD #10.
    let parsed: serde_json::Value =
        serde_json::from_str(&json_report).expect("report is valid JSON");

    assert_eq!(
        parsed["variant"], "Pass",
        "prepush on a fresh workspace should pass; full report: {json_report}"
    );

    let lanes = parsed
        .get("receipt")
        .and_then(|receipt| receipt.get("lanes"))
        .and_then(|lanes| lanes.as_array())
        .expect("passing template report must include receipt.lanes");

    assert!(
        lanes.iter().all(|lane| lane.get("clean").and_then(|clean| clean.as_bool()) == Some(true)),
        "all generated template prepush lanes must be clean; report: {json_report}"
    );

    // Clean up the generated workspace.
    std::fs::remove_dir_all(&workspace)
        .unwrap_or_else(|e| panic!("failed to clean up {workspace:?}: {e}"));
}

#[test]
fn template_moon_gate_prepush_generated_workspace_smoke() {
    let workspace = generate_workspace("tn-rld-4-moon-smoke");

    let result = run_moon_prepush_gate(&workspace);

    assert!(
        result.succeeded(),
        "moon run gate-prepush on generated template must exit 0 within timeout (timed_out={}):\nstderr: {}\nstdout: {}",
        result.timed_out,
        result.stderr,
        result.stdout
    );
    assert_expected_prepush_artifacts(&workspace);

    std::fs::remove_dir_all(&workspace)
        .unwrap_or_else(|e| panic!("failed to clean up {workspace:?}: {e}"));
}
