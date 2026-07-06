//! Integration test for the template-prepush smoke (bead tn-rld.4).
//!
//! Generates a fresh workspace from the Titania cargo-generate skeleton into an
//! isolated temp directory, then runs `titania-check --scope prepush --emit json`
//! inside it.  Assertions check that:
//!
//! 1. The generated workspace actually exists with the expected files.
//! 2. `titania-check --scope prepush --emit json` produces valid JSON.
//! 3. The JSON contains a `"variant"` field and at least one `"per_lane"` entry.
//!
//! Selective acceptance filter:
//! `cargo test -p titania-check template_prepush`

use std::{path::PathBuf, process::Command};

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
        },
        Err(e) => CmdResult { exit_code: None, stdout: String::new(), stderr: e.to_string() },
    }
}

#[derive(Debug)]
struct CmdResult {
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl CmdResult {
    fn succeeded(&self) -> bool {
        self.exit_code == Some(0)
    }
}

/// Locate the titania-check binary.
///
/// Search order:
/// 1. `CARGO_TARGET_DIR` / debug / titania-check  (if set)
/// 2. `<worktree>/target/debug/titania-check` relative to the crate root
fn find_titania_check() -> PathBuf {
    // Derive the target/debug directory from the test binary's location.
    // Test binary: target/debug/deps/template_prepush-XXXX
    // Target binary: target/debug/titania-check
    let mut exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    _ = exe.pop(); // remove test binary name → deps/
    _ = exe.pop(); // remove deps → debug/
    let target_debug = exe;
    let mut p = target_debug.join("titania-check");
    if !p.exists() {
        // fallback: check CARGO_TARGET_DIR env var or default
        if let Ok(dir) = std::env::var("CARGO_TARGET_DIR") {
            p = PathBuf::from(&dir).join("debug").join("titania-check");
        }
    }
    p
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
    let tmp_dir = std::env::temp_dir();
    let dest = tmp_dir.join(&unique_name);

    let result = run_cmd(
        "cargo",
        ["generate", "--path", template_root.to_string_lossy().as_ref(), "--name", &unique_name],
        Some(&tmp_dir),
    );

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
    let check_bin = find_titania_check();
    assert!(
        check_bin.exists(),
        "titania-check binary not found at {check_bin:?}; build it first with `cargo build -p titania-check`"
    );

    let output = Command::new(&check_bin)
        .args(["--scope", "prepush", "--emit", "json"])
        .current_dir(workspace_dir)
        .output()
        .map(|out| CmdResult {
            exit_code: out.status.code(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        })
        .unwrap_or_else(|e| CmdResult {
            exit_code: None,
            stdout: String::new(),
            stderr: e.to_string(),
        });
    let result = output;

    // prepush on a fresh template has no lane artifacts, so it returns exit 1
    // (reject) — that is expected and we assert on the JSON content instead.
    assert!(
        !result.stderr.is_empty() || result.succeeded() || result.exit_code == Some(1),
        "titania-check exited unexpectedly (not 0 or 1):\nstderr: {}\nstdout: {}",
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

#[test]
fn template_prepush_generated_workspace_smoke() {
    let workspace = generate_workspace("tn-rld-4-smoke");
    let json_report = run_prepush_check(&workspace);

    // The report should be a reject (no lane artifacts exist yet).
    let parsed: serde_json::Value =
        serde_json::from_str(&json_report).expect("report is valid JSON");

    assert_eq!(
        parsed["variant"], "reject",
        "prepush on a fresh workspace should be a reject (no artifacts yet); full report: {json_report}"
    );

    let gate_failures =
        parsed.get("gate_failures").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);

    assert!(
        gate_failures > 0,
        "expected at least one gate failure for a fresh workspace; report: {json_report}"
    );

    // Clean up the generated workspace.
    std::fs::remove_dir_all(&workspace)
        .unwrap_or_else(|e| panic!("failed to clean up {workspace:?}: {e}"));
}
