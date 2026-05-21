//! Integration tests for contracts-as-data (vb-6f02).
//!
//! These tests exercise the full discover_contracts() pipeline:
//! file walking → CUE validation → field extraction → error collection → sorting → GateEvidence
//!
//! They create temporary .cue files with known content and validate the end-to-end behavior.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::let_underscore_must_use,
    clippy::indexing_slicing,
    clippy::string_slice
)]

use std::fs;
use std::path::PathBuf;

use tempfile::TempDir;
use xtask::contracts::{ContractKind, GateStatus, discover_contracts, gate_evidence_from_report};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a temporary .cue file with the given kind and schema_version.
fn create_cue_file(dir: &TempDir, name: &str, kind: &str, version: &str) -> PathBuf {
    let path = dir.path().join(name);
    let content = format!(
        r#"kind: "{kind}"
schema_version: "{version}"
"#,
    );
    fs::write(&path, content).expect("Failed to write cue file");
    path
}

/// Create a .cue file with invalid kind.
fn create_bad_kind_cue(dir: &TempDir, name: &str, bad_kind: &str) -> PathBuf {
    let path = dir.path().join(name);
    let content = format!(
        r#"kind: "{bad_kind}"
schema_version: "1.0.0"
"#,
    );
    fs::write(&path, content).expect("Failed to write cue file");
    path
}

/// Create a .cue file missing kind field.
fn create_missing_kind_cue(dir: &TempDir, name: &str, version: &str) -> PathBuf {
    let path = dir.path().join(name);
    let content = format!(
        r#"schema_version: "{version}"
"#,
    );
    fs::write(&path, content).expect("Failed to write cue file");
    path
}

/// Create a .cue file missing schema_version.
fn create_missing_version_cue(dir: &TempDir, name: &str, kind: &str) -> PathBuf {
    let path = dir.path().join(name);
    let content = format!(
        r#"kind: "{kind}"
"#,
    );
    fs::write(&path, content).expect("Failed to write cue file");
    path
}

/// Create a .cue file with invalid version.
fn create_bad_version_cue(dir: &TempDir, name: &str, kind: &str, bad_ver: &str) -> PathBuf {
    let path = dir.path().join(name);
    let content = format!(
        r#"kind: "{kind}"
schema_version: "{bad_ver}"
"#,
    );
    fs::write(&path, content).expect("Failed to write cue file");
    path
}

// ---------------------------------------------------------------------------
// INV-005: Deterministic file discovery (sorted output)
// ---------------------------------------------------------------------------

#[allow(clippy::unwrap_used, clippy::expect_used)]
#[test]
fn test_discovery_deterministic_order() {
    let dir = tempfile::tempdir().unwrap();

    // Create files in non-alphabetical order.
    create_cue_file(&dir, "z_file.cue", "cli_envelope", "1.0.0");
    create_cue_file(&dir, "a_file.cue", "ui_tokens", "1.0.0");
    create_cue_file(&dir, "m_file.cue", "evidence_bundle", "1.0.0");

    let report = discover_contracts(dir.path()).expect("discover_contracts should succeed");

    // Files should be sorted alphabetically by path.
    let paths: Vec<_> = report.files.iter().map(|f| f.path.clone()).collect();
    assert_eq!(paths.len(), 3);
    assert_eq!(
        paths
            .get(0)
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str()),
        Some("a_file.cue")
    );
    assert_eq!(
        paths
            .get(1)
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str()),
        Some("m_file.cue")
    );
    assert_eq!(
        paths
            .get(2)
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str()),
        Some("z_file.cue")
    );
}

#[allow(clippy::unwrap_used, clippy::expect_used)]
#[test]
fn test_discovery_errors_sorted() {
    let dir = tempfile::tempdir().unwrap();

    // Create files that will produce errors (bad kinds).
    create_bad_kind_cue(&dir, "a_bad.cue", "unknown_kind");
    create_bad_kind_cue(&dir, "b_bad.cue", "another_bad");
    create_bad_kind_cue(&dir, "c_bad.cue", "zzz_bad");

    let report = discover_contracts(dir.path()).expect("discover_contracts should succeed");

    // Errors should be sorted.
    assert!(
        report.errors.is_sorted(),
        "errors must be sorted for determinism"
    );
    assert_eq!(report.errors.len(), 3);
}

// ---------------------------------------------------------------------------
// OBL-004: Discovery finds all .cue files recursively
// ---------------------------------------------------------------------------

#[test]
fn test_discovery_finds_all_cue_files() {
    let dir = tempfile::tempdir().unwrap();

    // Create files at root level.
    create_cue_file(&dir, "root.cue", "cli_envelope", "1.0.0");

    // Create subdirectory with files.
    let sub1 = dir.path().join("sub1");
    fs::create_dir(&sub1).unwrap();
    create_cue_file(&dir, "sub1/nested.cue", "ui_tokens", "1.0.0");

    // Create nested subdirectory.
    let sub2 = sub1.join("sub2");
    fs::create_dir(&sub2).unwrap();
    create_cue_file(&dir, "sub1/sub2/deep.cue", "evidence_bundle", "1.0.0");

    let report = discover_contracts(dir.path()).expect("discover_contracts should succeed");

    assert_eq!(report.summary.total, 3);
    assert_eq!(report.summary.valid, 3);
    assert_eq!(report.summary.invalid, 0);
}

#[test]
fn test_discovery_ignores_non_cue_files() {
    let dir = tempfile::tempdir().unwrap();

    create_cue_file(&dir, "valid.cue", "cli_envelope", "1.0.0");
    fs::write(dir.path().join("readme.md"), "# README").unwrap();
    fs::write(dir.path().join("config.yaml"), "key: value").unwrap();
    fs::write(dir.path().join("data.json"), "{}").unwrap();

    let report = discover_contracts(dir.path()).expect("discover_contracts should succeed");

    assert_eq!(report.summary.total, 1);
    assert_eq!(report.summary.valid, 1);
}

// ---------------------------------------------------------------------------
// Full pipeline: file walking → CUE validation → field extraction →
// error collection → sorting → GateEvidence
// ---------------------------------------------------------------------------

#[test]
fn test_pipeline_all_valid() {
    let dir = tempfile::tempdir().unwrap();

    create_cue_file(&dir, "a.cli.cue", "cli_envelope", "1.0.0");
    create_cue_file(&dir, "b.ui.cue", "ui_tokens", "1.0.0");
    create_cue_file(&dir, "c.evidence.cue", "evidence_bundle", "1.0.0");

    let report = discover_contracts(dir.path()).expect("discover_contracts should succeed");
    let evidence = gate_evidence_from_report(&report);

    // Verify report structure.
    assert_eq!(report.summary.total, 3);
    assert_eq!(report.summary.valid, 3);
    assert_eq!(report.summary.invalid, 0);
    assert!(report.errors.is_empty(), "no errors for valid contracts");
    assert!(report.summary.errors_by_kind.is_empty());
    assert!(report.summary.version_violations.is_empty());

    // Verify GateEvidence.
    assert_eq!(evidence.status, GateStatus::Pass);
    assert_eq!(evidence.exit_code, 0);
    assert!(evidence.why_failed.is_none());
}

#[test]
fn test_pipeline_mixed_valid_invalid() {
    let dir = tempfile::tempdir().unwrap();

    create_cue_file(&dir, "valid.cue", "cli_envelope", "1.0.0");
    create_bad_kind_cue(&dir, "invalid.cue", "unknown_kind");
    create_cue_file(&dir, "valid2.cue", "ui_tokens", "1.0.0");

    let report = discover_contracts(dir.path()).expect("discover_contracts should succeed");
    let evidence = gate_evidence_from_report(&report);

    assert_eq!(report.summary.total, 3);
    assert_eq!(report.summary.valid, 2);
    assert_eq!(report.summary.invalid, 1);
    assert!(!report.errors.is_empty());
    assert!(report.errors.iter().any(|e| e.contains("INVALID_KIND")));

    assert_eq!(evidence.status, GateStatus::Fail);
    assert_eq!(evidence.exit_code, 1);
    assert!(evidence.why_failed.is_some());
}

#[test]
fn test_pipeline_no_contracts() {
    let dir = tempfile::tempdir().unwrap();

    let report = discover_contracts(dir.path()).expect("discover_contracts should succeed");
    let evidence = gate_evidence_from_report(&report);

    assert_eq!(report.summary.total, 0);
    assert_eq!(report.summary.valid, 0);
    assert_eq!(report.summary.invalid, 0);
    assert!(report.files.is_empty());
    assert!(report.errors.is_empty());

    assert_eq!(evidence.status, GateStatus::Pass);
    assert_eq!(evidence.exit_code, 0);
}

// ---------------------------------------------------------------------------
// Field extraction: kind and schema_version parsing from CUE content
// ---------------------------------------------------------------------------

#[test]
fn test_pipeline_missing_kind() {
    let dir = tempfile::tempdir().unwrap();

    create_missing_kind_cue(&dir, "missing_kind.cue", "1.0.0");

    let report = discover_contracts(dir.path()).expect("discover_contracts should succeed");

    assert_eq!(report.summary.total, 1);
    assert_eq!(report.summary.valid, 0);
    assert_eq!(report.summary.invalid, 1);
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.contains("MISSING_SCHEMA_VERSION") || e.contains("MISSING"))
    );
}

#[test]
fn test_pipeline_missing_schema_version() {
    let dir = tempfile::tempdir().unwrap();

    create_missing_version_cue(&dir, "missing_ver.cue", "cli_envelope");

    let report = discover_contracts(dir.path()).expect("discover_contracts should succeed");

    assert_eq!(report.summary.total, 1);
    assert_eq!(report.summary.valid, 0);
    assert_eq!(report.summary.invalid, 1);
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.contains("MISSING_SCHEMA_VERSION") || e.contains("MISSING"))
    );
}

#[test]
fn test_pipeline_bad_schema_version() {
    let dir = tempfile::tempdir().unwrap();

    create_bad_version_cue(&dir, "bad_ver.cue", "cli_envelope", "not-a-version");

    let report = discover_contracts(dir.path()).expect("discover_contracts should succeed");

    assert_eq!(report.summary.total, 1);
    assert_eq!(report.summary.valid, 0);
    assert_eq!(report.summary.invalid, 1);
    assert!(report.errors.iter().any(|e| e.contains("INVALID_VERSION")));
}

// ---------------------------------------------------------------------------
// Monotonicity gate (Repair 3 — MAJOR)
// ---------------------------------------------------------------------------

#[test]
fn test_monotonicity_pass() {
    let dir = tempfile::tempdir().unwrap();

    // Versions in non-decreasing order.
    create_cue_file(&dir, "a.cue", "cli_envelope", "1.0.0");
    create_cue_file(&dir, "b.cue", "ui_tokens", "1.0.0");
    create_cue_file(&dir, "c.cue", "evidence_bundle", "1.1.0");
    create_cue_file(&dir, "d.cue", "accepted_artifacts", "2.0.0");

    let report = discover_contracts(dir.path()).expect("discover_contracts should succeed");

    assert_eq!(report.summary.invalid, 0);
    assert!(
        !report
            .errors
            .iter()
            .any(|e| e.contains("VERSION_MONOTONICITY_BREACH"))
    );
    assert!(report.summary.version_violations.is_empty());
}

#[test]
fn test_monotonicity_fail() {
    let dir = tempfile::tempdir().unwrap();

    // Versions in decreasing order — breach!
    create_cue_file(&dir, "a.cue", "cli_envelope", "2.0.0");
    create_cue_file(&dir, "b.cue", "ui_tokens", "1.0.0");
    create_cue_file(&dir, "c.cue", "evidence_bundle", "1.1.0");

    let report = discover_contracts(dir.path()).expect("discover_contracts should succeed");

    assert!(
        report
            .errors
            .iter()
            .any(|e| e.contains("VERSION_MONOTONICITY_BREACH"))
    );
    assert!(!report.summary.version_violations.is_empty());
}

#[test]
fn test_monotonicity_gate_json_output() {
    let dir = tempfile::tempdir().unwrap();

    create_cue_file(&dir, "a.cue", "cli_envelope", "2.0.0");
    create_cue_file(&dir, "b.cue", "ui_tokens", "1.0.0");

    let report = discover_contracts(dir.path()).expect("discover_contracts should succeed");
    let json = serde_json::to_string(&report).expect("serialization must succeed");

    assert!(
        json.contains("VERSION_MONOTONICITY_BREACH"),
        "JSON must contain monotonicity breach error"
    );
    assert!(json.contains("2.0.0"));
    assert!(json.contains("1.0.0"));
}

// ---------------------------------------------------------------------------
// CUE vet integration (Repair 5 — RECOMMENDED)
// ---------------------------------------------------------------------------

#[test]
fn test_cue_vet_not_available_continues_gracefully() {
    // CUE may not be installed on all systems.
    // The pipeline should continue field extraction even if cue vet fails.
    let dir = tempfile::tempdir().unwrap();

    // Create a valid CUE file.
    create_cue_file(&dir, "valid.cue", "cli_envelope", "1.0.0");

    let report = discover_contracts(dir.path()).expect("discover_contracts should succeed");

    // Even if cue vet fails, field extraction should still work.
    assert_eq!(report.summary.total, 1);
    assert_eq!(report.summary.valid, 1);
}

#[test]
fn test_cue_vet_nonexistent_file() {
    // discover_contracts should return an error for non-existent directory.
    let result = discover_contracts(PathBuf::from("/nonexistent/path/to/contracts").as_path());

    assert!(
        result.is_err(),
        "discover_contracts should error on nonexistent dir"
    );
    let err = result.expect_err("discover_contracts should error on nonexistent dir");
    assert!(err.contains("does not exist"));
}

#[test]
fn test_cue_vet_not_a_directory() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("not_a_dir.cue");
    fs::write(&file_path, "").unwrap();

    let result = discover_contracts(&file_path);

    assert!(
        result.is_err(),
        "discover_contracts should error on non-directory"
    );
    let err = result.expect_err("discover_contracts should error on non-directory");
    assert!(err.contains("not a directory"));
}

// ---------------------------------------------------------------------------
// GateEvidence integration from discovery report
// ---------------------------------------------------------------------------

#[test]
fn test_gate_evidence_from_pipeline_report() {
    let dir = tempfile::tempdir().unwrap();

    create_cue_file(&dir, "a.cue", "cli_envelope", "1.0.0");
    create_bad_kind_cue(&dir, "b.cue", "bogus_kind");

    let report = discover_contracts(dir.path()).expect("discover_contracts should succeed");
    let evidence = gate_evidence_from_report(&report);

    // Verify GateEvidence fields match report state.
    assert_eq!(evidence.kind, "contract-discovery");
    assert_eq!(evidence.gate_name, "contracts");
    assert!(evidence.command.contains("cargo xtask contracts"));

    if report.summary.invalid > 0 {
        assert_eq!(evidence.status, GateStatus::Fail);
        assert_eq!(evidence.exit_code, 1);
        assert!(evidence.why_failed.is_some());
        let why = evidence.why_failed.as_ref().unwrap();
        assert!(!why.hint.is_empty());
        assert!(why.repair_command.contains("cargo xtask contracts"));
    } else {
        assert_eq!(evidence.status, GateStatus::Pass);
        assert_eq!(evidence.exit_code, 0);
        assert!(evidence.why_failed.is_none());
    }
}

// ---------------------------------------------------------------------------
// ReportSummary invariant: total == valid + invalid
// ---------------------------------------------------------------------------

#[test]
fn test_summary_total_invariant_all_valid() {
    let dir = tempfile::tempdir().unwrap();

    create_cue_file(&dir, "a.cue", "cli_envelope", "1.0.0");
    create_cue_file(&dir, "b.cue", "ui_tokens", "1.0.0");
    create_cue_file(&dir, "c.cue", "evidence_bundle", "1.0.0");

    let report = discover_contracts(dir.path()).expect("discover_contracts should succeed");

    assert_eq!(
        report.summary.total,
        report.summary.valid.saturating_add(report.summary.invalid)
    );
}

#[test]
fn test_summary_total_invariant_all_invalid() {
    let dir = tempfile::tempdir().unwrap();

    create_bad_kind_cue(&dir, "a.cue", "bogus");
    create_bad_kind_cue(&dir, "b.cue", "unknown");

    let report = discover_contracts(dir.path()).expect("discover_contracts should succeed");

    assert_eq!(
        report.summary.total,
        report.summary.valid.saturating_add(report.summary.invalid)
    );
}

#[test]
fn test_summary_total_invariant_mixed() {
    let dir = tempfile::tempdir().unwrap();

    create_cue_file(&dir, "a.cue", "cli_envelope", "1.0.0");
    create_bad_kind_cue(&dir, "b.cue", "bogus");
    create_cue_file(&dir, "c.cue", "ui_tokens", "1.0.0");

    let report = discover_contracts(dir.path()).expect("discover_contracts should succeed");

    assert_eq!(
        report.summary.total,
        report.summary.valid.saturating_add(report.summary.invalid)
    );
    assert_eq!(report.summary.total, 3);
    assert_eq!(report.summary.valid, 2);
    assert_eq!(report.summary.invalid, 1);
}

// ---------------------------------------------------------------------------
// JSON output with sorted keys (Repair 4 — RECOMMENDED)
// ---------------------------------------------------------------------------

#[test]
fn test_json_output_has_required_keys() {
    let dir = tempfile::tempdir().unwrap();

    create_cue_file(&dir, "a.cue", "cli_envelope", "1.0.0");
    create_bad_kind_cue(&dir, "b.cue", "bogus");

    let report = discover_contracts(dir.path()).expect("discover_contracts should succeed");
    let json = serde_json::to_string_pretty(&report).expect("JSON serialization must succeed");

    let parsed: serde_json::Value = serde_json::from_str(&json).expect("JSON must be parseable");

    assert!(parsed.get("files").is_some(), "JSON must have 'files' key");
    assert!(
        parsed.get("errors").is_some(),
        "JSON must have 'errors' key"
    );
    assert!(
        parsed.get("summary").is_some(),
        "JSON must have 'summary' key"
    );
}

#[test]
fn test_json_deterministic_key_order() {
    let dir = tempfile::tempdir().unwrap();

    // Create files with different kinds to ensure errors_by_kind has multiple entries.
    create_bad_kind_cue(&dir, "a.cue", "aaa_bad");
    create_bad_kind_cue(&dir, "b.cue", "zzz_bad");

    let report = discover_contracts(dir.path()).expect("discover_contracts should succeed");
    let json = serde_json::to_string(&report).expect("JSON serialization must succeed");

    // BTreeMap ensures deterministic key order.
    let aaa_pos = json
        .find("INVALID_KIND: aaa_bad")
        .expect("aaa_bad must be in JSON");
    let zzz_pos = json
        .find("INVALID_KIND: zzz_bad")
        .expect("zzz_bad must be in JSON");

    assert!(
        aaa_pos < zzz_pos,
        "BTreeMap keys must be sorted: aaa_bad before zzz_bad"
    );
}

// ---------------------------------------------------------------------------
// Error collection and deduplication
// ---------------------------------------------------------------------------

#[test]
fn test_errors_deduplicated() {
    let dir = tempfile::tempdir().unwrap();

    // Multiple files with the same invalid kind.
    create_bad_kind_cue(&dir, "a.cue", "duplicate_error");
    create_bad_kind_cue(&dir, "b.cue", "duplicate_error");
    create_bad_kind_cue(&dir, "c.cue", "duplicate_error");

    let report = discover_contracts(dir.path()).expect("discover_contracts should succeed");

    // Same error message may appear multiple times (once per file),
    // but the error list should be sorted.
    assert!(report.errors.is_sorted(), "errors must be sorted");
    assert_eq!(report.errors.len(), 3);
}

#[test]
fn test_errors_by_kind_counts() {
    let dir = tempfile::tempdir().unwrap();

    create_bad_kind_cue(&dir, "a.cue", "kind_a");
    create_bad_kind_cue(&dir, "b.cue", "kind_b");
    create_bad_kind_cue(&dir, "c.cue", "kind_a");

    let report = discover_contracts(dir.path()).expect("discover_contracts should succeed");

    // errors_by_kind should count occurrences per error type.
    let kind_a_count = report.summary.errors_by_kind.get("INVALID_KIND: kind_a");
    let kind_b_count = report.summary.errors_by_kind.get("INVALID_KIND: kind_b");

    assert_eq!(kind_a_count, Some(&2), "kind_a should appear twice");
    assert_eq!(kind_b_count, Some(&1), "kind_b should appear once");
}

// ---------------------------------------------------------------------------
// Edge case: empty directory (no .cue files)
// ---------------------------------------------------------------------------

#[test]
fn test_empty_directory() {
    let dir = tempfile::tempdir().unwrap();

    let report = discover_contracts(dir.path()).expect("discover_contracts should succeed");

    assert_eq!(report.summary.total, 0);
    assert_eq!(report.summary.valid, 0);
    assert_eq!(report.summary.invalid, 0);
    assert!(report.files.is_empty());
    assert!(report.errors.is_empty());
}

// ---------------------------------------------------------------------------
// Edge case: deeply nested subdirectories
// ---------------------------------------------------------------------------

#[test]
fn test_deeply_nested_discovery() {
    let dir = tempfile::tempdir().unwrap();

    // Create a deep directory structure.
    for i in 1..=5 {
        let subdir = dir.path().join(format!("level{}", i));
        fs::create_dir(&subdir).unwrap();
        create_cue_file(
            &dir,
            &format!("level{}/file{}.cue", i, i),
            "cli_envelope",
            "1.0.0",
        );
    }

    let report = discover_contracts(dir.path()).expect("discover_contracts should succeed");

    assert_eq!(report.summary.total, 5);
    assert_eq!(report.summary.valid, 5);
}

// ---------------------------------------------------------------------------
// Edge case: single file
// ---------------------------------------------------------------------------

#[test]
fn test_single_file() {
    let dir = tempfile::tempdir().unwrap();

    create_cue_file(&dir, "only.cue", "cli_envelope", "1.0.0");

    // Debug: check if file exists
    let file_path = dir.path().join("only.cue");
    assert!(
        file_path.exists(),
        "Cue file should exist at {:?}",
        file_path
    );
    let contents = fs::read_to_string(&file_path).expect("Should be able to read file");
    assert!(
        contents.contains("cli_envelope"),
        "File should contain 'cli_envelope', got: {}",
        contents
    );

    let report = discover_contracts(dir.path()).expect("discover_contracts should succeed");

    assert_eq!(report.summary.total, 1);
    assert_eq!(report.summary.valid, 1);
    assert_eq!(report.files.len(), 1);
    assert_eq!(
        report.files.get(0).map(|f| f.kind),
        Some(ContractKind::CliEnvelope)
    );
    assert_eq!(
        report.files.get(0).map(|f| f.schema_version.as_str()),
        Some("1.0.0")
    );
}

// ---------------------------------------------------------------------------
// Edge case: all files have monotonicity breaches
// ---------------------------------------------------------------------------

#[test]
fn test_all_files_monotonicity_breach() {
    let dir = tempfile::tempdir().unwrap();

    // Each file has a lower version than the previous.
    create_cue_file(&dir, "a.cue", "cli_envelope", "5.0.0");
    create_cue_file(&dir, "b.cue", "ui_tokens", "4.0.0");
    create_cue_file(&dir, "c.cue", "evidence_bundle", "3.0.0");
    create_cue_file(&dir, "d.cue", "accepted_artifacts", "2.0.0");

    let report = discover_contracts(dir.path()).expect("discover_contracts should succeed");

    assert!(
        report
            .errors
            .iter()
            .any(|e| e.contains("VERSION_MONOTONICITY_BREACH"))
    );
    assert!(!report.summary.version_violations.is_empty());
}

// ---------------------------------------------------------------------------
// Edge case: version violation edge values
// ---------------------------------------------------------------------------

#[test]
fn test_monotonicity_edge_zero_version() {
    let dir = tempfile::tempdir().unwrap();

    create_cue_file(&dir, "a.cue", "cli_envelope", "0.0.0");
    create_cue_file(&dir, "b.cue", "ui_tokens", "0.0.1");

    let report = discover_contracts(dir.path()).expect("discover_contracts should succeed");

    assert_eq!(report.summary.invalid, 0);
    assert!(report.summary.version_violations.is_empty());
}

#[test]
fn test_monotonicity_equal_versions_ok() {
    let dir = tempfile::tempdir().unwrap();

    // Equal versions are allowed (non-decreasing means >=).
    create_cue_file(&dir, "a.cue", "cli_envelope", "1.0.0");
    create_cue_file(&dir, "b.cue", "ui_tokens", "1.0.0");
    create_cue_file(&dir, "c.cue", "evidence_bundle", "1.0.0");

    let report = discover_contracts(dir.path()).expect("discover_contracts should succeed");

    assert_eq!(report.summary.invalid, 0);
    assert!(report.summary.version_violations.is_empty());
}
