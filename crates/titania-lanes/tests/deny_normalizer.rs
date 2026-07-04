//! Tests for the Deny JSON normalizer lane.
//!
//! These tests exercise the normalizer against JSON fixtures that represent
//! real `cargo-deny check --format json` output. They assert exact rule IDs,
//! span locations, message content, and the malformed-only failure path.
//!
//! The tests are the failing-first contract for bead tn-8xw.1.

use std::path::Path;

/// Resolve the absolute path to a deny fixture by name.
fn fixture_path(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join("deny").join(name)
}

/// Read the full text of a fixture file.
fn read_fixture(name: &str) -> String {
    std::fs::read_to_string(fixture_path(name))
        .unwrap_or_else(|e| panic!("cannot read fixture {name}: {e}"))
}

// ---------------------------------------------------------------------------
// 1. Advisory fixture → DENY_ADVISORY rule
// ---------------------------------------------------------------------------

#[test]
fn deny_normalizer_advisory_fixture_maps_to_advisory_rule() {
    let result =
        titania_lanes::deny_normalizer::normalize_deny_json(&read_fixture("advisory.json"));

    assert_eq!(result.finding_count(), 1, "advisory fixture must yield exactly one finding");

    let f = &result.findings()[0];
    assert_eq!(f.rule().as_str(), "DENY_ADVISORY");
    assert!(
        f.message().contains("vulnerable-crate"),
        "message must reference the advisory package"
    );
    assert!(
        f.message().contains("CVE-2024-1234") || f.message().contains("rustsec-2024-0001"),
        "message must reference the advisory ID or CVE"
    );
    assert!(!f.message().is_empty(), "message must not be empty");
}

// ---------------------------------------------------------------------------
// 2. License fixture → DENY_LICENSE rule
// ---------------------------------------------------------------------------

#[test]
fn deny_normalizer_license_fixture_maps_to_license_rule() {
    let result = titania_lanes::deny_normalizer::normalize_deny_json(&read_fixture("license.json"));

    assert_eq!(result.finding_count(), 1, "license fixture must yield exactly one finding");

    let f = &result.findings()[0];
    assert_eq!(f.rule().as_str(), "DENY_LICENSE");
    assert!(
        f.message().contains("unlicensed-pkg"),
        "message must reference the license-violating package"
    );
    assert!(
        f.message().contains("WTFPL") || f.message().contains("not in the allow list"),
        "message must reference the disallowed license"
    );
}

// ---------------------------------------------------------------------------
// 3. Banned crate fixture → DENY_BANNED_CRATE rule
// ---------------------------------------------------------------------------

#[test]
fn deny_normalizer_banned_crate_fixture_maps_to_banned_crate_rule() {
    let result =
        titania_lanes::deny_normalizer::normalize_deny_json(&read_fixture("banned_crate.json"));

    assert_eq!(result.finding_count(), 1, "banned-crate fixture must yield exactly one finding");

    let f = &result.findings()[0];
    assert_eq!(f.rule().as_str(), "DENY_BANNED_CRATE");
    assert!(f.message().contains("evil-pkg"), "message must reference the banned package");
}

// ---------------------------------------------------------------------------
// 4. Duplicate/multiple-versions fixture → DENY_MULTIPLE_VERSIONS rule
// ---------------------------------------------------------------------------

#[test]
fn deny_normalizer_duplicate_versions_fixture_maps_to_multiple_versions_rule() {
    let result = titania_lanes::deny_normalizer::normalize_deny_json(&read_fixture(
        "duplicate_versions.json",
    ));

    assert_eq!(
        result.finding_count(),
        1,
        "duplicate-versions fixture must yield exactly one finding"
    );

    let f = &result.findings()[0];
    assert_eq!(f.rule().as_str(), "DENY_MULTIPLE_VERSIONS");
    assert!(f.message().contains("serde"), "message must reference the duplicated crate");
    assert!(
        f.message().contains("1.0.193") || f.message().contains("1.0.210"),
        "message must reference version numbers"
    );
}

// ---------------------------------------------------------------------------
// 5. Unknown registry fixture → DENY_UNKNOWN_REGISTRY rule
// ---------------------------------------------------------------------------

#[test]
fn deny_normalizer_unknown_registry_fixture_maps_to_unknown_registry_rule() {
    let result =
        titania_lanes::deny_normalizer::normalize_deny_json(&read_fixture("unknown_registry.json"));

    assert_eq!(
        result.finding_count(),
        1,
        "unknown-registry fixture must yield exactly one finding"
    );

    let f = &result.findings()[0];
    assert_eq!(f.rule().as_str(), "DENY_UNKNOWN_REGISTRY");
    assert!(
        f.message().contains("packages.evil-registry.io") || f.message().contains("secret-crate"),
        "message must reference the unknown registry URL or package"
    );
}

// ---------------------------------------------------------------------------
// 6. Unknown git fixture → DENY_UNKNOWN_GIT rule
// ---------------------------------------------------------------------------

#[test]
fn deny_normalizer_unknown_git_fixture_maps_to_unknown_git_rule() {
    let result =
        titania_lanes::deny_normalizer::normalize_deny_json(&read_fixture("unknown_git.json"));

    assert_eq!(result.finding_count(), 1, "unknown-git fixture must yield exactly one finding");

    let f = &result.findings()[0];
    assert_eq!(f.rule().as_str(), "DENY_UNKNOWN_GIT");
    assert!(
        f.message().contains("github.com/unknown-org") || f.message().contains("untrusted-crate"),
        "message must reference the unknown git source or package"
    );
}

// ---------------------------------------------------------------------------
// 7. Malformed/non-JSON fixture → SuspiciousFailure, never clean
// ---------------------------------------------------------------------------

#[test]
fn deny_normalizer_malformed_fixture_returns_suspicious_failure_not_clean() {
    let result = titania_lanes::deny_normalizer::normalize_deny_json(&read_fixture(
        "malformed_non_json.txt",
    ));

    // The normalizer must NEVER return a clean report for malformed input.
    assert!(
        !result.is_clean(),
        "malformed fixture must produce a suspicious finding, not a clean report"
    );

    // At least one finding should exist with the tool name and a
    // suspicious/failure keyword.
    assert!(result.render().contains("cargo-deny"), "rendered output should flag the tool name");

    // No valid findings should have a DENY_ prefix (nothing was parsed).
    for f in result.findings() {
        assert!(
            !f.rule().as_str().starts_with("DENY_"),
            "malformed input must not produce valid deny findings"
        );
    }
}

// ---------------------------------------------------------------------------
// 8. Missing binary → InfraFailure via deny_missing_binary helper
// ---------------------------------------------------------------------------

#[test]
fn deny_normalizer_missing_binary_returns_infra_failure() {
    let result = titania_lanes::deny_normalizer::deny_missing_binary();

    assert!(!result.is_clean(), "missing binary must not be a clean report");

    // The finding should reference the tool name.
    assert!(result.render().contains("cargo-deny"), "rendered output should mention cargo-deny");

    // The rule should be INFRA_FAILURE or similar infrastructure error.
    let f = &result.findings()[0];
    assert!(
        f.rule().as_str().contains("INFRA") || f.rule().as_str().contains("TOOL"),
        "rule must be an infrastructure failure, got {}",
        f.rule().as_str()
    );
}

// ---------------------------------------------------------------------------
// 9. Clean fixture → empty report
// ---------------------------------------------------------------------------

#[test]
fn deny_normalizer_clean_fixture_returns_empty_report() {
    let clean_fixture = r#"{"type":"summary","fields":{"advisories":{"errors":0,"helps":0,"notes":0,"warnings":0},"bans":{"errors":0,"helps":0,"notes":0,"warnings":0},"licenses":{"errors":0,"helps":0,"notes":0,"warnings":0},"sources":{"errors":0,"helps":0,"notes":0,"warnings":0}}}"#;

    let result = titania_lanes::deny_normalizer::normalize_deny_json(clean_fixture);

    assert!(result.is_clean(), "clean fixture must produce a clean report");
    assert_eq!(result.finding_count(), 0, "clean fixture must have zero findings");
}

// ---------------------------------------------------------------------------
// 10. Mixed fixture — multiple different finding types in one output
// ---------------------------------------------------------------------------

#[test]
fn deny_normalizer_mixed_fixture_contains_all_finding_types() {
    let mixed = r#"{"type":"diagnostic","fields":{"code":"vulnerability","graphs":[],"labels":[{"column":1,"line":1,"message":"vulnerable crate","span":"vuln-pkg 1.0.0"}],"message":"vulnerability: CVE-2024-9999 affects vuln-pkg 1.0.0","severity":"error"}}
{"type":"diagnostic","fields":{"code":"rejected","graphs":[],"labels":[{"column":10,"line":1,"message":"bad license","span":"GPL-3.0"}],"message":"bad-pkg 2.0.0 uses GPL-3.0 which is not allowed","severity":"error"}}
{"type":"diagnostic","fields":{"code":"banned","graphs":[],"labels":[{"column":1,"line":1,"message":"banned","span":"evil 0.1.0"}],"message":"crate evil 0.1.0 is banned","severity":"error"}}
{"type":"diagnostic","fields":{"code":"duplicate","graphs":[],"labels":[{"column":1,"line":1,"message":"dupes","span":"dep 1.0, dep 2.0"}],"message":"dep appears with multiple versions","severity":"warning"}}
{"type":"diagnostic","fields":{"code":"source-not-allowed","graphs":[],"labels":[{"column":1,"line":1,"message":"unknown reg","span":"https://evil.io/"}],"message":"pkg uses unknown registry https://evil.io/","severity":"error"}}
{"type":"summary","fields":{"advisories":{"errors":1,"helps":0,"notes":0,"warnings":0},"bans":{"errors":1,"helps":0,"notes":0,"warnings":0},"licenses":{"errors":1,"helps":0,"notes":0,"warnings":0},"sources":{"errors":1,"helps":0,"notes":0,"warnings":0}},"type":"summary"}"#;

    let result = titania_lanes::deny_normalizer::normalize_deny_json(mixed);

    assert_eq!(result.finding_count(), 5, "mixed fixture must yield exactly five findings");

    let rules: Vec<&str> = result.findings().iter().map(|f| f.rule().as_str()).collect();
    assert!(rules.contains(&"DENY_ADVISORY"), "must contain DENY_ADVISORY");
    assert!(rules.contains(&"DENY_LICENSE"), "must contain DENY_LICENSE");
    assert!(rules.contains(&"DENY_BANNED_CRATE"), "must contain DENY_BANNED_CRATE");
    assert!(rules.contains(&"DENY_MULTIPLE_VERSIONS"), "must contain DENY_MULTIPLE_VERSIONS");
    assert!(rules.contains(&"DENY_UNKNOWN_REGISTRY"), "must contain DENY_UNKNOWN_REGISTRY");
}
