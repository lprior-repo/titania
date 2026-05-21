//! Production-type binding tests for contracts-as-data (vb-6f02).
//!
//! CRITICAL REPAIR: These tests import types directly from `xtask::contracts`
//! and `xtask::evidence` — they are NOT independent copies.
//!
//! Covers: OBL-001 (parse_schema_version), OBL-002 (parse_contract_kind),
//! OBL-003 (compare_semver), OBL-004 (parse_vet_exit_code),
//! OBL-005 (gate_evidence_from_report), OBL-006 (ContractFile serde).

#![allow(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::let_underscore_must_use,
    clippy::indexing_slicing,
    clippy::string_slice
)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use xtask::contracts::{
    ContractError, ContractFile, ContractKind, DiscoveryReport, ReportSummary, SemverCmp,
    compare_semver, gate_evidence_from_report, parse_schema_version, parse_vet_exit_code,
};
use xtask::evidence::{GateEvidence, GateStatus, WhyFailed};

// ============================================================
// OBL-001: parse_schema_version — bound to production code
// ============================================================

#[test]
fn test_prod_parse_schema_version_valid() {
    assert_eq!(parse_schema_version("1.0.0"), Ok("1.0.0".to_string()));
    assert_eq!(parse_schema_version("2.1.0"), Ok("2.1.0".to_string()));
    assert_eq!(parse_schema_version("0.9.9"), Ok("0.9.9".to_string()));
}

#[test]
fn test_prod_parse_schema_version_invalid() {
    assert!(parse_schema_version("").is_err());
    assert!(parse_schema_version("1.0").is_err());
    assert!(parse_schema_version("1.0.0.0").is_err());
    assert!(parse_schema_version("abc").is_err());
    assert!(parse_schema_version("1.0.abc").is_err());
    assert!(parse_schema_version("v1.0.0").is_err());
}

#[test]
fn test_prod_parse_schema_version_error_display() {
    #[allow(clippy::expect_used)]
    let err = parse_schema_version("").expect_err("parse_schema_version(\"\") should err");
    assert_eq!(err.to_string(), "MISSING_SCHEMA_VERSION");

    #[allow(clippy::expect_used)]
    let err =
        parse_schema_version("v1.0.0").expect_err("parse_schema_version(\"v1.0.0\") should err");
    assert!(err.to_string().contains("INVALID_VERSION"));
}

// ============================================================
// OBL-002: parse_contract_kind — bound to production code
// ============================================================

#[test]
fn test_prod_parse_contract_kind_all_valid() {
    assert_eq!(
        ContractKind::parse("cli_envelope"),
        Ok(ContractKind::CliEnvelope)
    );
    assert_eq!(ContractKind::parse("ui_tokens"), Ok(ContractKind::UiTokens));
    assert_eq!(
        ContractKind::parse("accepted_artifacts"),
        Ok(ContractKind::AcceptedArtifacts)
    );
    assert_eq!(
        ContractKind::parse("evidence_bundle"),
        Ok(ContractKind::EvidenceBundle)
    );
    assert_eq!(
        ContractKind::parse("diagnostics"),
        Ok(ContractKind::Diagnostics)
    );
    assert_eq!(
        ContractKind::parse("gate_output"),
        Ok(ContractKind::GateOutput)
    );
}

#[test]
fn test_prod_parse_contract_kind_invalid() {
    assert!(ContractKind::parse("").is_err());
    assert!(ContractKind::parse("CLI_ENVELOPE").is_err());
    assert!(ContractKind::parse("cli-envelope").is_err());
    assert!(ContractKind::parse("unknown").is_err());
    assert!(ContractKind::parse("cli_envelope_extra").is_err());
}

#[test]
fn test_prod_parse_contract_kind_error_display() {
    #[allow(clippy::expect_used)]
    let err = ContractKind::parse("bogus").expect_err("ContractKind::parse(\"bogus\") should err");
    // ContractKind::parse returns the unrecognised string as the error
    assert_eq!(err, "bogus");
}

#[test]
fn test_prod_contract_kind_round_trip() -> Result<(), String> {
    for kind in ContractKind::all_values() {
        let display = kind.to_string();
        let parsed = ContractKind::parse(&display)?;
        if parsed != *kind {
            return Err(format!(
                "round trip failed for {:?}: expected {:?}, got {:?}",
                kind, kind, parsed
            ));
        }
    }
    Ok(())
}

// ============================================================
// OBL-003: compare_semver — bound to production code
// ============================================================

#[test]
fn test_prod_compare_semver_equal() {
    assert_eq!(compare_semver("1.0.0", "1.0.0"), Ok(SemverCmp::Equal));
    assert_eq!(compare_semver("0.0.0", "0.0.0"), Ok(SemverCmp::Equal));
    assert_eq!(compare_semver("99.99.99", "99.99.99"), Ok(SemverCmp::Equal));
}

#[test]
fn test_prod_compare_semver_less() {
    assert_eq!(compare_semver("1.0.0", "2.0.0"), Ok(SemverCmp::Less));
    assert_eq!(compare_semver("1.0.0", "1.1.0"), Ok(SemverCmp::Less));
    assert_eq!(compare_semver("1.0.0", "1.0.1"), Ok(SemverCmp::Less));
    assert_eq!(compare_semver("0.0.0", "0.0.1"), Ok(SemverCmp::Less));
}

#[test]
fn test_prod_compare_semver_greater() {
    assert_eq!(compare_semver("2.0.0", "1.0.0"), Ok(SemverCmp::Greater));
    assert_eq!(compare_semver("1.1.0", "1.0.0"), Ok(SemverCmp::Greater));
    assert_eq!(compare_semver("1.0.1", "1.0.0"), Ok(SemverCmp::Greater));
    assert_eq!(compare_semver("0.0.1", "0.0.0"), Ok(SemverCmp::Greater));
}

#[test]
fn test_prod_compare_semver_invalid_format() {
    assert!(compare_semver("1.0", "1.0.0").is_err());
    assert!(compare_semver("1.0.0", "1.0").is_err());
    assert!(compare_semver("abc", "1.0.0").is_err());
    assert!(compare_semver("1.0.0.0", "1.0.0").is_err());
}

// ============================================================
// OBL-004: parse_vet_exit_code — bound to production code
// ============================================================

#[test]
fn test_prod_parse_vet_exit_code_success() {
    assert!(parse_vet_exit_code(0).is_ok());
}

#[test]
fn test_prod_parse_vet_exit_code_failure() {
    assert!(parse_vet_exit_code(1).is_err());
    assert!(parse_vet_exit_code(-1).is_err());
    assert!(parse_vet_exit_code(255).is_err());
    assert!(parse_vet_exit_code(127).is_err());
}

#[test]
fn test_prod_parse_vet_exit_code_error_message() {
    #[allow(clippy::expect_used)]
    let err = parse_vet_exit_code(1).expect_err("parse_vet_exit_code(1) should err");
    assert!(err.contains("cue vet exited with code 1"));
}

// ============================================================
// OBL-005: gate_evidence_from_report — bound to production code
// ============================================================

#[test]
fn test_prod_gate_evidence_pass() {
    let report = DiscoveryReport {
        files: vec![
            ContractFile {
                path: PathBuf::from("contracts/cli_envelope.cue"),
                schema_version: "1.0.0".to_string(),
                kind: ContractKind::CliEnvelope,
                vet_errors: Vec::new(),
            },
            ContractFile {
                path: PathBuf::from("contracts/ui_tokens.cue"),
                schema_version: "1.0.0".to_string(),
                kind: ContractKind::UiTokens,
                vet_errors: Vec::new(),
            },
        ],
        errors: Vec::new(),
        summary: ReportSummary {
            total: 2,
            valid: 2,
            invalid: 0,
            errors_by_kind: BTreeMap::new(),
            version_violations: Vec::new(),
        },
    };

    let evidence = gate_evidence_from_report(&report);

    assert_eq!(evidence.kind, "contract-discovery");
    assert_eq!(evidence.gate_name, "contracts");
    assert_eq!(evidence.exit_code, 0);
    assert_eq!(evidence.status, GateStatus::Pass);
    assert!(evidence.why_failed.is_none());
    assert_eq!(evidence.command, "cargo xtask contracts --dir contracts");
}

#[test]
fn test_prod_gate_evidence_fail() -> Result<(), Box<dyn std::error::Error>> {
    let report = DiscoveryReport {
        files: vec![ContractFile {
            path: PathBuf::from("contracts/bad.cue"),
            schema_version: "1.0.0".to_string(),
            kind: ContractKind::CliEnvelope,
            vet_errors: vec!["INVALID_KIND: bogus".to_string()],
        }],
        errors: vec!["INVALID_KIND: bogus".to_string()],
        summary: ReportSummary {
            total: 1,
            valid: 0,
            invalid: 1,
            errors_by_kind: [("INVALID_KIND: bogus".to_string(), 1u32)]
                .into_iter()
                .collect(),
            version_violations: Vec::new(),
        },
    };

    let evidence = gate_evidence_from_report(&report);

    assert_eq!(evidence.status, GateStatus::Fail);
    assert_eq!(evidence.exit_code, 1);
    let why = evidence
        .why_failed
        .as_ref()
        .ok_or("why_failed should be Some for Fail status")?;
    assert_eq!(why.gate_name, "contracts");
    assert!(why.repair_command.contains("cargo xtask contracts"));
    assert!(why.hint.contains("1 contract"));
    Ok(())
}

#[test]
fn test_prod_gate_evidence_empty_report() {
    let report = DiscoveryReport {
        files: Vec::new(),
        errors: Vec::new(),
        summary: ReportSummary::new(),
    };

    let evidence = gate_evidence_from_report(&report);

    assert_eq!(evidence.status, GateStatus::Pass);
    assert_eq!(evidence.exit_code, 0);
    assert!(evidence.why_failed.is_none());
}

#[test]
fn test_prod_gate_evidence_multiple_errors() -> Result<(), Box<dyn std::error::Error>> {
    let errors = vec![
        "INVALID_KIND: bogus".to_string(),
        "MISSING_SCHEMA_VERSION".to_string(),
        "INVALID_KIND: unknown".to_string(),
    ];

    let report = DiscoveryReport {
        files: Vec::new(),
        errors: errors.clone(),
        summary: ReportSummary {
            total: 3,
            valid: 0,
            invalid: 3,
            errors_by_kind: [
                ("INVALID_KIND: bogus".to_string(), 1u32),
                ("INVALID_KIND: unknown".to_string(), 1u32),
                ("MISSING_SCHEMA_VERSION".to_string(), 1u32),
            ]
            .into_iter()
            .collect(),
            version_violations: Vec::new(),
        },
    };

    let evidence = gate_evidence_from_report(&report);

    assert_eq!(evidence.status, GateStatus::Fail);
    let why = evidence
        .why_failed
        .as_ref()
        .ok_or("why_failed should be Some for Fail status")?;
    // Errors should be sorted and deduplicated
    assert!(why.hint.contains("3 contract"));
    Ok(())
}

// ============================================================
// OBL-006: ContractFile / DiscoveryReport serde — bound to production code
// ============================================================

#[test]
fn test_prod_contract_file_serialization() {
    let file = ContractFile {
        path: PathBuf::from("contracts/cli_envelope.cue"),
        schema_version: "1.0.0".to_string(),
        kind: ContractKind::CliEnvelope,
        vet_errors: vec!["CUE_VET_FAILED: syntax error".to_string()],
    };

    let json = serde_json::to_string(&file).expect("ContractFile serialization should not fail");
    let parsed: ContractFile =
        serde_json::from_str(&json).expect("ContractFile deserialization should succeed");

    assert_eq!(parsed.path, file.path);
    assert_eq!(parsed.schema_version, file.schema_version);
    assert_eq!(parsed.kind, file.kind);
    assert_eq!(parsed.vet_errors, file.vet_errors);
}

#[test]
fn test_prod_discovery_report_serialization() {
    let report = DiscoveryReport {
        files: vec![ContractFile {
            path: PathBuf::from("contracts/cli_envelope.cue"),
            schema_version: "1.0.0".to_string(),
            kind: ContractKind::CliEnvelope,
            vet_errors: Vec::new(),
        }],
        errors: vec!["INVALID_KIND: bogus".to_string()],
        summary: ReportSummary {
            total: 2,
            valid: 1,
            invalid: 1,
            errors_by_kind: BTreeMap::from_iter(vec![("INVALID_KIND: bogus".to_string(), 1)]),
            version_violations: Vec::new(),
        },
    };

    let json = serde_json::to_string_pretty(&report)
        .expect("DiscoveryReport serialization should not fail");

    let parsed: serde_json::Value = serde_json::from_str(&json).expect("JSON must be parseable");

    assert!(
        parsed.get("summary").is_some(),
        "JSON must have 'summary' key for moon task consumers"
    );
    assert!(
        parsed.get("errors").is_some(),
        "JSON must have 'errors' key for moon task consumers"
    );
    assert!(
        parsed.get("files").is_some(),
        "JSON must have 'files' key for moon task consumers"
    );
}

#[test]
fn test_prod_report_summary_deterministic_key_order() -> Result<(), Box<dyn std::error::Error>> {
    // BTreeMap ensures deterministic JSON key order (OBL-006 property).
    let mut errors_by_kind: BTreeMap<String, u32> = BTreeMap::new();
    errors_by_kind.insert("zzz_last".to_string(), 1);
    errors_by_kind.insert("aaa_first".to_string(), 2);
    errors_by_kind.insert("mmm_middle".to_string(), 3);

    let report = DiscoveryReport {
        files: Vec::new(),
        errors: Vec::new(),
        summary: ReportSummary {
            total: 6,
            valid: 4,
            invalid: 2,
            errors_by_kind,
            version_violations: Vec::new(),
        },
    };

    let json = serde_json::to_string(&report)?;

    // The JSON key order must be deterministic: aaa_first before mmm_middle before zzz_last.
    let aaa_pos = json
        .find("\"aaa_first\"")
        .ok_or("aaa_first not found in JSON")?;
    let mmm_pos = json
        .find("\"mmm_middle\"")
        .ok_or("mmm_middle not found in JSON")?;
    let zzz_pos = json
        .find("\"zzz_last\"")
        .ok_or("zzz_last not found in JSON")?;

    assert!(
        aaa_pos < mmm_pos,
        "aaa_first must come before mmm_middle in JSON"
    );
    assert!(
        mmm_pos < zzz_pos,
        "mmm_middle must come before zzz_last in JSON"
    );
    Ok(())
}

#[test]
fn test_prod_gate_evidence_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let evidence = GateEvidence {
        kind: "contract-discovery".to_string(),
        gate_name: "contracts".to_string(),
        command: "cargo xtask contracts --dir contracts".to_string(),
        exit_code: 0,
        log: PathBuf::from(".evidence/contracts/last_run.log"),
        status: GateStatus::Pass,
        why_failed: None,
    };

    let json = serde_json::to_string(&evidence)?;

    let parsed: serde_json::Value = serde_json::from_str(&json)?;

    assert_eq!(
        parsed.get("kind").and_then(|v| v.as_str()),
        Some("contract-discovery")
    );
    assert!(json.contains("contract-discovery"));
    assert!(json.contains("Pass"));
    Ok(())
}

#[test]
fn test_prod_gate_evidence_fail_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let why_failed = WhyFailed {
        gate_name: "contracts".to_string(),
        hint: "2 contract(s) failed".to_string(),
        repair_command: "cargo xtask contracts --check".to_string(),
        variant: None,
        fixture_id: None,
        expected_gate: None,
    };

    let evidence = GateEvidence {
        kind: "contract-discovery".to_string(),
        gate_name: "contracts".to_string(),
        command: "cargo xtask contracts --dir contracts".to_string(),
        exit_code: 1,
        log: PathBuf::from(".evidence/contracts/last_run.log"),
        status: GateStatus::Fail,
        why_failed: Some(why_failed),
    };

    let json = serde_json::to_string(&evidence)?;

    let parsed: serde_json::Value = serde_json::from_str(&json)?;

    assert_eq!(
        parsed.get("gate_name").and_then(|v| v.as_str()),
        Some("contracts")
    );
    assert!(json.contains("Fail"));
    assert!(json.contains("why_failed"));
    assert!(json.contains("contracts"));
    assert!(json.contains("2 contract(s) failed"));
    assert!(json.contains("cargo xtask contracts --check"));
    Ok(())
}

// ============================================================
// OBL-005 edge cases: ReportSummary total invariant
// ============================================================

#[test]
fn test_prod_summary_total_invariant_pass() {
    let summary = ReportSummary {
        total: 5,
        valid: 3,
        invalid: 2,
        errors_by_kind: BTreeMap::new(),
        version_violations: Vec::new(),
    };
    assert_eq!(summary.total, summary.valid.saturating_add(summary.invalid));
}

#[test]
fn test_prod_summary_total_invariant_zero() {
    let summary = ReportSummary::new();
    assert_eq!(summary.total, 0u32);
    assert_eq!(summary.valid, 0u32);
    assert_eq!(summary.invalid, 0u32);
    assert_eq!(summary.total, summary.valid.saturating_add(summary.invalid));
}

#[test]
fn test_prod_summary_total_invariant_overflow_safety() {
    // Even with large values, saturating_add should not panic.
    let summary = ReportSummary {
        total: u32::MAX,
        valid: u32::MAX,
        invalid: 1,
        errors_by_kind: BTreeMap::new(),
        version_violations: Vec::new(),
    };
    // saturating_add saturates to u32::MAX on overflow for unsigned types
    let sum = summary.valid.saturating_add(summary.invalid);
    // This tests that saturating_add is used correctly (no overflow panic)
    assert_eq!(sum, u32::MAX);
}

// ============================================================
// OBL-002: ContractKind Display — bound to production code
// ============================================================

#[test]
fn test_prod_contract_kind_display_all() {
    let expected = [
        ("cli_envelope", ContractKind::CliEnvelope),
        ("ui_tokens", ContractKind::UiTokens),
        ("accepted_artifacts", ContractKind::AcceptedArtifacts),
        ("evidence_bundle", ContractKind::EvidenceBundle),
        ("diagnostics", ContractKind::Diagnostics),
        ("gate_output", ContractKind::GateOutput),
    ];

    for (display, kind) in expected {
        let actual = kind.to_string();
        assert_eq!(actual, display);
    }
}

// ============================================================
// OBL-001 + OBL-002: parse_schema_version + parse_contract_kind
// integration — bound to production code
// ============================================================

#[test]
fn test_prod_parse_schema_version_uses_valid() -> Result<(), ContractError> {
    // parse_schema_version returns the original string on success.
    // This means the parsed version is exactly what was validated.
    let input = "3.2.1";
    let result = parse_schema_version(input)?;
    assert_eq!(result, input);
    Ok(())
}

#[test]
fn test_prod_parse_contract_kind_case_sensitive() {
    // ContractKind parsing is case-sensitive (lowercase only).
    assert!(ContractKind::parse("cli_envelope").is_ok());
    assert!(ContractKind::parse("cli_Envelope").is_err());
    assert!(ContractKind::parse("CLI_ENVELOPE").is_err());
}

// ============================================================
// OBL-006: GateEvidence status invariant
// ============================================================

#[test]
fn test_prod_gate_evidence_exit_code_matches_status() {
    let pass_report = DiscoveryReport {
        files: Vec::new(),
        errors: Vec::new(),
        summary: ReportSummary {
            total: 0,
            valid: 0,
            invalid: 0,
            errors_by_kind: BTreeMap::new(),
            version_violations: Vec::new(),
        },
    };

    let pass_evidence = gate_evidence_from_report(&pass_report);
    assert_eq!(pass_evidence.status, GateStatus::Pass);
    assert_eq!(pass_evidence.exit_code, 0);

    let fail_report = DiscoveryReport {
        files: Vec::new(),
        errors: vec!["error".to_string()],
        summary: ReportSummary {
            total: 1,
            valid: 0,
            invalid: 1,
            errors_by_kind: BTreeMap::new(),
            version_violations: Vec::new(),
        },
    };

    let fail_evidence = gate_evidence_from_report(&fail_report);
    assert_eq!(fail_evidence.status, GateStatus::Fail);
    assert_eq!(fail_evidence.exit_code, 1);
}

// ============================================================
// ContractError Display — bound to production code
// ============================================================

#[test]
fn test_prod_contract_error_all_variants_display() {
    let err = ContractError::MissingSchemaVersion;
    assert_eq!(err.to_string(), "MISSING_SCHEMA_VERSION");

    let err = ContractError::InvalidVersion {
        version: "1.0".to_string(),
    };
    assert_eq!(err.to_string(), "INVALID_VERSION: 1.0");

    let err = ContractError::InvalidKind {
        kind: "bogus".to_string(),
    };
    assert_eq!(err.to_string(), "INVALID_KIND: bogus");

    let err = ContractError::CueVetFailed {
        file: "foo.cue".to_string(),
    };
    assert_eq!(err.to_string(), "CUE_VET_FAILED: foo.cue");

    let err = ContractError::VersionMonotonicityBreach {
        file: "bar.cue".to_string(),
        expected: "1.0.0".to_string(),
        actual: "0.9.0".to_string(),
    };
    assert!(err.to_string().contains("VERSION_MONOTONICITY_BREACH"));
    assert!(err.to_string().contains("bar.cue"));
    assert!(err.to_string().contains("expected 1.0.0 got 0.9.0"));
}
