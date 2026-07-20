//! v1.5 contract tests for `MutantsBaseline::parse_str` error surface.

use std::path::PathBuf;

use titania_core::{
    MutantBaselineEntry, MutantId, MutantOperator, MutantsBaseline, MutantsBaselineError,
};

fn fixture_path(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("fixtures");
    p.push(name);
    p
}

fn read_fixture(name: &str) -> (String, String) {
    let path = fixture_path(name);
    let label = path.display().to_string();
    let contents = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("fixture `{}` unreadable: {error}", path.display()));
    (label, contents)
}

#[test]
fn happy_path_loads_empty_baseline() {
    let (label, contents) = read_fixture("v15_mutants_baseline_empty.json");
    let baseline = MutantsBaseline::parse_str(&contents, &label).unwrap();
    assert_eq!(baseline.schema_version(), 1);
    assert_eq!(baseline.entries().len(), 0);
}

#[test]
fn malformed_json_returns_parse_error() {
    let (label, contents) = read_fixture("v15_mutants_baseline_malformed.json");
    let err = MutantsBaseline::parse_str(&contents, &label).unwrap_err();
    assert!(matches!(err, MutantsBaselineError::JsonParse { .. }));
}

#[test]
fn wrong_schema_version_returns_unsupported() {
    let (label, contents) = read_fixture("v15_mutants_baseline_wrong_version.json");
    let err = MutantsBaseline::parse_str(&contents, &label).unwrap_err();
    assert!(
        matches!(err, MutantsBaselineError::UnsupportedSchemaVersion { found: 999, .. }),
        "got {err:?}"
    );
}

#[test]
fn rejects_wildcard_mutation_id() {
    let (label, contents) = read_fixture("v15_mutants_baseline_wildcard.json");
    let err = MutantsBaseline::parse_str(&contents, &label).unwrap_err();
    assert!(
        matches!(err, MutantsBaselineError::JsonParse { .. }),
        "wildcard mutation_id is a malformed MutantId and must surface a JSON parse failure; got {err:?}"
    );
}

#[test]
fn rejects_accepted_by_rule_without_owner() {
    let (label, contents) = read_fixture("v15_mutants_baseline_bad_rule.json");
    let err = MutantsBaseline::parse_str(&contents, &label).unwrap_err();
    assert!(matches!(err, MutantsBaselineError::InvalidAcceptedByRule { .. }), "got {err:?}");
}

#[test]
fn rejects_accepted_by_rule_with_extra_segment() {
    // `mutant-accept/owner/reason/expiry/extra` carries one more `/`
    // segment than the contract family allows.
    let err = MutantsBaseline::parse_str(
        r#"{
            "schema_version": 1,
            "entries": [
                {
                    "mutation_id": "pkg::src/foo.rs:1:1:equal_replace",
                    "accepted_by_rule": "mutant-accept/owner-a/reason-a/never/garbage",
                    "reason": "test reason",
                    "expires_on_unix": null
                }
            ]
        }"#,
        "<inline>",
    )
    .unwrap_err();
    assert!(matches!(err, MutantsBaselineError::InvalidAcceptedByRule { .. }), "got {err:?}");
}

#[test]
fn rejects_accepted_by_rule_with_zero_expiry() {
    let err = MutantsBaseline::parse_str(
        r#"{
            "schema_version": 1,
            "entries": [
                {
                    "mutation_id": "pkg::src/foo.rs:1:1:equal_replace",
                    "accepted_by_rule": "mutant-accept/owner-a/reason-a/0",
                    "reason": "test reason",
                    "expires_on_unix": null
                }
            ]
        }"#,
        "<inline>",
    )
    .unwrap_err();
    assert!(matches!(err, MutantsBaselineError::InvalidAcceptedByRule { .. }), "got {err:?}");
}

#[test]
fn rejects_accepted_by_rule_with_overflow_expiry() {
    // 21 decimal digits exceeds `u64::MAX` (max 19 digits).
    let overflow = "9".repeat(21);
    let err = MutantsBaseline::parse_str(
        r#"{
            "schema_version": 1,
            "entries": [
                {
                    "mutation_id": "pkg::src/foo.rs:1:1:equal_replace",
                    "accepted_by_rule": "mutant-accept/owner-a/reason-a/EXPIRY",
                    "reason": "test reason",
                    "expires_on_unix": null
                }
            ]
        }"#
        .replace("EXPIRY", &overflow)
        .as_str(),
        "<inline>",
    )
    .unwrap_err();
    assert!(matches!(err, MutantsBaselineError::InvalidAcceptedByRule { .. }), "got {err:?}");
}

#[test]
fn rejects_empty_reason() {
    let err = MutantsBaseline::parse_str(
        r#"{
            "schema_version": 1,
            "entries": [
                {
                    "mutation_id": "pkg::src/foo.rs:1:1:equal_replace",
                    "accepted_by_rule": "mutant-accept/owner-a/reason-a/never",
                    "reason": "",
                    "expires_on_unix": null
                }
            ]
        }"#,
        "<inline>",
    )
    .unwrap_err();
    assert!(matches!(err, MutantsBaselineError::InvalidReason { .. }), "got {err:?}");
}

#[test]
fn rejects_whitespace_only_reason() {
    let err = MutantsBaseline::parse_str(
        r#"{
            "schema_version": 1,
            "entries": [
                {
                    "mutation_id": "pkg::src/foo.rs:1:1:equal_replace",
                    "accepted_by_rule": "mutant-accept/owner-a/reason-a/never",
                    "reason": "   \t  \n ",
                    "expires_on_unix": null
                }
            ]
        }"#,
        "<inline>",
    )
    .unwrap_err();
    assert!(matches!(err, MutantsBaselineError::InvalidReason { .. }), "got {err:?}");
}

#[test]
fn accepts_reason_with_visible_content() {
    let baseline = MutantsBaseline::parse_str(
        r#"{
            "schema_version": 1,
            "entries": [
                {
                    "mutation_id": "pkg::src/foo.rs:1:1:equal_replace",
                    "accepted_by_rule": "mutant-accept/owner-a/reason-a/never",
                    "reason": "  visible  ",
                    "expires_on_unix": null
                }
            ]
        }"#,
        "<inline>",
    );
    assert!(baseline.is_ok(), "got {baseline:?}");
}

#[test]
fn from_bypasses_round_trip_via_json() {
    let entries = vec![MutantBaselineEntry {
        mutation_id: MutantId::new("pkg", "src/foo.rs", 1, 1, MutantOperator::EqualReplace)
            .unwrap(),
        accepted_by_rule: "mutant-accept/le owner/test reason/never".to_owned(),
        reason: "test reason".to_owned(),
        expires_on_unix: None,
    }];
    let baseline = MutantsBaseline::from_bypasses(entries);
    let json = serde_json::to_string(&baseline).unwrap();
    let back: MutantsBaseline = serde_json::from_str(&json).unwrap();
    assert_eq!(back, baseline);
}
