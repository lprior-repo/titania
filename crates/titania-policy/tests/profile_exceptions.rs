//! Policy and exception parser behavior tests.

use std::{fs, path::PathBuf};
use titania_policy::{ExceptionError, PolicyValue, ProfileError, parse_exceptions, parse_profile};

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

    let core_dirs =
        policy.architecture.core_dirs.iter().map(|path| path.as_str()).collect::<Vec<_>>();

    assert_eq!(core_dirs, ["src/core", "src/domain"]);
    assert_eq!(policy.architecture.infra_crates, ["tokio", "sqlx"]);
    assert!(policy.lints.is_empty());
    assert!(policy.thresholds.is_empty());
    assert!(policy.supply_chain.is_empty());
}

#[test]
fn checked_in_strict_ai_policy_parses_with_expected_architecture() {
    let content = fs::read_to_string(repo_root().join(".titania/profiles/strict-ai/policy.toml"))
        .expect("checked-in strict-ai policy must be readable");
    let policy = parse_profile(&content).expect("checked-in strict-ai policy must parse");
    let core_dirs =
        policy.architecture.core_dirs.iter().map(|path| path.as_str()).collect::<Vec<_>>();

    assert_eq!(core_dirs, ["src/core", "src/domain", "crates/*-core/src"]);
    assert_eq!(policy.architecture.infra_crates, ["tokio", "axum", "sqlx", "reqwest"]);
}

#[test]
fn checked_in_strict_ai_exceptions_parse_audit_exception() {
    let content =
        fs::read_to_string(repo_root().join(".titania/profiles/strict-ai/exceptions.toml"))
            .expect("checked-in strict-ai exceptions must be readable");
    let exceptions =
        parse_exceptions(&content, "2026-07-04").expect("checked-in exceptions must parse");

    assert_eq!(exceptions.len(), 1, "expected exactly one audited exception");
    let ex = &exceptions[0];
    assert_eq!(ex.rule_id.as_str(), "BYPASS_EXPECT_ATTR");
    assert_eq!(ex.path.as_str(), "crates/titania-dylint/src/lib.rs");
    assert_eq!(ex.owner.as_ref(), "titania-maintainers");
    assert!(
        ex.reason.contains("Dylint ABI"),
        "reason should cite Dylint ABI/no_mangle rationale, got: {}",
        ex.reason
    );
    assert_eq!(ex.expires_on.as_ref(), "2026-10-01");
    assert_eq!(ex.review.as_ref(), "tn-dylint-abi-expect");
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
    assert_eq!(policy.thresholds.get("too_many_lines"), Some(&PolicyValue::Integer(40)));
    assert_eq!(
        policy.supply_chain.get("multiple_versions"),
        Some(&PolicyValue::String("deny".into()))
    );
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
fn policy_unknown_root_key_returns_parse_error() {
    let err = parse_profile(
        r#"
unknown = true

[architecture]
core_dirs = ["src/core"]
infra_crates = ["tokio"]
"#,
    )
    .expect_err("unknown root key must fail");

    let ProfileError::ParseError { message } = err else {
        panic!("expected parse error");
    };
    assert!(message.contains("unknown field"));
}

#[test]
fn policy_invalid_lint_level_fails() {
    let err = parse_profile(
        r#"
[lints]
"clippy::needless_return" = "sometimes"

[architecture]
core_dirs = ["src/core"]
infra_crates = ["tokio"]
"#,
    )
    .expect_err("invalid lint level must fail");

    assert_invalid_profile_field(err, "lints");
}

#[test]
fn policy_whitespace_core_dir_fails() {
    let err = parse_profile(
        r#"
[architecture]
core_dirs = [" src/core"]
infra_crates = ["tokio"]
"#,
    )
    .expect_err("whitespace-padded core dir must fail");

    assert_invalid_profile_field(err, "architecture.core_dirs");
}

#[test]
fn policy_absolute_core_dir_fails() {
    let err = parse_profile(
        r#"
[architecture]
core_dirs = ["/src/core"]
infra_crates = ["tokio"]
"#,
    )
    .expect_err("absolute core dir must fail");

    assert_invalid_profile_field(err, "architecture.core_dirs");
}

#[test]
fn policy_whitespace_infra_crate_fails() {
    let err = parse_profile(
        r#"
[architecture]
core_dirs = ["src/core"]
infra_crates = [" tokio"]
"#,
    )
    .expect_err("whitespace-padded infra crate must fail");

    assert_invalid_profile_field(err, "architecture.infra_crates");
}

#[test]
fn policy_uppercase_infra_crate_fails() {
    let err = parse_profile(
        r#"
[architecture]
core_dirs = ["src/core"]
infra_crates = ["Tokio"]
"#,
    )
    .expect_err("uppercase infra crate must fail");

    assert_invalid_profile_field(err, "architecture.infra_crates");
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
    assert_eq!(exceptions[0].rule_id.as_str(), "FUNC_LOOPS_FOR");
    assert_eq!(exceptions[0].path.as_str(), "src/control/loop.rs");
    assert_eq!(&*exceptions[0].owner, "flight-control");
    assert_eq!(&*exceptions[0].review, "SAFETY-1234");
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
    assert_eq!(exceptions[1].rule_id.as_str(), "FORMAT_PRINT_001");
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
fn equal_expiry_date_is_still_valid() {
    let exceptions =
        parse_exceptions(valid_exception_with_expiry("2026-07-04").as_str(), "2026-07-04")
            .expect("same-day exception remains valid");

    assert_eq!(exceptions[0].expires_on.as_ref(), "2026-07-04");
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

#[test]
fn singular_exception_table_is_rejected() {
    let err = parse_exceptions(
        r#"
[Exception]
rule_id = "FUNC_LOOPS_FOR"
path = "src/control/loop.rs"
owner = "flight-control"
reason = "Bounded sensor loop"
expires_on = "2026-12-31"
review = "SAFETY-1234"
"#,
        "2026-07-04",
    )
    .expect_err("unknown singular table must fail");

    let ExceptionError::ParseError { message } = err else {
        panic!("expected parse error");
    };
    assert!(message.contains("unknown field"));
}

#[test]
fn exception_lowercase_rule_id_fails() {
    let err =
        parse_exceptions(valid_exception_with_rule_id("func_loops_for").as_str(), "2026-07-04")
            .expect_err("lowercase rule id must fail");

    assert_invalid_exception_field(err, "rule_id", "POLICY_EXCEPTION_INVALID_FIELD");
}

#[test]
fn exception_invalid_workspace_path_fails() {
    let err =
        parse_exceptions(valid_exception_with_path("src/../control.rs").as_str(), "2026-07-04")
            .expect_err("path traversal must fail");

    assert_invalid_exception_field(err, "path", "POLICY_EXCEPTION_INVALID_FIELD");
}

#[test]
fn exception_non_padded_date_fails() {
    let err = parse_exceptions(valid_exception_with_expiry("2026-7-04").as_str(), "2026-07-04")
        .expect_err("non-padded date must fail");

    assert_invalid_exception_field(err, "expires_on", "POLICY_EXCEPTION_INVALID_FIELD");
}

#[test]
fn exception_non_leap_feb_29_fails() {
    let err = parse_exceptions(valid_exception_with_expiry("2026-02-29").as_str(), "2026-01-01")
        .expect_err("non-leap Feb 29 must fail");

    assert_invalid_exception_field(err, "expires_on", "POLICY_EXCEPTION_INVALID_FIELD");
}

#[test]
fn exception_leap_feb_29_parses() {
    let exceptions =
        parse_exceptions(valid_exception_with_expiry("2028-02-29").as_str(), "2026-01-01")
            .expect("leap-year Feb 29 parses");

    assert_eq!(exceptions[0].expires_on.as_ref(), "2028-02-29");
}

#[test]
fn invalid_today_date_fails() {
    let err = parse_exceptions(valid_exception_with_expiry("2028-02-29").as_str(), "2026-2-01")
        .expect_err("invalid today date must fail");

    assert_invalid_exception_field(err, "today", "POLICY_EXCEPTION_INVALID_FIELD");
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("crate manifest has workspace root parent")
        .to_path_buf()
}

fn assert_invalid_profile_field(err: ProfileError, expected: &'static str) {
    let ProfileError::InvalidField { field, message } = err else {
        panic!("expected invalid field");
    };
    assert_eq!(field, expected);
    assert!(!message.is_empty());
}

fn assert_invalid_exception_field(
    err: ExceptionError,
    expected: &'static str,
    expected_code: &'static str,
) {
    assert_eq!(err.code(), expected_code);
    let ExceptionError::InvalidField { field, message } = err else {
        panic!("expected invalid field");
    };
    assert_eq!(field, expected);
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

fn valid_exception_with_expiry(expires_on: &str) -> String {
    format!(
        r#"
[[exceptions]]
rule_id = "FUNC_LOOPS_FOR"
path = "src/control/loop.rs"
owner = "flight-control"
reason = "Bounded sensor loop"
expires_on = "{expires_on}"
review = "SAFETY-1234"
"#
    )
}

fn valid_exception_with_rule_id(rule_id: &str) -> String {
    format!(
        r#"
[[exceptions]]
rule_id = "{rule_id}"
path = "src/control/loop.rs"
owner = "flight-control"
reason = "Bounded sensor loop"
expires_on = "2026-12-31"
review = "SAFETY-1234"
"#
    )
}

fn valid_exception_with_path(path: &str) -> String {
    format!(
        r#"
[[exceptions]]
rule_id = "FUNC_LOOPS_FOR"
path = "{path}"
owner = "flight-control"
reason = "Bounded sensor loop"
expires_on = "2026-12-31"
review = "SAFETY-1234"
"#
    )
}
