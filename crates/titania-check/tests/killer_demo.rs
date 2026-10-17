use serde_json::Value;
use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::LazyLock,
};

fn binary() -> std::path::PathBuf {
    env::var("CARGO_BIN_EXE_titania-check")
        .expect("CARGO_BIN_EXE_titania-check not set — run via `cargo test`")
        .into()
}

fn run_in(cwd: &Path, args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(binary());
    let _ = cmd.current_dir(cwd);
    let _ = cmd.args(args);
    let _ = cmd.stdout(Stdio::piped());
    let _ = cmd.stderr(Stdio::piped());

    #[cfg(unix)]
    let _ = cmd.env("TITANIA_MOON_BIN", write_moon_stub(cwd, &None));
    let _ = cmd.env("TITANIA_CHECK_BIN", binary());
    #[cfg(unix)]
    configure_killer_environment(&mut cmd, cwd);

    if let Ok(ctd) = env::var("CARGO_TARGET_DIR") {
        let _ = cmd.env("CARGO_TARGET_DIR", ctd);
    }

    let output = cmd.output().expect("failed to spawn titania-check");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, stdout, stderr)
}

fn external_tag(value: &Value) -> Option<&str> {
    value.as_object().and_then(|object| object.keys().next().map(String::as_str))
}

#[cfg(unix)]
fn configure_killer_environment(command: &mut Command, cwd: &Path) {
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

#[cfg(unix)]
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

#[cfg(unix)]
fn dylint_library() -> Option<PathBuf> {
    static LIBRARY: LazyLock<Option<PathBuf>> = LazyLock::new(resolve_dylint_library);
    LIBRARY.clone()
}

#[cfg(unix)]
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

#[cfg(unix)]
fn resolve_target_dir(workspace: &Path) -> PathBuf {
    env::var_os("CARGO_TARGET_DIR").map_or_else(
        || workspace.join("target"),
        |value| {
            let target = PathBuf::from(value);
            if target.is_absolute() { target } else { workspace.join(target) }
        },
    )
}

#[cfg(unix)]
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

#[test]
fn bad_fixture_rejects_with_code_findings() {
    let bad = fixtures::bad_workspace();

    let (code, stdout, stderr) = run_in(bad.path(), &["--scope", "edit", "--emit", "json"]);

    assert_eq!(code, 1, "reject must exit 1, stderr: {stderr}");
    assert!(stderr.is_empty(), "no stderr on reject, got: {stderr}");
    let report: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_| panic!("stdout must be valid JSON: {stdout}"));

    assert_eq!(report["variant"], "Reject", "report variant must be Reject");

    let findings = report["code_findings"].as_array().expect("code_findings is array");
    assert!(
        findings.len() >= 2,
        "expected required code findings, got {}: {findings:?}",
        findings.len()
    );
    let gates = report["gate_failures"].as_array().expect("gate_failures is array");
    assert_eq!(
        gates.len(),
        0,
        "expected 0 gate_failures for bad fixture, got {}: {gates:?}",
        gates.len()
    );

    let per_lane = report["per_lane"].as_array().expect("per_lane is array");
    assert_eq!(
        per_lane.len(),
        7,
        "expected 7 per_lane outcomes, got {}: {per_lane:?}",
        per_lane.len()
    );

    let rule_ids: Vec<&str> = findings.iter().filter_map(|f| f["rule_id"].as_str()).collect();
    assert!(
        rule_ids.contains(&"BOUNDED_EXECUTION_UNBOUNDED_LOOP"),
        "must contain BOUNDED_EXECUTION_UNBOUNDED_LOOP, got: {rule_ids:?}"
    );
    assert!(
        rule_ids.contains(&"CLIPPY_UNWRAP_USED"),
        "must contain CLIPPY_UNWRAP_USED, got: {rule_ids:?}"
    );
}

#[test]
fn bad_fixture_has_func_loops_for_finding() {
    let bad = fixtures::bad_workspace();

    let (code, stdout, _stderr) = run_in(bad.path(), &["--scope", "edit", "--emit", "json"]);
    assert_eq!(code, 1, "must reject");
    let report: Value = serde_json::from_str(&stdout).expect("stdout must be valid JSON");
    let findings = report["code_findings"].as_array().expect("code_findings");
    let loop_finding = findings
        .iter()
        .find(|f| f["rule_id"] == "BOUNDED_EXECUTION_UNBOUNDED_LOOP" && f["lane"] == "AstGrep")
        .expect("must find BOUNDED_EXECUTION_UNBOUNDED_LOOP finding from AstGrep");

    assert_eq!(
        loop_finding["lane"].as_str(),
        Some("AstGrep"),
        "BOUNDED_EXECUTION_UNBOUNDED_LOOP must come from AstGrep lane"
    );
    assert_eq!(
        loop_finding["effect"].as_str(),
        Some("Reject"),
        "BOUNDED_EXECUTION_UNBOUNDED_LOOP finding must have effect Reject"
    );
    assert_eq!(
        external_tag(&loop_finding["repair"]),
        Some("RequiresHumanReview"),
        "BOUNDED_EXECUTION_UNBOUNDED_LOOP repair hint must be RequiresHumanReview"
    );
}

#[test]
fn bad_fixture_has_clippy_unwrap_used_finding() {
    let bad = fixtures::bad_workspace();

    let (code, stdout, _stderr) = run_in(bad.path(), &["--scope", "edit", "--emit", "json"]);
    assert_eq!(code, 1, "must reject");
    let report: Value = serde_json::from_str(&stdout).expect("stdout must be valid JSON");

    let findings = report["code_findings"].as_array().expect("code_findings");
    let unwrap_finding = findings
        .iter()
        .find(|f| f["rule_id"] == "CLIPPY_UNWRAP_USED")
        .expect("must find CLIPPY_UNWRAP_USED finding");

    assert_eq!(
        unwrap_finding["lane"].as_str(),
        Some("Clippy"),
        "CLIPPY_UNWRAP_USED must come from Clippy lane"
    );
    assert_eq!(
        unwrap_finding["effect"].as_str(),
        Some("Reject"),
        "CLIPPY_UNWRAP_USED finding must have effect Reject"
    );
    assert_eq!(
        external_tag(&unwrap_finding["repair"]),
        Some("RequiresHumanReview"),
        "CLIPPY_UNWRAP_USED repair hint must be RequiresHumanReview"
    );
}

#[test]
fn bad_fixture_findings_have_correct_repair_hints() {
    let bad = fixtures::bad_workspace();

    let (code, stdout, _stderr) = run_in(bad.path(), &["--scope", "edit", "--emit", "json"]);
    assert_eq!(code, 1, "must reject");
    let report: Value = serde_json::from_str(&stdout).expect("stdout must be valid JSON");

    let findings = report["code_findings"].as_array().expect("code_findings");

    let mut found_for_hint = false;
    let mut found_unwrap_hint = false;

    for f in findings {
        let rule = f["rule_id"].as_str().unwrap_or("");
        let hint = external_tag(&f["repair"]).unwrap_or("");
        match rule {
            "BOUNDED_EXECUTION_UNBOUNDED_LOOP" => {
                found_for_hint |= hint == "RequiresHumanReview";
            }
            "CLIPPY_UNWRAP_USED" => {
                found_unwrap_hint |= hint == "RequiresHumanReview";
            }
            _ => {}
        }
    }

    assert!(
        found_for_hint,
        "BOUNDED_EXECUTION_UNBOUNDED_LOOP must have requires_human_review repair hint"
    );
    assert!(found_unwrap_hint, "CLIPPY_UNWRAP_USED must have requires_human_review repair hint");
}

#[test]
fn bad_fixture_gate_failures_empty() {
    let bad = fixtures::bad_workspace();

    let (code, stdout, _stderr) = run_in(bad.path(), &["--scope", "edit", "--emit", "json"]);
    assert_eq!(code, 1, "must reject");
    let report: Value = serde_json::from_str(&stdout).expect("stdout must be valid JSON");

    let gates = report["gate_failures"].as_array().expect("gate_failures");
    assert_eq!(gates.len(), 0, "gate_failures must be empty for bad fixture, got: {gates:?}");

    let per_lane = report["per_lane"].as_array().expect("per_lane");
    for (i, outcome) in per_lane.iter().enumerate() {
        let variant = external_tag(&outcome["outcome"]).unwrap_or("missing");
        assert_ne!(variant, "Failed", "lane {i} ({variant}) must not be Failed for bad fixture");
    }
}

#[test]
fn bad_fixture_reject_kind_is_code_only() {
    let bad = fixtures::bad_workspace();

    let (code, stdout, _stderr) = run_in(bad.path(), &["--scope", "edit", "--emit", "json"]);
    assert_eq!(code, 1, "must reject");
    let report: Value = serde_json::from_str(&stdout).expect("stdout must be valid JSON");

    let findings = report["code_findings"].as_array().expect("code_findings");
    let gates = report["gate_failures"].as_array().expect("gate_failures");
    assert!(!findings.is_empty(), "code_findings must be non-empty for CodeOnly");
    assert!(gates.is_empty(), "gate_failures must be empty for CodeOnly");

    let typed_report: titania_core::Report =
        serde_json::from_str(&stdout).expect("report must deserialize as Report");
    let kind = typed_report.reject_kind().expect("reject report must have reject_kind");
    assert_eq!(kind, titania_core::RejectKind::CodeOnly, "reject_kind must be CodeOnly");
}

#[test]
fn repaired_fixture_passes_with_receipt() {
    let repaired = fixtures::repaired_workspace();

    let (code, stdout, stderr) = run_in(repaired.path(), &["--scope", "edit", "--emit", "json"]);

    assert_eq!(code, 0, "pass must exit 0, stderr: {stderr}");
    assert!(stderr.is_empty(), "no stderr on pass, got: {stderr}");
    let report: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_| panic!("stdout must be valid JSON: {stdout}"));

    assert_eq!(report["variant"], "Pass", "report variant must be Pass for repaired fixture");
}

#[test]
fn repaired_fixture_receipt_has_schema_version_one() {
    let repaired = fixtures::repaired_workspace();

    let (code, stdout, _stderr) = run_in(repaired.path(), &["--scope", "edit", "--emit", "json"]);
    assert_eq!(code, 0, "must pass");
    let report: Value = serde_json::from_str(&stdout).expect("stdout must be valid JSON");

    assert_eq!(report["receipt"]["schema_version"], 1, "receipt schema_version must be 1");
    assert_eq!(report["receipt"]["scope"], "Edit", "receipt scope must be Edit");
}

#[test]
fn repaired_fixture_receipt_contains_all_digests() {
    let repaired = fixtures::repaired_workspace();

    let (code, stdout, _stderr) = run_in(repaired.path(), &["--scope", "edit", "--emit", "json"]);
    assert_eq!(code, 0, "must pass");
    let report: Value = serde_json::from_str(&stdout).expect("stdout must be valid JSON");

    let receipt = &report["receipt"];
    let digests = [
        ("source_digest", receipt["source_digest"].as_str()),
        ("cargo_lock_digest", receipt["cargo_lock_digest"].as_str()),
        ("policy_digest", receipt["policy_digest"].as_str()),
        ("toolchain_digest", receipt["toolchain_digest"].as_str()),
    ];

    for (name, opt_str) in digests {
        let hex = opt_str.expect(&format!("{name} must be a string"));
        assert_eq!(
            hex.len(),
            64,
            "{name} must be 64-char hex, got len {} and value: {hex}",
            hex.len()
        );
        assert!(
            hex.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')),
            "{name} must be lowercase hex, got: {hex}"
        );
    }

    let values: Vec<&str> = digests.iter().filter_map(|(_, s)| *s).collect();
    for i in 0..values.len() {
        for j in (i + 1)..values.len() {
            assert_ne!(values[i], values[j], "digests must differ: {} vs {}", values[i], values[j]);
        }
    }
}

#[test]
fn repaired_fixture_per_lane_has_seven_entries() {
    let repaired = fixtures::repaired_workspace();

    let (code, stdout, _stderr) = run_in(repaired.path(), &["--scope", "edit", "--emit", "json"]);
    assert_eq!(code, 0, "must pass");
    let report: Value = serde_json::from_str(&stdout).expect("stdout must be valid JSON");

    let per_lane = report["per_lane"].as_array().expect("per_lane");
    assert_eq!(
        per_lane.len(),
        7,
        "per_lane must have 7 entries, got {}: {per_lane:?}",
        per_lane.len()
    );

    for (i, outcome) in per_lane.iter().enumerate() {
        let variant = external_tag(&outcome["outcome"]).unwrap_or("missing");
        assert!(
            variant == "Clean" || variant == "Skipped",
            "lane {i} variant must be Clean or Skipped, got: {variant}"
        );
    }
}

#[test]
fn bounded_for_with_question_passes() {
    let workspace = fixtures::repaired_workspace();
    let lib = workspace.path().join("src").join("lib.rs");
    let mut source = std::fs::read_to_string(&lib).expect("repaired lib.rs must be readable");
    source.push_str(
        "\n/// Bounded iteration: finite input, typed failure stays observable.\n///\n/// # Errors\n///\n/// Returns the first negative-input failure without discarding its cause.\npub fn bounded_total(items: &[i32]) -> Result<Vec<i32>, &'static str> {\n    \
         let mut out = Vec::new();\n    \
         for value in items {\n        \
         out.push(checked(*value)?);\n    }\n    \
         Ok(out)\n}\n\nconst fn checked(value: i32) -> Result<i32, &'static str> {\n    \
         if value < 0 {\n        return Err(\"negative input\");\n    }\n    \
         Ok(value)\n}\n",
    );
    std::fs::write(&lib, source).expect("bounded function must be appendable");

    let (code, stdout, stderr) = run_in(workspace.path(), &["--scope", "edit", "--emit", "json"]);

    let stdout_head: String = stdout.chars().take(2000).collect();
    assert_eq!(code, 0, "bounded for must pass, stderr: {stderr}, stdout: {stdout_head}");
    let report: Value = serde_json::from_str(&stdout).expect("stdout must be valid JSON");
    assert_eq!(report["variant"], "Pass", "bounded for report must be Pass");
    let empty = Vec::new();
    let findings = report["code_findings"].as_array().unwrap_or(&empty);
    assert!(
        findings
            .iter()
            .all(|f| !f["rule_id"].as_str().unwrap_or("").starts_with("BOUNDED_EXECUTION")),
        "bounded for must emit no BOUNDED_EXECUTION finding, got: {findings:?}"
    );
}

#[test]
fn repaired_fixture_per_lane_contains_all_edit_lanes() {
    let repaired = fixtures::repaired_workspace();

    let (code, stdout, _stderr) = run_in(repaired.path(), &["--scope", "edit", "--emit", "json"]);
    assert_eq!(code, 0, "must pass");
    let report: Value = serde_json::from_str(&stdout).expect("stdout must be valid JSON");

    let per_lane = report["per_lane"].as_array().expect("per_lane");
    let lane_names: Vec<&str> = per_lane.iter().filter_map(|o| o["lane"].as_str()).collect();

    let expected = ["Fmt", "Compile", "Clippy", "AstGrep", "Dylint", "PanicScan", "PolicyScan"];
    for exp in &expected {
        assert!(lane_names.contains(exp), "per_lane must contain lane {exp}, got: {lane_names:?}");
    }
    assert_eq!(
        lane_names.len(),
        7,
        "must have exactly 7 lanes, got {}: {lane_names:?}",
        lane_names.len()
    );
}

#[test]
fn missing_cargo_toml_produces_reject_report() {
    let empty = tempfile::tempdir().expect("tempdir must be created");

    let (code, stdout, stderr) = run_in(empty.path(), &["--scope", "edit", "--emit", "json"]);

    assert_eq!(
        code, 1,
        "missing Cargo.toml in empty workspace must exit 1 (reject), got {}: stdout={stdout}, stderr={stderr}",
        code
    );
    assert!(stderr.is_empty(), "reject must not write stderr, got: {stderr}");
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("stdout must be JSON");
    assert_eq!(report["variant"], "Reject");
}

#[test]
fn mixed_report_separates_code_findings_from_gate_failures() {
    let json = r#"{
        "variant": "Reject",
        "code_findings": [
            {
                "lane": "AstGrep",
                "rule_id": "FUNC_LOOPS_FOR",
                "location": "Workspace",
                "message": "for loop detected",
                "repair": {"UseIteratorPipeline": {"suggestion": "use iterator"}},
                "effect": "Reject"
            },
            {
                "lane": "Clippy",
                "rule_id": "CLIPPY_UNWRAP_USED",
                "location": "Workspace",
                "message": "unwrap used",
                "repair": {"RequiresHumanReview": {"note": "manual fix needed"}},
                "effect": "Reject"
            }
        ],
        "gate_failures": [
            {
                "InfraFailure": {
                    "tool": "Dylint",
                    "reason": "binary not found"
                }
            }
        ],
        "per_lane": [{"lane": "Fmt", "outcome": {"Skipped": "NotApplicable"}}]
    }"#;

    let report: Result<titania_core::Report, _> = serde_json::from_str(json);

    let report = report.expect("must deserialize Reject from JSON");
    assert!(report.is_reject(), "must be Reject");
    let code_findings = report.code_findings().expect("Reject must have code_findings");
    let gate_failures = report.gate_failures().expect("Reject must have gate_failures");
    assert_eq!(code_findings.len(), 2, "must have 2 code findings");
    assert_eq!(gate_failures.len(), 1, "must have 1 gate failure");

    let kind = report.reject_kind().expect("reject must have reject_kind");
    assert_eq!(kind, titania_core::RejectKind::Mixed, "reject_kind must be Mixed");

    let code_lanes: Vec<&str> = {
        let cf = report.code_findings().expect("Reject must have code_findings");
        cf.iter().map(|f| f.lane().name()).collect()
    };

    for lane_name in &code_lanes {
        assert!(
            matches!(*lane_name, "AstGrep" | "Clippy"),
            "code finding lane must be functional, got: {lane_name}"
        );
    }
}

#[test]
fn report_reject_gate_only() {
    let json = r#"{
        "variant": "Reject",
        "code_findings": [],
        "gate_failures": [
            {
                "InfraFailure": {
                    "tool": "Dylint",
                    "reason": "output file missing"
                }
            }
        ],
        "per_lane": [{"lane": "Fmt", "outcome": {"Skipped": "NotApplicable"}}]
    }"#;

    let report: titania_core::Report =
        serde_json::from_str(json).expect("must deserialize Reject from JSON");

    let kind = report.reject_kind().expect("reject must have reject_kind");
    assert_eq!(kind, titania_core::RejectKind::GateOnly, "reject_kind must be GateOnly");

    assert!(report.is_reject(), "must be Reject");
    let code_findings = report.code_findings().expect("Reject must have code_findings");
    let gate_failures = report.gate_failures().expect("Reject must have gate_failures");
    assert_eq!(code_findings.len(), 0, "code_findings must be empty");
    assert_eq!(gate_failures.len(), 1, "gate_failures must have 1 entry");
}

#[test]
fn report_reject_mixed() {
    let json = r#"{
        "variant": "Reject",
        "code_findings": [
            {
                "lane": "AstGrep",
                "rule_id": "FUNC_LOOPS_FOR",
                "location": "Workspace",
                "message": "for loop detected",
                "repair": {"UseIteratorPipeline": {"suggestion": "use iterator"}},
                "effect": "Reject"
            }
        ],
        "gate_failures": [
            {
                "InfraFailure": {
                    "tool": "Compile",
                    "reason": "compilation failed"
                }
            }
        ],
        "per_lane": [{"lane": "Fmt", "outcome": {"Skipped": "NotApplicable"}}]
    }"#;

    let report: titania_core::Report =
        serde_json::from_str(json).expect("must deserialize Reject from JSON");

    let kind = report.reject_kind().expect("reject must have reject_kind");
    assert_eq!(kind, titania_core::RejectKind::Mixed, "reject_kind must be Mixed");

    assert!(report.is_reject(), "must be Reject");
    let code_findings = report.code_findings().expect("Reject must have code_findings");
    let gate_failures = report.gate_failures().expect("Reject must have gate_failures");
    assert_eq!(code_findings.len(), 1, "must have 1 code finding");
    assert_eq!(gate_failures.len(), 1, "must have 1 gate failure");
}

mod fixtures {
    use std::path::{Path, PathBuf};

    fn repo_root() -> PathBuf {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let _ = p.pop();
        let _ = p.pop();
        p
    }

    pub(super) fn bad_path() -> PathBuf {
        let mut p = repo_root();
        p.push("fixtures");
        p.push("strict_ai_loop_unwrap");
        p.push("bad");
        p
    }

    pub(super) fn repaired_path() -> PathBuf {
        let mut p = repo_root();
        p.push("fixtures");
        p.push("strict_ai_loop_unwrap");
        p.push("repaired");
        p
    }

    pub(super) fn bad_workspace() -> tempfile::TempDir {
        let src = bad_path();
        assert!(
            src.exists(),
            "bad fixture must exist at {src:?} — run the fixture-creation step first"
        );
        let tmp = tempfile::tempdir().expect("tempdir must be created");
        copy_dir(&src, tmp.path()).expect("fixture copy must succeed");
        tmp
    }

    pub(super) fn repaired_workspace() -> tempfile::TempDir {
        let src = repaired_path();
        assert!(
            src.exists(),
            "repaired fixture must exist at {src:?} — run the fixture-creation step first"
        );
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
}

#[cfg(unix)]
#[test]
fn check_drives_moon_stub_and_aggregates_after() {
    let workspace = tempfile::tempdir().expect("tempdir must be created");

    let marker = workspace.path().join("moon_argv.txt");
    let stub_path = write_recording_stub(workspace.path(), &marker);

    let mut cmd = Command::new(binary());
    let _ = cmd.current_dir(workspace.path());
    let _ = cmd.args(&["--scope", "edit", "--emit", "json"]);
    let _ = cmd.stdout(Stdio::piped());
    let _ = cmd.stderr(Stdio::piped());
    let _ = cmd.env("TITANIA_MOON_BIN", &stub_path);
    if let Ok(ctd) = env::var("CARGO_TARGET_DIR") {
        let _ = cmd.env("CARGO_TARGET_DIR", ctd);
    }
    let output = cmd.output().expect("failed to spawn titania-check");

    assert!(
        marker.exists(),
        "check must invoke moon (marker file should exist at {})",
        marker.display(),
    );

    let argv = std::fs::read_to_string(&marker).expect("moon argv marker file must be readable");
    assert!(argv.contains("run"), "moon must be invoked with `run` subcommand; got argv: {argv}",);
    assert!(
        argv.contains(":titania-fmt"),
        "moon must be invoked with :titania-fmt; got argv: {argv}",
    );
    assert!(
        argv.contains(":titania-policy-scan"),
        "moon must be invoked with :titania-policy-scan; got argv: {argv}",
    );

    let code = output.status.code().unwrap_or(-1);
    assert_eq!(
        code, 1,
        "check after moon-stub must run aggregate (exit 1 reject on empty workspace), got {code}",
    );
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let report: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_| panic!("check must emit JSON report after moon; got: {stdout}"));
    assert_eq!(report["variant"], "Reject", "empty workspace aggregate must be Reject");
}

#[cfg(unix)]
fn write_recording_stub(dir: &Path, marker: &Path) -> String {
    let stub_path = dir.join("moon-recording-stub.sh");
    let script = format!("#!/bin/sh\nprintf '%s\\n' \"$@\" >> '{}'\nexit 0\n", marker.display(),);
    std::fs::write(&stub_path, script).expect("recording stub script must be written");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms =
            std::fs::metadata(&stub_path).expect("stub metadata must be readable").permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&stub_path, perms).expect("stub must be made executable");
    }
    stub_path.to_str().expect("stub path must be valid UTF-8").to_owned()
}
#[cfg(unix)]
fn write_moon_stub(dir: &Path, marker: &Option<&Path>) -> String {
    let stub_path = dir.join("moon-fixture-stub.sh");
    let marker_redir = match marker {
        Some(path) => format!(">> '{}' 2>/dev/null || :", path.display()),
        None => String::from(">> /dev/null 2>&1 || :"),
    };

    let script = format!(
        "#!/bin/sh\n\
         \n\
         # Record argv (best-effort).\n\
         for arg in \"$@\"; do printf '%s\\n' \"$arg\" {marker_redir}; done\n\
         printf '%s\\n' '---' {marker_redir}\n\
         \n\
         # Dispatch `run <task-id>` to a fresh run-lane invocation so Check's\n\
         # subsequent aggregate reads fresh artifacts. Unknown task ids are\n\
         # ignored to stay forward-compatible with future scope additions.\n\
         # Lane stderr is suppressed and the lane exit code is intentionally\n\
         # ignored (finding lanes legitimately return 1); the stub always\n\
         # exits 0 so Moon treats every task as successful and Check can\n\
         # aggregate the fresh artifacts written by run-lane.\n\
         if [ \"$1\" = 'run' ] && [ -n \"$2\" ] && [ -n \"$TITANIA_CHECK_BIN\" ]; then\n\
         \tcase \"$2\" in\n\
         \t\t:titania-fmt)         lane=fmt ;;\n\
         \t\t:titania-compile)     lane=compile ;;\n\
         \t\t:titania-clippy)      lane=clippy ;;\n\
         \t\t:titania-ast-grep)    lane=ast-grep ;;\n\
         \t\t:titania-dylint)      lane=dylint ;;\n\
         \t\t:titania-panic-scan)  lane=panic-scan ;;\n\
         \t\t:titania-policy-scan) lane=policy-scan ;;\n\
         \t\t*)                    lane= ;;\n\
         \tesac\n\
         \tif [ -n \"$lane\" ]; then\n\
         \t\t\"$TITANIA_CHECK_BIN\" run-lane \"$lane\" >/dev/null 2>&1 || :\n\
         \tfi\n\
         fi\n\
         \n\
         exit 0\n",
        marker_redir = marker_redir,
    );
    std::fs::write(&stub_path, script).expect("moon stub script must be written");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms =
            std::fs::metadata(&stub_path).expect("stub metadata must be readable").permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&stub_path, perms).expect("stub must be made executable");
    }
    stub_path.to_str().expect("stub path must be valid UTF-8").to_owned()
}
