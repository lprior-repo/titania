use std::{env, fs, path::Path, process::Command};

use serde_json::Value;
use tempfile::TempDir;

fn binary() -> std::path::PathBuf {
    env::var("CARGO_BIN_EXE_titania-check")
        .expect("CARGO_BIN_EXE_titania-check not set; run via cargo test")
        .into()
}

fn run_in(root: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(binary());
    let _ = cmd.args(args);
    let _ = cmd.current_dir(root);
    #[cfg(unix)]
    let _ = cmd.env("TITANIA_MOON_BIN", "/bin/true");
    cmd.output().expect("failed to spawn titania-check")
}

#[cfg(unix)]
fn run_in_with_path_prefix(root: &Path, args: &[&str], prefix: &Path) -> std::process::Output {
    let mut cmd = Command::new(binary());
    let _ = cmd.args(args);
    let _ = cmd.current_dir(root);
    let _ = cmd.env("TITANIA_MOON_BIN", "/bin/true");
    let inherited = env::var_os("PATH").unwrap_or_default();
    let mut entries = vec![prefix.to_path_buf()];
    entries.extend(env::split_paths(&inherited));
    let joined = env::join_paths(entries).expect("PATH entries must join");
    let _ = cmd.env("PATH", joined);
    cmd.output().expect("failed to spawn titania-check")
}

#[cfg(unix)]
fn run_in_with_rustup_home(root: &Path, args: &[&str], rustup_home: &Path) -> std::process::Output {
    let mut cmd = Command::new(binary());
    let _ = cmd.args(args);
    let _ = cmd.current_dir(root);
    let _ = cmd.env("TITANIA_MOON_BIN", "/bin/true");
    let _ = cmd.env("RUSTUP_HOME", rustup_home);
    cmd.output().expect("failed to spawn titania-check")
}

fn clean_workspace() -> TempDir {
    let temp = tempfile::tempdir().expect("tempdir must be created");
    let edit = temp.path().join(".titania").join("out").join("edit");
    fs::create_dir_all(&edit).expect("artifact dir must be created");
    GateArtifact::edit_lanes().iter().for_each(|artifact| artifact.write(&edit));
    temp
}

struct GateArtifact {
    lane: &'static str,
    file: &'static str,
}

impl GateArtifact {
    const fn edit_lanes() -> [Self; 7] {
        [
            Self { lane: "Fmt", file: "fmt" },
            Self { lane: "Compile", file: "compile" },
            Self { lane: "Clippy", file: "clippy" },
            Self { lane: "AstGrep", file: "ast-grep" },
            Self { lane: "Dylint", file: "dylint" },
            Self { lane: "PanicScan", file: "panic-scan" },
            Self { lane: "PolicyScan", file: "policy-scan" },
        ]
    }

    fn write(&self, edit: &Path) {
        fs::write(edit.join(self.file).with_extension("json"), self.json())
            .expect("lane artifact must be written");
    }

    fn json(&self) -> String {
        format!(
            r#"{{"lane":"{}","outcome":{{"Clean":{{"evidence":{{"command":{{"executable":"titania-check","argv":["titania-check","run-lane","{}"]}},"tool_version":"embedded-test","exit_status":{{"Exited":{{"code":0}}}},"parsed_result_digest":"0000000000000000000000000000000000000000000000000000000000000000"}}}}}}}}"#,
            self.lane, self.file
        )
    }
}

fn parse_stdout(output: &std::process::Output, expected_code: i32) -> Value {
    assert_eq!(
        output.status.code(),
        Some(expected_code),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "report output should not write stderr");
    serde_json::from_slice(&output.stdout).expect("stdout must be JSON report")
}

fn toolchain_digest(report: &Value) -> &str {
    report["receipt"]["toolchain_digest"].as_str().expect("receipt toolchain_digest")
}

#[cfg(unix)]
fn write_fake_tool(fake_bin: &Path, tool: &str, version_line: &str) {
    use std::os::unix::fs::PermissionsExt;

    let script = fake_bin.join(tool);
    fs::write(&script, format!("#!/bin/sh\necho \"{version_line}\"\n"))
        .expect("fake tool must be written");
    let mut permissions = fs::metadata(&script).expect("fake tool metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&script, permissions).expect("fake tool must be made executable");
}

#[cfg(unix)]
#[test]
fn aggregate_toolchain_digest_ignores_path_planted_fake_toolchain() {
    let workspace = clean_workspace();
    let args = ["aggregate", "--scope", "edit", "--emit", "json"];
    let clean = run_in(workspace.path(), &args);
    let clean_report = parse_stdout(&clean, 0);
    assert_eq!(clean_report["variant"], "Pass");
    let fake_bin = workspace.path().join("fake-bin");
    fs::create_dir_all(&fake_bin).expect("fake bin dir must be created");
    write_fake_tool(&fake_bin, "rustc", "rustc 9.9.9 (fake 2020-01-01)");
    write_fake_tool(&fake_bin, "cargo", "cargo 9.9.9 (fake 2020-01-01)");
    let spoofed = run_in_with_path_prefix(workspace.path(), &args, &fake_bin);
    let spoofed_report = parse_stdout(&spoofed, 0);
    assert_eq!(spoofed_report["variant"], "Pass");
    assert_eq!(toolchain_digest(&spoofed_report), toolchain_digest(&clean_report));
    assert_eq!(spoofed.stdout, clean.stdout);
}

#[cfg(unix)]
#[test]
fn aggregate_with_unresolvable_toolchain_home_fails_closed() {
    let workspace = clean_workspace();
    let empty_home = workspace.path().join("empty-rustup-home");
    fs::create_dir_all(&empty_home).expect("empty rustup home must be created");
    let output = run_in_with_rustup_home(
        workspace.path(),
        &["aggregate", "--scope", "edit", "--emit", "json"],
        &empty_home,
    );
    assert_eq!(output.status.code(), Some(3), "aggregate must exit 3 without a toolchain");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("InternalError: toolchain digest probe failed"),
        "stderr must carry the typed toolchain probe diagnostic, got: {stderr}"
    );
}
