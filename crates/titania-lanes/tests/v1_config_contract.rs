//! Contract tests for the strict workspace lint configuration.

use std::collections::BTreeMap;
use titania_lanes::{LaneReport, policy_scan::exceptions::parse_exception_content};
use titania_policy::{ExceptionError, parse_exceptions};

const ROOT_CARGO: &str = include_str!("../../../Cargo.toml");
const CLIPPY: &str = include_str!("../../../clippy.toml");

const REQUIRED_RUST_LINTS: &[(&str, &str)] = &[
    ("unsafe_code", "forbid"),
    ("unused_must_use", "deny"),
    ("unreachable_pub", "deny"),
    ("missing_docs", "deny"),
    ("unsafe_op_in_unsafe_fn", "deny"),
];

const REQUIRED_RUST_TABLE_LINTS: &[(&str, &str, &str)] = &[("rust_2018_idioms", "deny", "-1")];

const REQUIRED_CLIPPY_LINTS: &[(&str, &str)] = &[
    ("unwrap_used", "deny"),
    ("expect_used", "deny"),
    ("panic", "deny"),
    ("todo", "deny"),
    ("unimplemented", "deny"),
    ("indexing_slicing", "deny"),
    ("string_slice", "deny"),
    ("get_unwrap", "deny"),
    ("arithmetic_side_effects", "deny"),
    ("dbg_macro", "deny"),
    ("as_conversions", "deny"),
    ("await_holding_lock", "deny"),
    ("missing_errors_doc", "deny"),
    ("panic_in_result_fn", "deny"),
    ("print_stdout", "deny"),
    ("print_stderr", "deny"),
    ("integer_division", "deny"),
    ("integer_division_remainder_used", "deny"),
    ("modulo_arithmetic", "deny"),
    ("float_arithmetic", "deny"),
    ("allow_attributes", "deny"),
    ("allow_attributes_without_reason", "deny"),
    ("result_large_err", "deny"),
    ("result_unit_err", "deny"),
    ("map_err_ignore", "deny"),
    ("missing_panics_doc", "deny"),
    ("missing_safety_doc", "deny"),
    ("large_enum_variant", "deny"),
    ("cognitive_complexity", "deny"),
    ("too_many_arguments", "deny"),
    ("too_many_lines", "deny"),
    ("type_complexity", "deny"),
    ("excessive_nesting", "deny"),
    ("await_holding_refcell_ref", "deny"),
    ("future_not_send", "deny"),
    ("large_futures", "deny"),
    ("disallowed_methods", "deny"),
    ("disallowed_macros", "deny"),
    ("disallowed_types", "deny"),
    ("disallowed_fields", "deny"),
    ("multiple_crate_versions", "deny"),
    ("wildcard_dependencies", "deny"),
    ("negative_feature_names", "deny"),
    ("redundant_feature_names", "deny"),
];

const REQUIRED_CLIPPY_TABLE_LINTS: &[(&str, &str, &str)] = &[
    ("all", "deny", "-1"),
    ("pedantic", "deny", "-1"),
    ("nursery", "deny", "-1"),
    ("cargo", "deny", "-1"),
];

const REQUIRED_CLIPPY_THRESHOLDS: &[(&str, &str)] = &[
    ("too-many-lines-threshold", "40"),
    ("too-many-arguments-threshold", "5"),
    ("cognitive-complexity-threshold", "8"),
    ("excessive-nesting-threshold", "2"),
    ("type-complexity-threshold", "120"),
    ("max-fn-params-bools", "1"),
];

#[test]
fn workspace_lints() {
    let rust_lints = table(ROOT_CARGO, "workspace.lints.rust");
    let clippy_lints = table(ROOT_CARGO, "workspace.lints.clippy");
    let clippy_thresholds = flat_entries(CLIPPY);

    assert_scalar_lints(&rust_lints, REQUIRED_RUST_LINTS);
    assert_table_lints(&rust_lints, REQUIRED_RUST_TABLE_LINTS);
    assert_scalar_lints(&clippy_lints, REQUIRED_CLIPPY_LINTS);
    assert_table_lints(&clippy_lints, REQUIRED_CLIPPY_TABLE_LINTS);
    assert_entries(&clippy_thresholds, REQUIRED_CLIPPY_THRESHOLDS);
}

#[test]
fn workspace_lints_reject_weakened_fixture() {
    let weakened = ROOT_CARGO.replace("unwrap_used = \"deny\"", "unwrap_used = \"allow\"");
    let clippy_lints = table(&weakened, "workspace.lints.clippy");

    assert_eq!(scalar_lint(&clippy_lints, "unwrap_used"), Some("allow"));
    assert_ne!(scalar_lint(&clippy_lints, "unwrap_used"), Some("deny"));
}

#[test]
fn clippy_thresholds_reject_weakened_fixture() {
    // Weaken the checked-in baseline (40) to 80 in a copy of clippy.toml.
    let weakened = CLIPPY.replace("too-many-lines-threshold = 40", "too-many-lines-threshold = 80");
    let clippy_thresholds = flat_entries(&weakened);

    // The weakened fixture must actually carry the weakened value, proving the
    // replace targeted the real baseline entry rather than silently no-op'ing.
    assert_eq!(clippy_thresholds.get("too-many-lines-threshold"), Some(&"80"));

    // The contract gate requires the baseline value from
    // REQUIRED_CLIPPY_THRESHOLDS; the weakened fixture's value must differ from
    // it, i.e. the same `assert_entries` check used in `workspace_lints` would
    // reject this weakened fixture as a contract violation.
    let required_threshold = REQUIRED_CLIPPY_THRESHOLDS
        .iter()
        .copied()
        .find(|(key, _)| *key == "too-many-lines-threshold")
        .map(|(_, value)| value);
    assert_eq!(required_threshold, Some("40"));
    assert_ne!(clippy_thresholds.get("too-many-lines-threshold").copied(), required_threshold);
}

fn assert_scalar_lints(entries: &BTreeMap<&str, &str>, required: &[(&str, &str)]) {
    required.iter().for_each(|(key, level)| {
        assert_eq!(scalar_lint(entries, key), Some(*level), "{key} must be {level}");
    });
}

fn assert_table_lints(entries: &BTreeMap<&str, &str>, required: &[(&str, &str, &str)]) {
    required.iter().for_each(|(key, level, priority)| {
        assert_eq!(
            table_lint(entries, key),
            Some((*level, *priority)),
            "{key} must be {level}/{priority}"
        );
    });
}

fn assert_entries(entries: &BTreeMap<&str, &str>, required: &[(&str, &str)]) {
    required.iter().for_each(|(key, value)| {
        assert_eq!(entries.get(key), Some(value), "{key} must be {value}");
    });
}

fn scalar_lint<'a>(entries: &'a BTreeMap<&str, &'a str>, key: &str) -> Option<&'a str> {
    entries.get(key).and_then(|value| quoted(value))
}

fn table_lint<'a>(entries: &'a BTreeMap<&str, &'a str>, key: &str) -> Option<(&'a str, &'a str)> {
    let value = entries.get(key)?;
    let fields = inline_table(value)?;
    Some((quoted(fields.get("level")?)?, fields.get("priority")?))
}

fn table<'a>(text: &'a str, section: &str) -> BTreeMap<&'a str, &'a str> {
    text.lines()
        .scan(false, |inside, line| Some(section_line_entries(inside, section, line)))
        .flatten()
        .collect()
}

fn section_line_entries<'a>(
    inside: &mut bool,
    section: &str,
    line: &'a str,
) -> Option<(&'a str, &'a str)> {
    let trimmed = line.split('#').next()?.trim();
    if trimmed.starts_with('[') {
        *inside = trimmed == format!("[{section}]");
        return None;
    }
    inside.then(|| split_entry(trimmed)).flatten()
}

fn flat_entries(text: &str) -> BTreeMap<&str, &str> {
    text.lines().filter_map(|line| split_entry(line.split('#').next()?.trim())).collect()
}

fn split_entry(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once('=')?;
    Some((key.trim(), value.trim()))
}

fn quoted(value: &str) -> Option<&str> {
    value.trim().strip_prefix('"')?.strip_suffix('"')
}

fn inline_table(value: &str) -> Option<BTreeMap<&str, &str>> {
    let inner = value.trim().strip_prefix('{')?.strip_suffix('}')?;
    Some(inner.split(',').filter_map(|part| split_entry(part.trim())).collect())
}

// ── strict-ai exceptions.toml contract tests ──────────────────────────

const EXCEPTIONS: &str = include_str!("../../../.titania/profiles/strict-ai/exceptions.toml");
const POLICY_TODAY: &str = "2026-07-05";

#[test]
fn strict_ai_exceptions_all_fields_present() {
    let exceptions =
        parse_exceptions(EXCEPTIONS, POLICY_TODAY).expect("checked-in exceptions.toml must parse");

    assert_eq!(exceptions.len(), 1, "expected exactly one reviewed self exception");

    exceptions.iter().for_each(|exception| {
        assert!(!exception.rule_id.as_str().is_empty(), "rule_id must be present");
        assert!(!exception.path.as_str().is_empty(), "path must be present");
        assert!(!exception.owner.is_empty(), "owner must be present");
        assert!(!exception.reason.is_empty(), "reason must be present");
        assert!(!exception.expires_on.is_empty(), "expires_on must be present");
        assert!(!exception.review.is_empty(), "review must be present");
    });
}

#[test]
fn strict_ai_exceptions_metadata_matches_audit() {
    let exceptions =
        parse_exceptions(EXCEPTIONS, POLICY_TODAY).expect("checked-in exceptions.toml must parse");
    let exception = exceptions.first().expect("one audited exception must exist");

    assert_eq!(exception.rule_id.as_str(), "BYPASS_EXPECT_ATTR");
    assert_eq!(exception.path.as_str(), "crates/titania-dylint/src/lib.rs");
    assert_eq!(exception.owner.as_ref(), "titania-maintainers");
    assert_eq!(
        exception.reason.as_ref(),
        "Dylint ABI exports require audited #[expect(unsafe_code)] on unsafe no_mangle exports, and register_lints also requires #[expect(clippy::no_mangle_with_rust_abi)] because Dylint documents a Rust-ABI no_mangle registration hook."
    );
    assert_eq!(exception.expires_on.as_ref(), "2026-10-01");
    assert_eq!(exception.review.as_ref(), "tn-dylint-abi-expect");
}

#[test]
fn strict_ai_exceptions_expired_fixture_rejected_by_policy_parser() {
    let err = parse_exceptions(expired_exception_fixture(), POLICY_TODAY)
        .expect_err("expired exception must fail through production parser");

    assert_eq!(
        err,
        ExceptionError::ExceptionExpired {
            rule_id: Box::<str>::from("BYPASS_EXPECT_ATTR"),
            expires_on: Box::<str>::from("2020-01-01"),
            today: Box::<str>::from(POLICY_TODAY),
        }
    );
    assert_eq!(err.code(), "POLICY_EXCEPTION_EXPIRED");
}

#[test]
fn strict_ai_exceptions_expired_fixture_emits_policy_finding() {
    let mut report = LaneReport::new();
    let exceptions =
        parse_exception_content(expired_exception_fixture(), POLICY_TODAY, &mut report)
            .expect("expired exception diagnostic rule id must be valid");

    assert!(exceptions.is_empty(), "expired exceptions must not become active suppressions");
    assert_eq!(report.finding_count(), 1, "expired exception must emit one policy finding");

    let finding = report.findings().first().expect("one expired-exception finding must exist");
    assert_eq!(finding.rule().as_str(), "POLICY_EXCEPTION_EXPIRED");
    assert_eq!(finding.path(), ".titania/profiles/strict-ai/exceptions.toml");
    assert_eq!(finding.line(), 0);
    assert!(
        finding.message().contains("BYPASS_EXPECT_ATTR"),
        "finding must name the expired rule, got: {}",
        finding.message()
    );
}

fn expired_exception_fixture() -> &'static str {
    r#"
[[exceptions]]
rule_id = "BYPASS_EXPECT_ATTR"
path = "crates/titania-dylint/src/lib.rs"
owner = "titania-maintainers"
reason = "Dylint ABI exports require audited temporary exception"
expires_on = "2020-01-01"
review = "tn-dylint-abi-expect"
"#
}
