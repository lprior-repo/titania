//! kani-list public-API smoke tests.

use std::{error::Error, fs, path::Path, process::Command};

use serde_json::Value;
use tempfile::TempDir;

const MOON_TASKS: &str = include_str!("../../../.moon/tasks/all.yml");
const KANI_CORE: &str = include_str!("../../../.evidence/kani-list/titania-core.json");

fn write_workspace(root: &Path) -> Result<(), std::io::Error> {
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/alpha\", \"crates/beta\"]\n",
    )?;
    write_package(root, "alpha")?;
    write_package(root, "beta")
}

fn write_package(root: &Path, name: &str) -> Result<(), std::io::Error> {
    let package_dir = root.join("crates").join(name);
    fs::create_dir_all(package_dir.join("src"))?;
    fs::write(
        package_dir.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
    )?;
    fs::write(package_dir.join("src/lib.rs"), "pub fn marker() {}\n")
}

#[cfg(unix)]
fn make_fake_cargo(bin_dir: &Path) -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt;

    fs::create_dir_all(bin_dir)?;
    let cargo_path = bin_dir.join("cargo");
    fs::write(
        &cargo_path,
        r#"#!/bin/sh
printf 'PWD=%s
ARGS=%s
' "$PWD" "$*" >> "$CARGO_LOG_FILE"
if [ "$1" = "metadata" ]; then
  cat <<JSON
{"packages":[{"name":"alpha","manifest_path":"$FAKE_WORKSPACE_ROOT/crates/alpha/Cargo.toml"},{"name":"beta","manifest_path":"$FAKE_WORKSPACE_ROOT/crates/beta/Cargo.toml"}]}
JSON
  exit 0
fi
if [ "$1" = "kani" ] && [ "$2" = "list" ]; then
  printf '{"standard-harnesses":{},"contract-harnesses":{},"contracts":[],"totals":{}}\n' > kani-list.json
  exit 0
fi
exit 64
"#,
    )?;
    let mut permissions = fs::metadata(&cargo_path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&cargo_path, permissions)?;
    Ok(())
}

fn path_with_fake_cargo(bin_dir: &Path) -> String {
    std::env::var("PATH").map_or_else(
        |_| format!("{}:", bin_dir.display()),
        |existing| format!("{}:{existing}", bin_dir.display()),
    )
}

macro_rules! must {
    ($result:expr, $context:expr) => {
        must($result, $context)
    };
}

fn must<T, E: std::fmt::Display>(result: Result<T, E>, context: &str) -> T {
    match result {
        Ok(value) => value,
        Err(error) => {
            let message = format!("{context}: {error}");
            assert_eq!(message, "", "{message}");
            std::process::abort();
        }
    }
}

#[cfg(unix)]
#[test]
fn kani_list_package_mode_runs_cargo_kani_from_selected_package_dir() {
    let workspace = must!(TempDir::new(), "create workspace tempdir");
    must!(write_workspace(workspace.path()), "write workspace fixture");
    let fake_bin = workspace.path().join("fake-bin");
    must!(make_fake_cargo(&fake_bin), "create fake cargo");
    let output_dir = workspace.path().join("evidence");
    let log_path = workspace.path().join("cargo.log");

    let output = must!(
        Command::new(env!("CARGO_BIN_EXE_kani-list"))
            .arg("alpha")
            .current_dir(workspace.path())
            .env("PATH", path_with_fake_cargo(&fake_bin))
            .env("FAKE_WORKSPACE_ROOT", workspace.path())
            .env("CARGO_LOG_FILE", &log_path)
            .env("KANI_LIST_DIR", &output_dir)
            .output(),
        "run kani-list"
    );

    assert!(output.status.success(), "stderr={}", String::from_utf8_lossy(&output.stderr));
    assert!(output_dir.join("alpha.json").is_file());

    let log = must!(fs::read_to_string(&log_path), "read cargo log");
    let alpha_dir = workspace.path().join("crates/alpha");
    let beta_dir = workspace.path().join("crates/beta");
    assert!(
        log.contains(&format!("PWD={}", alpha_dir.display())),
        "cargo kani list was not run from alpha package dir; log:\n{log}"
    );
    assert!(
        log.contains("ARGS=kani list --format json"),
        "cargo kani list args changed unexpectedly; log:\n{log}"
    );
    assert!(!log.contains(&format!("PWD={}", beta_dir.display())));
}

#[test]
fn moon_kani_core_command_lists_every_recorded_core_harness() {
    let task = must!(moon_task("lane-kani-core"), "read lane-kani-core Moon task");
    let evidence: Value =
        must!(serde_json::from_str(KANI_CORE), "parse Kani titania-core evidence");
    let harnesses = must!(
        evidence
            .get("standard-harnesses")
            .and_then(|v| v.get("crates/titania-core/src/kani.rs"))
            .and_then(Value::as_array)
            .ok_or("missing titania-core Kani harness inventory"),
        "read titania-core Kani harness inventory"
    );

    for harness in harnesses {
        let harness = must!(harness.as_str().ok_or("non-string harness name"), "read harness name");
        let short_name = must!(
            harness.rsplit("::").next().ok_or("empty harness name"),
            "read short harness name"
        );
        assert!(
            task.contains(&format!("--harness {short_name}")),
            "lane-kani-core omits harness from evidence: {harness}\n{task}"
        );
    }
}

#[test]
fn moon_geiger_inputs_track_lockfile_and_crate_manifests() {
    let task = must!(moon_task("geiger"), "read geiger Moon task");
    assert!(task.contains("- 'Cargo.lock'"), "geiger must include Cargo.lock input\n{task}");
    assert!(
        task.contains("- 'crates/**/Cargo.toml'"),
        "geiger must include crate manifest inputs\n{task}"
    );
}

#[test]
fn moon_kani_list_command_scopes_to_titania_core() {
    // `lane-kani-list` must NOT run cargo-kani workspace-wide.
    // It must pass `titania-core` as a package argument so that only
    // the designated Kani-bearing production package is scanned.
    let task = must!(moon_task("lane-kani-list"), "read lane-kani-list Moon task");
    assert!(
        task.contains("-- titania-core"),
        "lane-kani-list must scope cargo-kani to titania-core package; task:\n{task}"
    );
    assert!(
        !task.contains("cargo kani"),
        "lane-kani-list must not invoke cargo kani workspace-wide; task:\n{task}"
    );
}

fn moon_task(name: &str) -> Result<&'static str, Box<dyn Error>> {
    let marker = format!("  {name}:\n");
    let (_before, after_marker) =
        MOON_TASKS.split_once(&marker).ok_or_else(|| format!("missing Moon task {name}"))?;
    let end = task_end(after_marker);
    after_marker.get(..end).ok_or_else(|| "invalid Moon task slice".into())
}

fn task_end(after_marker: &str) -> usize {
    let Some(end) = after_marker.match_indices("\n  ").find_map(|(idx, matched)| {
        let next_start = idx.checked_add(matched.len())?;
        let tail = after_marker.get(next_start..)?;
        tail.chars().next().filter(|next| !next.is_whitespace()).map(|_| idx)
    }) else {
        return after_marker.len();
    };
    end
}
