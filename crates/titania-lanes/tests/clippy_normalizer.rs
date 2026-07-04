//! Tests for the Clippy JSON normalizer lane.
//!
//! These tests exercise the normalizer against JSONL fixtures that represent
//! real `cargo clippy -Z json` output.  They assert exact rule IDs, span
//! locations, message content, and the malformed-only failure path.
//!
//! The tests are the failing-first contract for bead tn-d2l.2.

use std::path::Path;

/// Resolve the absolute path to a clippy fixture by name.
fn fixture_path(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join("clippy").join(name)
}

/// Read the full text of a fixture file.
fn read_fixture(name: &str) -> String {
    std::fs::read_to_string(fixture_path(name))
        .unwrap_or_else(|e| panic!("cannot read fixture {name}: {e}"))
}

// ---------------------------------------------------------------------------
// unwrap.jsonl — `clippy::unwrap_used` lints
// ---------------------------------------------------------------------------

#[test]
fn clippy_normalizer_unwrap_fixture_maps_to_unwrap_used_rule() {
    // The production module must exist and provide `normalize_clippy_jsonl`.
    let result =
        titania_lanes::clippy_normalizer::normalize_clippy_jsonl(&read_fixture("unwrap.jsonl"));

    // Exactly two findings for two unwrap_used spans.
    assert_eq!(result.finding_count(), 2, "unwrap fixture must yield exactly two findings");

    // First finding: src/lib.rs:3, rule CLIPPY_UNWRAP_USED.
    let f0 = &result.findings()[0];
    assert_eq!(f0.rule().as_str(), "CLIPPY_UNWRAP_USED");
    assert_eq!(f0.path(), "src/lib.rs");
    assert_eq!(f0.line(), 3);
    assert!(f0.message().contains("unwrap"), "message must mention unwrap");

    // Second finding: src/lib.rs:7, same rule.
    let f1 = &result.findings()[1];
    assert_eq!(f1.rule().as_str(), "CLIPPY_UNWRAP_USED");
    assert_eq!(f1.path(), "src/lib.rs");
    assert_eq!(f1.line(), 7);
}

// ---------------------------------------------------------------------------
// warning_only.jsonl — non-unwrap clippy warning → typed CLIPPY_* finding
// ---------------------------------------------------------------------------

#[test]
fn clippy_normalizer_warning_only_fixture_maps_to_typed_rule() {
    let result = titania_lanes::clippy_normalizer::normalize_clippy_jsonl(&read_fixture(
        "warning_only.jsonl",
    ));

    assert_eq!(result.finding_count(), 1, "warning-only fixture must yield exactly one finding");

    let f = &result.findings()[0];
    // Must start with CLIPPY_ prefix (typed clippy finding).
    assert!(
        f.rule().as_str().starts_with("CLIPPY_"),
        "rule must have CLIPPY_ prefix, got {}",
        f.rule().as_str()
    );
    assert_eq!(f.path(), "src/lib.rs");
    assert_eq!(f.line(), 20);
    // The original lint name should be preserved in the message.
    assert!(
        f.message().contains("too_many_lines") || f.message().contains("too many lines"),
        "message must reference the original lint"
    );
}

// ---------------------------------------------------------------------------
// malformed_only.jsonl — all lines broken → suspicious failure, never clean
// ---------------------------------------------------------------------------

#[test]
fn clippy_normalizer_malformed_fixture_returns_suspicious_failure_not_clean() {
    let result = titania_lanes::clippy_normalizer::normalize_clippy_jsonl(&read_fixture(
        "malformed_only.jsonl",
    ));

    // The normalizer must NEVER return a clean report for malformed input.
    assert!(
        !result.is_clean(),
        "malformed fixture must produce a suspicious finding, not a clean report"
    );

    // At least one finding should exist with the tool name and a
    // suspicious/failure keyword.
    assert!(result.render().contains("cargo clippy"), "rendered output should flag the tool name");

    // No valid findings should have a CLIPPY_ prefix (nothing was parsed).
    for f in result.findings() {
        assert!(
            !f.rule().as_str().starts_with("CLIPPY_"),
            "malformed input must not produce valid clippy findings"
        );
    }
}

// ---------------------------------------------------------------------------
// unknown_lint.jsonl — unrecognized lint → CLIPPY_UNKNOWN with original name
// ---------------------------------------------------------------------------

#[test]
fn clippy_normalizer_unknown_lint_fixture_maps_to_unknown_rule() {
    let result = titania_lanes::clippy_normalizer::normalize_clippy_jsonl(&read_fixture(
        "unknown_lint.jsonl",
    ));

    assert_eq!(result.finding_count(), 1, "unknown-lint fixture must yield exactly one finding");

    let f = &result.findings()[0];
    assert_eq!(f.rule().as_str(), "CLIPPY_UNKNOWN");
    assert_eq!(f.path(), "src/lib.rs");
    assert_eq!(f.line(), 42);
    // The original lint name must appear in the message for traceability.
    assert!(f.message().contains("hypothetical_lint"), "message must contain original lint name");
}
