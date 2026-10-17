use std::path::Path;

fn fixture_path(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join("clippy").join(name)
}

fn read_fixture(name: &str) -> String {
    std::fs::read_to_string(fixture_path(name))
        .unwrap_or_else(|e| panic!("cannot read fixture {name}: {e}"))
}

#[test]
fn clippy_normalizer_unwrap_fixture_maps_to_unwrap_used_rule() {
    let result =
        titania_lanes::clippy_normalizer::normalize_clippy_jsonl(&read_fixture("unwrap.jsonl"));

    assert_eq!(result.finding_count(), 2, "unwrap fixture must yield exactly two findings");

    let f0 = &result.findings()[0];
    assert_eq!(f0.rule().as_str(), "CLIPPY_UNWRAP_USED");
    assert_eq!(f0.path(), "src/lib.rs");
    assert_eq!(f0.line(), 3);
    assert!(f0.message().contains("unwrap"), "message must mention unwrap");

    let f1 = &result.findings()[1];
    assert_eq!(f1.rule().as_str(), "CLIPPY_UNWRAP_USED");
    assert_eq!(f1.path(), "src/lib.rs");
    assert_eq!(f1.line(), 7);
}

#[test]
fn clippy_normalizer_warning_only_fixture_maps_to_typed_rule() {
    let result = titania_lanes::clippy_normalizer::normalize_clippy_jsonl(&read_fixture(
        "warning_only.jsonl",
    ));

    assert_eq!(result.finding_count(), 1, "warning-only fixture must yield exactly one finding");

    let f = &result.findings()[0];
    assert!(
        f.rule().as_str().starts_with("CLIPPY_"),
        "rule must have CLIPPY_ prefix, got {}",
        f.rule().as_str()
    );
    assert_eq!(f.path(), "src/lib.rs");
    assert_eq!(f.line(), 20);
    assert!(
        f.message().contains("too_many_lines") || f.message().contains("too many lines"),
        "message must reference the original lint"
    );
}

#[test]
fn clippy_normalizer_malformed_fixture_returns_suspicious_failure_not_clean() {
    let result = titania_lanes::clippy_normalizer::normalize_clippy_jsonl(&read_fixture(
        "malformed_only.jsonl",
    ));

    assert!(
        !result.is_clean(),
        "malformed fixture must produce a suspicious finding, not a clean report"
    );

    assert!(result.render().contains("cargo clippy"), "rendered output should flag the tool name");

    for f in result.findings() {
        assert!(
            !f.rule().as_str().starts_with("CLIPPY_"),
            "malformed input must not produce valid clippy findings"
        );
    }
}

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
    assert!(f.message().contains("hypothetical_lint"), "message must contain original lint name");
}
