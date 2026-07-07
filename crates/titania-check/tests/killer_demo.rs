//! Failing-first behavior tests for bead `tn-pdn`.
//!
//! Killer demo: bad code (for-loop + `.unwrap()`) is rejected with code findings;
//! repaired code (iterator pipeline, no unwrap) passes with a v1 receipt.
//!
//! Selective filter: `cargo test -p titania-check --test killer_demo bad_fixture`

use serde_json::Value;
use std::{
    env,
    path::Path,
    process::{Command, Stdio},
};

fn binary() -> std::path::PathBuf {
    env::var("CARGO_BIN_EXE_titania-check")
        .expect("CARGO_BIN_EXE_titania-check not set — run via `cargo test`")
        .into()
}

fn run_in(cwd: &Path, args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(binary());
    let _ = cmd.current_dir(cwd);
    let normalized_args = aggregate_args(args);
    let _ = cmd.args(&normalized_args);
    let _ = cmd.stdout(Stdio::piped());
    let _ = cmd.stderr(Stdio::piped());

    // Stub Moon via TITANIA_MOON_BIN so `Command::Check` does not invoke the
    // real moon binary (which would error on tempdirs without `.moon/`).
    // `/bin/true` exits 0 with any args. The moon-dispatch integration test
    // (`check_drives_moon_stub` below) sets its own recording stub.
    let _ = cmd.env("TITANIA_MOON_BIN", "/bin/true");

    // Pass CARGO_TARGET_DIR through as-is so that library_is_available
    // can resolve it relative to the workspace root when walking up
    // from CARGO_MANIFEST_DIR.  Converting to absolute here would
    // anchor it to the parent's cwd (often a crate subdirectory),
    // which breaks dylint library discovery.
    if let Ok(ctd) = env::var("CARGO_TARGET_DIR") {
        let _ = cmd.env("CARGO_TARGET_DIR", ctd);
    }

    let output = cmd.output().expect("failed to spawn titania-check");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, stdout, stderr)
}

fn aggregate_args<'a>(args: &'a [&'a str]) -> Vec<&'a str> {
    if args.first().is_some_and(|arg| *arg == "--scope") {
        return std::iter::once("aggregate").chain(args.iter().copied()).collect();
    }
    args.to_vec()
}

fn external_tag(value: &Value) -> Option<&str> {
    value.as_object().and_then(|object| object.keys().next().map(String::as_str))
}

// ---------------------------------------------------------------------------
// E2E: Bad Fixture — Report::Reject with code findings
// ---------------------------------------------------------------------------

/// AC-1: A bad fixture (for-loop + `.unwrap()`) is rejected with exactly two code findings.
#[test]
fn bad_fixture_rejects_with_code_findings() {
    // Given: the bad fixture workspace
    let bad = fixtures::bad_workspace();

    // When: titania-check --scope edit --emit json runs against it
    let (code, stdout, stderr) = run_in(bad.path(), &["--scope", "edit", "--emit", "json"]);

    // Then: exit code 1, JSON report with variant Reject
    assert_eq!(code, 1, "reject must exit 1, stderr: {stderr}");
    assert!(stderr.is_empty(), "no stderr on reject, got: {stderr}");
    let report: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_| panic!("stdout must be valid JSON: {stdout}"));

    assert_eq!(report["variant"], "Reject", "report variant must be Reject");

    // Exactly 2 code findings
    let findings = report["code_findings"].as_array().expect("code_findings is array");
    assert_eq!(findings.len(), 2, "expected 2 code_findings, got {}: {findings:?}", findings.len());

    // Exactly 0 gate failures
    let gates = report["gate_failures"].as_array().expect("gate_failures is array");
    assert_eq!(
        gates.len(),
        0,
        "expected 0 gate_failures for bad fixture, got {}: {gates:?}",
        gates.len()
    );

    // 7 per-lane outcomes
    let per_lane = report["per_lane"].as_array().expect("per_lane is array");
    assert_eq!(
        per_lane.len(),
        7,
        "expected 7 per_lane outcomes, got {}: {per_lane:?}",
        per_lane.len()
    );

    // Exact rule IDs
    let rule_ids: Vec<&str> = findings.iter().filter_map(|f| f["rule_id"].as_str()).collect();
    assert!(rule_ids.contains(&"FUNC_LOOPS_FOR"), "must contain FUNC_LOOPS_FOR, got: {rule_ids:?}");
    assert!(
        rule_ids.contains(&"CLIPPY_UNWRAP_USED"),
        "must contain CLIPPY_UNWRAP_USED, got: {rule_ids:?}"
    );
}

/// AC-1 (detail): The FUNC_LOOPS_FOR finding has the correct lane, effect, and repair hint.
#[test]
fn bad_fixture_has_func_loops_for_finding() {
    // Given: the bad fixture workspace
    let bad = fixtures::bad_workspace();

    // When: titania-check runs
    let (code, stdout, _stderr) = run_in(bad.path(), &["--scope", "edit", "--emit", "json"]);
    assert_eq!(code, 1, "must reject");
    let report: Value = serde_json::from_str(&stdout).expect("stdout must be valid JSON");

    // Then: find the FUNC_LOOPS_FOR finding
    let findings = report["code_findings"].as_array().expect("code_findings");
    let for_finding = findings
        .iter()
        .find(|f| f["rule_id"] == "FUNC_LOOPS_FOR")
        .expect("must find FUNC_LOOPS_FOR finding");

    assert_eq!(
        for_finding["lane"].as_str(),
        Some("AstGrep"),
        "FUNC_LOOPS_FOR must come from AstGrep lane"
    );
    assert_eq!(
        for_finding["effect"].as_str(),
        Some("Reject"),
        "FUNC_LOOPS_FOR finding must have effect Reject"
    );
    assert_eq!(
        external_tag(&for_finding["repair"]),
        Some("UseIteratorPipeline"),
        "FUNC_LOOPS_FOR repair hint must be UseIteratorPipeline"
    );
}

/// AC-1 (detail): The CLIPPY_UNWRAP_USED finding has the correct lane, effect, and repair hint.
#[test]
fn bad_fixture_has_clippy_unwrap_used_finding() {
    // Given: the bad fixture workspace
    let bad = fixtures::bad_workspace();

    // When: titania-check runs
    let (code, stdout, _stderr) = run_in(bad.path(), &["--scope", "edit", "--emit", "json"]);
    assert_eq!(code, 1, "must reject");
    let report: Value = serde_json::from_str(&stdout).expect("stdout must be valid JSON");

    // Then: find the CLIPPY_UNWRAP_USED finding
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

/// AC-2: Both findings have the correct RepairHint variant.
#[test]
fn bad_fixture_findings_have_correct_repair_hints() {
    // Given: the bad fixture workspace
    let bad = fixtures::bad_workspace();

    // When: titania-check runs
    let (code, stdout, _stderr) = run_in(bad.path(), &["--scope", "edit", "--emit", "json"]);
    assert_eq!(code, 1, "must reject");
    let report: Value = serde_json::from_str(&stdout).expect("stdout must be valid JSON");

    // Then: each finding's repair variant matches the expected hint
    let findings = report["code_findings"].as_array().expect("code_findings");

    let mut found_for_hint = false;
    let mut found_unwrap_hint = false;

    for f in findings {
        let rule = f["rule_id"].as_str().unwrap_or("");
        let hint = external_tag(&f["repair"]).unwrap_or("");
        match rule {
            "FUNC_LOOPS_FOR" => {
                found_for_hint = hint == "UseIteratorPipeline";
            }
            "CLIPPY_UNWRAP_USED" => {
                found_unwrap_hint = hint == "RequiresHumanReview";
            }
            _ => {}
        }
    }

    assert!(found_for_hint, "FUNC_LOOPS_FOR must have use_iterator_pipeline repair hint");
    assert!(found_unwrap_hint, "CLIPPY_UNWRAP_USED must have requires_human_review repair hint");
}

/// AC-4: Gate failures are empty for the bad fixture; no lane outcome is "failed".
#[test]
fn bad_fixture_gate_failures_empty() {
    // Given: the bad fixture workspace
    let bad = fixtures::bad_workspace();

    // When: titania-check runs
    let (code, stdout, _stderr) = run_in(bad.path(), &["--scope", "edit", "--emit", "json"]);
    assert_eq!(code, 1, "must reject");
    let report: Value = serde_json::from_str(&stdout).expect("stdout must be valid JSON");

    // Then: gate_failures is empty
    let gates = report["gate_failures"].as_array().expect("gate_failures");
    assert_eq!(gates.len(), 0, "gate_failures must be empty for bad fixture, got: {gates:?}");

    // All per_lane outcomes must be Clean, Findings, or Skipped — none Failed.
    let per_lane = report["per_lane"].as_array().expect("per_lane");
    for (i, outcome) in per_lane.iter().enumerate() {
        let variant = external_tag(&outcome["outcome"]).unwrap_or("missing");
        assert_ne!(variant, "Failed", "lane {i} ({variant}) must not be Failed for bad fixture");
    }
}

/// AC-5: RejectKind is CodeOnly for the bad fixture (code findings present, gate failures empty).
#[test]
fn bad_fixture_reject_kind_is_code_only() {
    // Given: the bad fixture workspace
    let bad = fixtures::bad_workspace();

    // When: titania-check runs
    let (code, stdout, _stderr) = run_in(bad.path(), &["--scope", "edit", "--emit", "json"]);
    assert_eq!(code, 1, "must reject");
    let report: Value = serde_json::from_str(&stdout).expect("stdout must be valid JSON");

    // Then: code_findings non-empty AND gate_failures empty = CodeOnly
    let findings = report["code_findings"].as_array().expect("code_findings");
    let gates = report["gate_failures"].as_array().expect("gate_failures");
    assert!(!findings.is_empty(), "code_findings must be non-empty for CodeOnly");
    assert!(gates.is_empty(), "gate_failures must be empty for CodeOnly");

    // Also assert the report parses as Report type and reject_kind() == CodeOnly
    let typed_report: titania_core::Report =
        serde_json::from_str(&stdout).expect("report must deserialize as Report");
    let kind = typed_report.reject_kind().expect("reject report must have reject_kind");
    assert_eq!(kind, titania_core::RejectKind::CodeOnly, "reject_kind must be CodeOnly");
}

// ---------------------------------------------------------------------------
// E2E: Repaired Fixture — Report::Pass with v1 receipt
// ---------------------------------------------------------------------------

/// AC-3: Repaired code (iterator pipeline, no unwrap) passes with a v1 receipt.
#[test]
fn repaired_fixture_passes_with_receipt() {
    // Given: the repaired fixture workspace
    let repaired = fixtures::repaired_workspace();

    // When: titania-check --scope edit --emit json runs against it
    let (code, stdout, stderr) = run_in(repaired.path(), &["--scope", "edit", "--emit", "json"]);

    // Then: exit code 0, report variant is Pass
    assert_eq!(code, 0, "pass must exit 0, stderr: {stderr}");
    assert!(stderr.is_empty(), "no stderr on pass, got: {stderr}");
    let report: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_| panic!("stdout must be valid JSON: {stdout}"));

    assert_eq!(report["variant"], "Pass", "report variant must be Pass for repaired fixture");
}

/// AC-3 (detail): The pass receipt has schema_version == 1 and scope == Edit.
#[test]
fn repaired_fixture_receipt_has_schema_version_one() {
    // Given: the repaired fixture workspace
    let repaired = fixtures::repaired_workspace();

    // When: titania-check runs
    let (code, stdout, _stderr) = run_in(repaired.path(), &["--scope", "edit", "--emit", "json"]);
    assert_eq!(code, 0, "must pass");
    let report: Value = serde_json::from_str(&stdout).expect("stdout must be valid JSON");

    // Then: schema_version == 1, scope == "Edit"
    assert_eq!(report["receipt"]["schema_version"], 1, "receipt schema_version must be 1");
    assert_eq!(report["receipt"]["scope"], "Edit", "receipt scope must be Edit");
}

/// AC-6: The pass receipt contains all four digests as 64-char hex strings.
#[test]
fn repaired_fixture_receipt_contains_all_digests() {
    // Given: the repaired fixture workspace
    let repaired = fixtures::repaired_workspace();

    // When: titania-check runs
    let (code, stdout, _stderr) = run_in(repaired.path(), &["--scope", "edit", "--emit", "json"]);
    assert_eq!(code, 0, "must pass");
    let report: Value = serde_json::from_str(&stdout).expect("stdout must be valid JSON");

    // Then: each digest field is a non-empty string of length 64
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
            hex.chars().all(|c| c.is_ascii_hexdigit()),
            "{name} must be lowercase hex, got: {hex}"
        );
    }

    // All four digests must be different from each other
    let values: Vec<&str> = digests.iter().filter_map(|(_, s)| *s).collect();
    for i in 0..values.len() {
        for j in (i + 1)..values.len() {
            assert_ne!(values[i], values[j], "digests must differ: {} vs {}", values[i], values[j]);
        }
    }
}

/// AC-7 (detail): per_lane has exactly 7 entries, all Clean or Skipped.
#[test]
fn repaired_fixture_per_lane_has_seven_entries() {
    // Given: the repaired fixture workspace
    let repaired = fixtures::repaired_workspace();

    // When: titania-check runs
    let (code, stdout, _stderr) = run_in(repaired.path(), &["--scope", "edit", "--emit", "json"]);
    assert_eq!(code, 0, "must pass");
    let report: Value = serde_json::from_str(&stdout).expect("stdout must be valid JSON");

    // Then: per_lane array length == 7
    let per_lane = report["per_lane"].as_array().expect("per_lane");
    assert_eq!(
        per_lane.len(),
        7,
        "per_lane must have 7 entries, got {}: {per_lane:?}",
        per_lane.len()
    );

    // Each outcome variant must be Clean or Skipped (no Failed or Findings).
    for (i, outcome) in per_lane.iter().enumerate() {
        let variant = external_tag(&outcome["outcome"]).unwrap_or("missing");
        assert!(
            variant == "Clean" || variant == "Skipped",
            "lane {i} variant must be Clean or Skipped, got: {variant}"
        );
    }
}

/// AC-7 (detail): per_lane contains all 7 Edit lane names.
#[test]
fn repaired_fixture_per_lane_contains_all_edit_lanes() {
    // Given: the repaired fixture workspace
    let repaired = fixtures::repaired_workspace();

    // When: titania-check runs
    let (code, stdout, _stderr) = run_in(repaired.path(), &["--scope", "edit", "--emit", "json"]);
    assert_eq!(code, 0, "must pass");
    let report: Value = serde_json::from_str(&stdout).expect("stdout must be valid JSON");

    // Then: all 7 Edit lane names present in per_lane
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

// ---------------------------------------------------------------------------
// E2E: Missing Cargo.toml — Reject Report
// ---------------------------------------------------------------------------

/// B11: An empty directory with no Cargo.toml produces exit code 1 reject report.
#[test]
fn missing_cargo_toml_produces_reject_report() {
    // Given: an empty temp directory (no Cargo.toml)
    let empty = tempfile::tempdir().expect("tempdir must be created");

    // When: titania-check runs against it
    let (code, stdout, stderr) = run_in(empty.path(), &["--scope", "edit", "--emit", "json"]);

    // Then: exit code 1 (reject), JSON report on stdout, no stderr
    assert_eq!(
        code, 1,
        "missing Cargo.toml in empty workspace must exit 1 (reject), got {}: stdout={stdout}, stderr={stderr}",
        code
    );
    assert!(stderr.is_empty(), "reject must not write stderr, got: {stderr}");
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("stdout must be JSON");
    assert_eq!(report["variant"], "Reject");
}

// ---------------------------------------------------------------------------
// Integration: RejectKind classification via JSON deserialization
// ---------------------------------------------------------------------------

/// B12: Mixed reject separates code findings from gate failures (no cross-contamination).
#[test]
fn mixed_report_separates_code_findings_from_gate_failures() {
    // Given: JSON for a Reject with 2 code findings + 1 gate failure
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
        "per_lane": []
    }"#;

    // When: deserialize and compute reject_kind
    let report: Result<titania_core::Report, _> = serde_json::from_str(json);

    // Then: reject_kind is Mixed
    let report = report.expect("must deserialize Reject from JSON");
    assert!(report.is_reject(), "must be Reject");
    let code_findings = report.code_findings().expect("Reject must have code_findings");
    let gate_failures = report.gate_failures().expect("Reject must have gate_failures");
    assert_eq!(code_findings.len(), 2, "must have 2 code findings");
    assert_eq!(gate_failures.len(), 1, "must have 1 gate failure");

    let kind = report.reject_kind().expect("reject must have reject_kind");
    assert_eq!(kind, titania_core::RejectKind::Mixed, "reject_kind must be Mixed");

    // Functional lanes only in code_findings
    let code_lanes: Vec<&str> = {
        let cf = report.code_findings().expect("Reject must have code_findings");
        cf.iter().map(|f| f.lane().name()).collect()
    };

    // Functional lanes: AstGrep, Clippy (not infrastructure-only lanes)
    for lane_name in &code_lanes {
        assert!(
            matches!(*lane_name, "AstGrep" | "Clippy"),
            "code finding lane must be functional, got: {lane_name}"
        );
    }
}

/// B13: GateOnly — zero code findings, one gate failure → RejectKind::GateOnly.
#[test]
fn report_reject_gate_only() {
    // Given: JSON for a Reject with 0 code findings + 1 gate failure
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
        "per_lane": []
    }"#;

    // When: deserialize and compute reject_kind
    let report: titania_core::Report =
        serde_json::from_str(json).expect("must deserialize Reject from JSON");

    // Then: reject_kind is GateOnly
    let kind = report.reject_kind().expect("reject must have reject_kind");
    assert_eq!(kind, titania_core::RejectKind::GateOnly, "reject_kind must be GateOnly");

    assert!(report.is_reject(), "must be Reject");
    let code_findings = report.code_findings().expect("Reject must have code_findings");
    let gate_failures = report.gate_failures().expect("Reject must have gate_failures");
    assert_eq!(code_findings.len(), 0, "code_findings must be empty");
    assert_eq!(gate_failures.len(), 1, "gate_failures must have 1 entry");
}

/// B14: Mixed — one code finding + one gate failure → RejectKind::Mixed.
#[test]
fn report_reject_mixed() {
    // Given: JSON for a Reject with 1 code finding + 1 gate failure
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
        "per_lane": []
    }"#;

    // When: deserialize and compute reject_kind
    let report: titania_core::Report =
        serde_json::from_str(json).expect("must deserialize Reject from JSON");

    // Then: reject_kind is Mixed
    let kind = report.reject_kind().expect("reject must have reject_kind");
    assert_eq!(kind, titania_core::RejectKind::Mixed, "reject_kind must be Mixed");

    assert!(report.is_reject(), "must be Reject");
    let code_findings = report.code_findings().expect("Reject must have code_findings");
    let gate_failures = report.gate_failures().expect("Reject must have gate_failures");
    assert_eq!(code_findings.len(), 1, "must have 1 code finding");
    assert_eq!(gate_failures.len(), 1, "must have 1 gate failure");
}

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

/// Module grouping fixture path helpers and workspace builders.
mod fixtures {
    use std::path::{Path, PathBuf};

    fn repo_root() -> PathBuf {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let _ = p.pop(); // crates
        let _ = p.pop(); // workspace root
        p
    }

    /// Path to the bad fixture workspace at repo root.
    pub(super) fn bad_path() -> PathBuf {
        let mut p = repo_root();
        p.push("fixtures");
        p.push("strict_ai_loop_unwrap");
        p.push("bad");
        p
    }

    /// Path to the repaired fixture workspace at repo root.
    pub(super) fn repaired_path() -> PathBuf {
        let mut p = repo_root();
        p.push("fixtures");
        p.push("strict_ai_loop_unwrap");
        p.push("repaired");
        p
    }

    /// Return a TempDir with a copy of the bad fixture (so the CLI sees the real files).
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

    /// Return a TempDir with a copy of the repaired fixture.
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

// ---------------------------------------------------------------------------
// Integration: Command::Check drives Moon (spec §12, §13)
// ---------------------------------------------------------------------------

/// `Command::Check` must invoke Moon before aggregating. Proven by pointing
/// `TITANIA_MOON_BIN` at a recording stub that writes a marker file with its
/// argv when invoked: the marker must exist and contain `moon run` plus the
/// scope's task list. This is the central fraud fix — `check` is no longer a
/// synonym for `aggregate`; it actually drives Moon.
#[test]
fn check_drives_moon_stub_and_aggregates_after() {
    let workspace = tempfile::tempdir().expect("tempdir must be created");

    // Write a recording stub that captures argv to a marker file.
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

    // Moon was invoked (marker exists).
    assert!(
        marker.exists(),
        "check must invoke moon (marker file should exist at {})",
        marker.display(),
    );

    // The captured argv must start with `run` and contain the edit-scope task
    // list from spec §13 (fmt, compile, clippy, ast-grep, dylint, panic-scan,
    // policy-scan).
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

    // After the (stubbed, exit-0) moon run, check must run the in-process
    // aggregate, producing a typed report on stdout. On an empty workspace
    // (no lane artifacts) the aggregate classifies this as Reject (exit 1).
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

/// Write a tiny POSIX shell stub that captures its argv to `marker` on
/// invocation, then exits 0. Used to prove the check→moon spawn path.
fn write_recording_stub(dir: &Path, marker: &Path) -> String {
    let stub_path = dir.join("moon-recording-stub.sh");
    // `"$@"` preserves argv quoting; `printf %s\\n "$@"` writes each arg on
    // its own line so the test can assert presence of individual task IDs.
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
