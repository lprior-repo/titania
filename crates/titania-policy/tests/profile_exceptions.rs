//! Policy and exception parser behavior tests.

use titania_policy::{ExceptionError, ProfileError, parse_exceptions, parse_profile};

#[test]
fn valid_policy_minimal_parses_architecture() {
    let policy = parse_profile(
        r#"
[architecture]
core_dirs = ["src/core", "src/domain"]
infra_crates = ["tokio", "sqlx"]
"#,
    )
    .expect("valid policy parses");

    assert_eq!(policy.architecture.core_dirs, ["src/core", "src/domain"]);
    assert_eq!(policy.architecture.infra_crates, ["tokio", "sqlx"]);
    assert!(policy.lints.is_empty());
    assert!(policy.thresholds.is_empty());
    assert!(policy.supply_chain.is_empty());
}

#[test]
fn valid_policy_with_sections_preserves_overrides() {
    let policy = parse_profile(
        r#"
[lints]
"clippy::needless_return" = "allow"

[thresholds]
too_many_lines = 40

[architecture]
core_dirs = ["crates/titania-core/src"]
infra_crates = ["tokio"]

[supply_chain]
multiple_versions = "deny"
"#,
    )
    .expect("valid policy parses");

    assert_eq!(policy.lints.get("clippy::needless_return").map(String::as_str), Some("allow"));
    assert!(policy.thresholds.contains_key("too_many_lines"));
    assert!(policy.supply_chain.contains_key("multiple_versions"));
}

#[test]
fn policy_missing_architecture_section_fails() {
    let err = parse_profile(
        r#"
[lints]
"clippy::needless_return" = "allow"
"#,
    )
    .expect_err("missing architecture must fail");

    assert_eq!(err, ProfileError::MissingField { field: "architecture" });
}

#[test]
fn policy_empty_core_dirs_fails() {
    let err = parse_profile(
        r#"
[architecture]
core_dirs = []
infra_crates = ["tokio"]
"#,
    )
    .expect_err("empty core_dirs must fail");

    assert_eq!(err, ProfileError::MissingField { field: "architecture.core_dirs" });
}

#[test]
fn policy_empty_infra_crates_fails() {
    let err = parse_profile(
        r#"
[architecture]
core_dirs = ["src/core"]
infra_crates = []
"#,
    )
    .expect_err("empty infra_crates must fail");

    assert_eq!(err, ProfileError::MissingField { field: "architecture.infra_crates" });
}

#[test]
fn malformed_policy_toml_returns_parse_error() {
    let err = parse_profile("[architecture\ncore_dirs = []").expect_err("malformed TOML must fail");

    let ProfileError::ParseError { message } = err else {
        panic!("expected parse error");
    };
    assert!(!message.is_empty());
}

#[test]
fn valid_single_exception_parses() {
    let exceptions = parse_exceptions(
        r#"
[[exceptions]]
rule_id = "FUNC_LOOPS_FOR"
path = "src/control/loop.rs"
owner = "flight-control"
reason = "Fixed 8-iteration control loop over sensor lanes"
expires_on = "2026-12-31"
review = "SAFETY-1234"
"#,
        "2026-07-04",
    )
    .expect("valid exception parses");

    assert_eq!(exceptions.len(), 1);
    assert_eq!(exceptions[0].rule_id, "FUNC_LOOPS_FOR");
    assert_eq!(exceptions[0].path, "src/control/loop.rs");
    assert_eq!(exceptions[0].owner, "flight-control");
    assert_eq!(exceptions[0].review, "SAFETY-1234");
}

#[test]
fn valid_multiple_exceptions_parse() {
    let exceptions = parse_exceptions(
        r#"
[[exceptions]]
rule_id = "FUNC_LOOPS_FOR"
path = "src/control/loop.rs"
owner = "flight-control"
reason = "Bounded sensor loop"
expires_on = "2026-12-31"
review = "SAFETY-1234"

[[exceptions]]
rule_id = "FORMAT_PRINT_001"
path = "src/report.rs"
owner = "cli"
reason = "CLI output boundary"
expires_on = "2027-01-01"
review = "CLI-9"
"#,
        "2026-07-04",
    )
    .expect("valid exceptions parse");

    assert_eq!(exceptions.len(), 2);
    assert_eq!(exceptions[1].rule_id, "FORMAT_PRINT_001");
}

#[test]
fn exception_missing_rule_id_fails() {
    assert_eq!(missing_exception_field("rule_id"), "rule_id");
}

#[test]
fn exception_missing_path_fails() {
    assert_eq!(missing_exception_field("path"), "path");
}

#[test]
fn exception_missing_owner_fails() {
    assert_eq!(missing_exception_field("owner"), "owner");
}

#[test]
fn exception_missing_reason_fails() {
    assert_eq!(missing_exception_field("reason"), "reason");
}

#[test]
fn exception_missing_expires_on_fails() {
    assert_eq!(missing_exception_field("expires_on"), "expires_on");
}

#[test]
fn exception_missing_review_fails() {
    assert_eq!(missing_exception_field("review"), "review");
}

#[test]
fn expired_exception_produces_policy_exception_expired() {
    let err = parse_exceptions(
        r#"
[[exceptions]]
rule_id = "FUNC_LOOPS_FOR"
path = "src/control/loop.rs"
owner = "flight-control"
reason = "Bounded sensor loop"
expires_on = "2020-01-01"
review = "SAFETY-1234"
"#,
        "2026-07-04",
    )
    .expect_err("expired exception must fail");

    assert_eq!(
        err,
        ExceptionError::ExceptionExpired {
            rule_id: Box::<str>::from("FUNC_LOOPS_FOR"),
            expires_on: Box::<str>::from("2020-01-01"),
            today: Box::<str>::from("2026-07-04"),
        }
    );
    assert_eq!(err.code(), "POLICY_EXCEPTION_EXPIRED");
}

#[test]
fn malformed_exceptions_toml_returns_parse_error() {
    let err = parse_exceptions("[[exceptions]\nrule_id = 1", "2026-07-04")
        .expect_err("malformed TOML must fail");

    let ExceptionError::ParseError { message } = err else {
        panic!("expected parse error");
    };
    assert!(!message.is_empty());
}

fn missing_exception_field(field: &'static str) -> &'static str {
    let mut lines = vec![
        r#"[[exceptions]]"#,
        r#"rule_id = "FUNC_LOOPS_FOR""#,
        r#"path = "src/control/loop.rs""#,
        r#"owner = "flight-control""#,
        r#"reason = "Bounded sensor loop""#,
        r#"expires_on = "2026-12-31""#,
        r#"review = "SAFETY-1234""#,
    ];
    lines.retain(|line| !line.starts_with(field));
    let content = lines.join("\n");
    let err = parse_exceptions(&content, "2026-07-04").expect_err("missing field must fail");

    let ExceptionError::MissingField { field } = err else {
        panic!("expected missing field");
    };
    field
}
