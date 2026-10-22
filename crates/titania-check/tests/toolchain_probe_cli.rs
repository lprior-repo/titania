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
const PINNED_TOOLCHAIN_DIR: &str = "nightly-2026-04-27-x86_64-unknown-linux-gnu";

#[cfg(unix)]
const DEFAULT_TOOLCHAIN_DIR: &str = "stable-x86_64-unknown-linux-gnu";

#[cfg(unix)]
fn write_logging_tool(home: &Path, toolchain_dir: &str, tag: &str, tool: &str, version_line: &str) {
    use std::os::unix::fs::PermissionsExt;

    let bin_dir = home.join("toolchains").join(toolchain_dir).join("bin");
    fs::create_dir_all(&bin_dir).expect("fake toolchain bin dir must be created");
    let script = bin_dir.join(tool);
    let body = format!(
        "#!/bin/sh\necho \"{tag}-{tool}\" >> \"{}\"\necho \"{version_line}\"\n",
        home.join("calls.log").display()
    );
    fs::write(&script, body).expect("fake logging tool must be written");
    let mut permissions = fs::metadata(&script).expect("fake tool metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&script, permissions).expect("fake tool must be made executable");
}

#[cfg(unix)]
fn fake_rustup_home(settings: &str) -> TempDir {
    let home = tempfile::tempdir().expect("fake rustup home must be created");
    write_logging_tool(
        home.path(),
        PINNED_TOOLCHAIN_DIR,
        "pinned",
        "rustc",
        "rustc 1.97.0-nightly (ca9a134e0 2026-04-26)",
    );
    write_logging_tool(
        home.path(),
        PINNED_TOOLCHAIN_DIR,
        "pinned",
        "cargo",
        "cargo 1.97.0-nightly (eb9b60f1f 2026-04-24)",
    );
    write_logging_tool(
        home.path(),
        DEFAULT_TOOLCHAIN_DIR,
        "default",
        "rustc",
        "rustc 1.85.0 (4d91de4e4 2025-02-17)",
    );
    write_logging_tool(
        home.path(),
        DEFAULT_TOOLCHAIN_DIR,
        "default",
        "cargo",
        "cargo 1.85.0 (d73d2caf9 2024-12-31)",
    );
    fs::write(home.path().join("settings.toml"), settings)
        .expect("fake rustup settings must be written");
    home
}

#[cfg(unix)]
fn calls_log(home: &Path) -> String {
    fs::read_to_string(home.join("calls.log")).expect("calls log must exist")
}

#[cfg(unix)]
fn assert_probe_fails_closed(output: &std::process::Output, home: &Path) {
    assert_eq!(output.status.code(), Some(3), "aggregate must exit 3");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("InternalError: toolchain digest probe failed"),
        "stderr must carry the typed toolchain probe diagnostic, got: {stderr}"
    );
    assert!(!home.join("calls.log").exists(), "no toolchain binary may execute");
}

#[cfg(unix)]
#[test]
fn aggregate_executes_pinned_toolchain_for_commented_pin() {
    let workspace = clean_workspace();
    fs::write(
        workspace.path().join("rust-toolchain.toml"),
        "[toolchain]\nchannel = \"nightly-2026-04-27\" # pinned for reproducibility\n",
    )
    .expect("pin file must be written");
    let home = fake_rustup_home("default_toolchain = \"stable-x86_64-unknown-linux-gnu\"\n");
    let output = run_in_with_rustup_home(
        workspace.path(),
        &["aggregate", "--scope", "edit", "--emit", "json"],
        home.path(),
    );
    let report = parse_stdout(&output, 0);
    assert_eq!(report["variant"], "Pass");
    assert_eq!(calls_log(home.path()), "pinned-rustc\npinned-cargo\n");
}

#[cfg(unix)]
#[test]
fn aggregate_executes_pinned_toolchain_for_single_quoted_pin() {
    let workspace = clean_workspace();
    fs::write(
        workspace.path().join("rust-toolchain.toml"),
        "[toolchain]\nchannel = 'nightly-2026-04-27'\n",
    )
    .expect("pin file must be written");
    let home = fake_rustup_home("default_toolchain = \"stable-x86_64-unknown-linux-gnu\"\n");
    let output = run_in_with_rustup_home(
        workspace.path(),
        &["aggregate", "--scope", "edit", "--emit", "json"],
        home.path(),
    );
    let report = parse_stdout(&output, 0);
    assert_eq!(report["variant"], "Pass");
    assert_eq!(calls_log(home.path()), "pinned-rustc\npinned-cargo\n");
}

#[cfg(unix)]
#[test]
fn aggregate_executes_default_toolchain_for_commented_settings_default() {
    let workspace = clean_workspace();
    let home =
        fake_rustup_home("default_toolchain = \"stable-x86_64-unknown-linux-gnu\" # the default\n");
    let output = run_in_with_rustup_home(
        workspace.path(),
        &["aggregate", "--scope", "edit", "--emit", "json"],
        home.path(),
    );
    let report = parse_stdout(&output, 0);
    assert_eq!(report["variant"], "Pass");
    assert_eq!(calls_log(home.path()), "default-rustc\ndefault-cargo\n");
}

#[cfg(unix)]
#[test]
fn aggregate_rejects_malformed_toolchain_toml() {
    let workspace = clean_workspace();
    fs::write(
        workspace.path().join("rust-toolchain.toml"),
        "[toolchain]\nchannel = \"nightly-2026-04-27\n",
    )
    .expect("pin file must be written");
    let home = fake_rustup_home("default_toolchain = \"stable-x86_64-unknown-linux-gnu\"\n");
    let output = run_in_with_rustup_home(
        workspace.path(),
        &["aggregate", "--scope", "edit", "--emit", "json"],
        home.path(),
    );
    assert_probe_fails_closed(&output, home.path());
}

#[cfg(unix)]
#[test]
fn aggregate_rejects_channelless_toolchain_toml_despite_legacy_file() {
    let workspace = clean_workspace();
    fs::write(
        workspace.path().join("rust-toolchain.toml"),
        "[toolchain]\ncomponents = [\"rustfmt\"]\n",
    )
    .expect("pin file must be written");
    fs::write(workspace.path().join("rust-toolchain"), "nightly-2026-04-27\n")
        .expect("legacy pin file must be written");
    let home = fake_rustup_home("default_toolchain = \"stable-x86_64-unknown-linux-gnu\"\n");
    let output = run_in_with_rustup_home(
        workspace.path(),
        &["aggregate", "--scope", "edit", "--emit", "json"],
        home.path(),
    );
    assert_probe_fails_closed(&output, home.path());
}

#[cfg(unix)]
#[test]
fn aggregate_rejects_malformed_settings_toml() {
    let workspace = clean_workspace();
    let home = fake_rustup_home("default_toolchain = \"stable-x86_64-unknown-linux-gnu\n");
    let output = run_in_with_rustup_home(
        workspace.path(),
        &["aggregate", "--scope", "edit", "--emit", "json"],
        home.path(),
    );
    assert_probe_fails_closed(&output, home.path());
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
