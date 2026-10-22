use serde_json::Value;
use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::LazyLock,
};

fn binary() -> PathBuf {
    env::var("CARGO_BIN_EXE_titania-check")
        .expect("CARGO_BIN_EXE_titania-check not set — run via `cargo test`")
        .into()
}

fn run_check(cwd: &Path, scope: &str) -> (i32, String, String) {
    let mut cmd = Command::new(binary());
    let _ = cmd.current_dir(cwd);
    let _ = cmd.args(["--scope", scope, "--emit", "json"]);
    let _ = cmd.stdout(Stdio::piped());
    let _ = cmd.stderr(Stdio::piped());
    let _ = cmd.env("TITANIA_MOON_BIN", write_moon_stub(cwd));
    let _ = cmd.env("TITANIA_CHECK_BIN", binary());
    configure_environment(&mut cmd, cwd);
    if let Ok(ctd) = env::var("CARGO_TARGET_DIR") {
        let _ = cmd.env("CARGO_TARGET_DIR", ctd);
    }
    let output = cmd.output().expect("failed to spawn titania-check");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, stdout, stderr)
}

fn configure_environment(command: &mut Command, cwd: &Path) {
    let hermetic_root = cwd.join(".titania").join("hermetic");
    let cargo_home = controlled_home_path(
        &hermetic_root,
        "cargo-home",
        env::var_os("CARGO_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo"))),
    );
    let rustup_home = controlled_home_path(
        &hermetic_root,
        "rustup-home",
        env::var_os("RUSTUP_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".rustup"))),
    );
    let _ = command.env("CARGO_HOME", cargo_home);
    let _ = command.env("RUSTUP_HOME", rustup_home);
    match dylint_library() {
        Some(path) => {
            let _ = command.env("TITANIA_DYLINT_LIB", path);
        }
        None => {
            let _ = command.env_remove("TITANIA_DYLINT_LIB");
        }
    }
}

fn controlled_home_path(root: &Path, suffix: &str, source: Option<PathBuf>) -> PathBuf {
    let controlled = root.join(suffix);
    if !controlled.exists() {
        if let Some(source) = source.filter(|path| path.is_dir()) {
            let _symlink_result = std::os::unix::fs::symlink(source, &controlled);
        }
        if !controlled.exists() {
            std::fs::create_dir_all(&controlled).expect("controlled home must be creatable");
        }
    }
    controlled
}

fn dylint_library() -> Option<PathBuf> {
    static LIBRARY: LazyLock<Option<PathBuf>> = LazyLock::new(resolve_dylint_library);
    LIBRARY.clone()
}

fn resolve_dylint_library() -> Option<PathBuf> {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).ancestors().nth(2)?.to_path_buf();
    let target_dir = resolve_target_dir(&workspace);
    let library_root = target_dir.join("dylint").join("libraries");
    find_dylint_library(&library_root).or_else(|| {
        let status = Command::new("cargo")
            .current_dir(&workspace)
            .args(["dylint", "list", "--all"])
            .status()
            .ok()?;
        if !status.success() {
            return None;
        }
        find_dylint_library(&library_root)
    })
}

fn resolve_target_dir(workspace: &Path) -> PathBuf {
    env::var_os("CARGO_TARGET_DIR").map_or_else(
        || workspace.join("target"),
        |value| {
            let target = PathBuf::from(value);
            if target.is_absolute() { target } else { workspace.join(target) }
        },
    )
}

fn find_dylint_library(root: &Path) -> Option<PathBuf> {
    std::fs::read_dir(root).ok()?.filter_map(Result::ok).find_map(|entry| {
        let path = entry.path();
        if path.is_dir() {
            return find_dylint_library(&path);
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

fn write_moon_stub(dir: &Path) -> String {
    let stub_path = dir.join("moon-all-lanes-stub.sh");
    let script = "#!/bin/sh\n\
         if [ \"$1\" = 'run' ] && [ -n \"$2\" ] && [ -n \"$TITANIA_CHECK_BIN\" ]; then\n\
         \tcase \"$2\" in\n\
         \t\t:titania-fmt)         lane=fmt ;;\n\
         \t\t:titania-compile)     lane=compile ;;\n\
         \t\t:titania-clippy)      lane=clippy ;;\n\
         \t\t:titania-ast-grep)    lane=ast-grep ;;\n\
         \t\t:titania-dylint)      lane=dylint ;;\n\
         \t\t:titania-panic-scan)  lane=panic-scan ;;\n\
         \t\t:titania-policy-scan) lane=policy-scan ;;\n\
         \t\t:titania-test)        lane=test ;;\n\
         \t\t:titania-deny)        lane=deny ;;\n\
         \t\t:titania-build)       lane=build ;;\n\
         \t\t*)                    lane= ;;\n\
         \tesac\n\
         \tif [ -n \"$lane\" ]; then\n\
         \t\t\"$TITANIA_CHECK_BIN\" run-lane \"$lane\" >/dev/null 2>&1 || :\n\
         \tfi\n\
         fi\n\
         exit 0\n";
    std::fs::write(&stub_path, script).expect("moon stub script must be written");
    use std::os::unix::fs::PermissionsExt;
    let mut perms =
        std::fs::metadata(&stub_path).expect("stub metadata must be readable").permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&stub_path, perms).expect("stub must be made executable");
    stub_path.to_str().expect("stub path must be valid UTF-8").to_owned()
}

fn fixture_workspace() -> tempfile::TempDir {
    let mut src = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let _ = src.pop();
    let _ = src.pop();
    src.push("fixtures");
    src.push("compile_failure");
    assert!(src.exists(), "compile_failure fixture must exist at {src:?}");
    let tmp = tempfile::tempdir().expect("tempdir must be created");
    copy_dir(&src, tmp.path()).expect("fixture copy must succeed");
    tmp
}

fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            std::fs::create_dir_all(&dst_path)?;
            copy_dir(&src_path, &dst_path)?;
        } else {
            let _ = std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn lane_outcome<'a>(report: &'a Value, lane: &str) -> &'a Value {
    report["per_lane"]
        .as_array()
        .expect("per_lane is array")
        .iter()
        .find(|entry| entry["lane"] == lane)
        .map(|entry| &entry["outcome"])
        .unwrap_or_else(|| panic!("per_lane must contain {lane}: {}", report["per_lane"]))
}

fn outcome_tag(outcome: &Value) -> &str {
    outcome.as_object().and_then(|object| object.keys().next().map(String::as_str)).unwrap_or("")
}

fn assert_prior_compilation_skip(report: &Value, lane: &str) {
    let outcome = lane_outcome(report, lane);
    assert_eq!(
        outcome,
        &serde_json::json!({ "Skipped": "PriorCompilationFailure" }),
        "{lane} must be Skipped(PriorCompilationFailure), got: {outcome}"
    );
}

fn assert_recorded_clippy_findings(report: &Value) {
    let outcome = lane_outcome(report, "Clippy");
    assert_eq!(
        outcome_tag(outcome),
        "Findings",
        "Clippy outcome must be its recorded Findings, got: {outcome}"
    );
    let findings = outcome["Findings"].as_array().expect("Clippy Findings must be an array");
    assert_eq!(findings.len(), 1, "Clippy must record exactly one finding, got: {findings:?}");
    let finding = &findings[0];
    assert_eq!(finding["lane"], "Clippy", "recorded finding must belong to Clippy: {finding}");
    assert_eq!(
        finding["rule_id"], "CLIPPY_E0308",
        "recorded finding must be CLIPPY_E0308: {finding}"
    );
    assert_eq!(
        finding["location"]["Span"]["file"], "src/lib.rs",
        "recorded finding must point at src/lib.rs: {finding}"
    );
    assert_eq!(
        finding["location"]["Span"]["line_start"], 2,
        "recorded finding must point at line 2: {finding}"
    );
    let code_findings = report["code_findings"].as_array().expect("code_findings is array");
    assert!(
        code_findings
            .iter()
            .any(|entry| entry["lane"] == "Clippy" && entry["rule_id"] == "CLIPPY_E0308"),
        "code_findings must surface the recorded CLIPPY_E0308 finding, got: {code_findings:?}"
    );
}

fn gate_failure_tools(report: &Value) -> Vec<String> {
    report["gate_failures"]
        .as_array()
        .expect("gate_failures is array")
        .iter()
        .filter_map(|failure| {
            failure["ToolFailure"]["tool"]
                .as_str()
                .or_else(|| failure["InfraFailure"]["tool"].as_str())
                .map(str::to_owned)
        })
        .collect()
}

fn assert_no_gate_failure_from(report: &Value, tools: &[&str]) {
    let actual = gate_failure_tools(report);
    for tool in tools {
        assert!(
            !actual.iter().any(|candidate| candidate == tool),
            "no gate failure may be attributable to {tool}, got tools: {actual:?}"
        );
    }
}

fn assert_compile_failure_report(report: &Value) {
    assert_eq!(report["variant"], "Reject", "report variant must be Reject");
    let compile = lane_outcome(report, "Compile");
    assert_eq!(outcome_tag(compile), "Failed", "Compile outcome must be Failed, got: {compile}");
    let ast_grep = lane_outcome(report, "AstGrep");
    assert_eq!(
        outcome_tag(ast_grep),
        "Findings",
        "AstGrep must have run and reported findings, got: {ast_grep}"
    );
    let policy_scan = lane_outcome(report, "PolicyScan");
    let policy_tag = outcome_tag(policy_scan);
    assert!(
        policy_tag == "Clean" || policy_tag == "Findings",
        "PolicyScan must have run, got: {policy_scan}"
    );
    let findings = report["code_findings"].as_array().expect("code_findings is array");
    assert!(
        findings.iter().any(|finding| finding["rule_id"] == "BOUNDED_EXECUTION_UNBOUNDED_LOOP"),
        "code_findings must contain the AstGrep loop finding, got: {findings:?}"
    );
}

#[cfg(unix)]
#[test]
fn edit_scope_preserves_clippy_findings_on_compile_failure() {
    let workspace = fixture_workspace();

    let (code, stdout, stderr) = run_check(workspace.path(), "edit");

    assert_eq!(code, 1, "compile failure must exit 1, stderr: {stderr}");
    assert!(stderr.is_empty(), "no stderr on reject, got: {stderr}");
    let report: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_| panic!("stdout must be valid JSON: {stdout}"));
    assert_compile_failure_report(&report);
    assert_recorded_clippy_findings(&report);
    assert_no_gate_failure_from(&report, &["Clippy", "cargo-clippy"]);
}

#[cfg(unix)]
#[test]
fn prepush_scope_preserves_clippy_findings_and_skips_test_on_compile_failure() {
    let workspace = fixture_workspace();

    let (code, stdout, stderr) = run_check(workspace.path(), "prepush");

    assert_eq!(code, 1, "compile failure must exit 1, stderr: {stderr}");
    assert!(stderr.is_empty(), "no stderr on reject, got: {stderr}");
    let report: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_| panic!("stdout must be valid JSON: {stdout}"));
    assert_compile_failure_report(&report);
    assert_recorded_clippy_findings(&report);
    assert_prior_compilation_skip(&report, "Test");
    assert_no_gate_failure_from(&report, &["Clippy", "cargo-clippy", "Test", "cargo-test"]);
}

#[cfg(unix)]
#[test]
fn release_scope_preserves_clippy_findings_and_skips_test_and_build_on_compile_failure() {
    let workspace = fixture_workspace();

    let (code, stdout, stderr) = run_check(workspace.path(), "release");

    assert_eq!(code, 1, "compile failure must exit 1, stderr: {stderr}");
    assert!(stderr.is_empty(), "no stderr on reject, got: {stderr}");
    let report: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_| panic!("stdout must be valid JSON: {stdout}"));
    assert_compile_failure_report(&report);
    assert_recorded_clippy_findings(&report);
    assert_prior_compilation_skip(&report, "Test");
    assert_prior_compilation_skip(&report, "Build");
    assert_no_gate_failure_from(
        &report,
        &["Clippy", "cargo-clippy", "Test", "cargo-test", "Build", "cargo-build"],
    );
}
