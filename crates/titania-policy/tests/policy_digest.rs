//! Tests for canonical policy digest calculation.

use titania_policy::{PolicyDefaults, PolicyDigest};

fn embedded_defaults() -> PolicyDefaults {
    PolicyDefaults::embedded()
}

fn digest(
    defaults: &PolicyDefaults,
    policy: Option<&str>,
    exceptions: Option<&str>,
    deny: Option<&str>,
    clippy: Option<&str>,
) -> PolicyDigest {
    PolicyDigest::compute(defaults, policy, exceptions, deny, clippy)
}

#[test]
fn policy_digest_changes_when_binary_schema_version_changes() {
    let baseline_defaults = embedded_defaults();
    let mut changed_defaults = embedded_defaults();
    changed_defaults.schema_version = 2;

    let baseline = digest(&baseline_defaults, None, None, None, None);
    let changed = digest(&changed_defaults, None, None, None, None);

    assert_ne!(baseline.as_hex(), changed.as_hex());
}

#[test]
fn policy_digest_changes_when_binary_profile_name_changes() {
    let baseline_defaults = embedded_defaults();
    let mut changed_defaults = embedded_defaults();
    changed_defaults.profile_name = String::from("strict-rust");

    let baseline = digest(&baseline_defaults, None, None, None, None);
    let changed = digest(&changed_defaults, None, None, None, None);

    assert_ne!(baseline.as_hex(), changed.as_hex());
}

#[test]
fn policy_digest_changes_when_policy_toml_changes() {
    let defaults = embedded_defaults();
    let first = r#"[thresholds]
too_many_lines = 40
"#;
    let second = r#"[thresholds]
too_many_lines = 60
"#;

    let first_digest = digest(&defaults, Some(first), None, None, None);
    let second_digest = digest(&defaults, Some(second), None, None, None);

    assert_ne!(first_digest.as_hex(), second_digest.as_hex());
}

#[test]
fn policy_digest_changes_when_exceptions_toml_changes() {
    let defaults = embedded_defaults();
    let first = r#"[[exceptions]]
rule_id = "FUNC_LOOPS_FOR"
path = "src/control/loop.rs"
"#;
    let second = r#"[[exceptions]]
rule_id = "FUNC_LOOPS_WHILE"
path = "src/control/loop.rs"
"#;

    let first_digest = digest(&defaults, None, Some(first), None, None);
    let second_digest = digest(&defaults, None, Some(second), None, None);

    assert_ne!(first_digest.as_hex(), second_digest.as_hex());
}

#[test]
fn policy_digest_changes_when_deny_toml_changes() {
    let defaults = embedded_defaults();
    let first = r#"[bans]
wildcards = "deny"
"#;
    let second = r#"[bans]
wildcards = "allow"
"#;

    let first_digest = digest(&defaults, None, None, Some(first), None);
    let second_digest = digest(&defaults, None, None, Some(second), None);

    assert_ne!(first_digest.as_hex(), second_digest.as_hex());
}

#[test]
fn policy_digest_changes_when_clippy_toml_changes() {
    let defaults = embedded_defaults();
    let first = "cognitive-complexity-threshold = 8\n";
    let second = "cognitive-complexity-threshold = 16\n";

    let first_digest = digest(&defaults, None, None, None, Some(first));
    let second_digest = digest(&defaults, None, None, None, Some(second));

    assert_ne!(first_digest.as_hex(), second_digest.as_hex());
}

#[test]
fn policy_digest_ignores_toml_comments() {
    let defaults = embedded_defaults();
    let with_comments = r#"# leading comment
[thresholds]
# section comment
too_many_lines = 40 # inline comment
"#;
    let without_comments = r#"[thresholds]
too_many_lines = 40
"#;

    let with_digest = digest(&defaults, Some(with_comments), None, None, None);
    let without_digest = digest(&defaults, Some(without_comments), None, None, None);

    assert_eq!(with_digest.as_hex(), without_digest.as_hex());
}

#[test]
fn policy_digest_ignores_policy_toml_key_order() {
    let defaults = embedded_defaults();
    let first = r#"[thresholds]
too_many_lines = 40
too_many_arguments = 5
"#;
    let second = r#"[thresholds]
too_many_arguments = 5
too_many_lines = 40
"#;

    let first_digest = digest(&defaults, Some(first), None, None, None);
    let second_digest = digest(&defaults, Some(second), None, None, None);

    assert_eq!(first_digest.as_hex(), second_digest.as_hex());
}

#[test]
fn policy_digest_ignores_exceptions_toml_key_order() {
    let defaults = embedded_defaults();
    let first = r#"[[exceptions]]
rule_id = "FUNC_LOOPS_FOR"
path = "src/control/loop.rs"
expires_on = "2026-12-31"
"#;
    let second = r#"[[exceptions]]
expires_on = "2026-12-31"
path = "src/control/loop.rs"
rule_id = "FUNC_LOOPS_FOR"
"#;

    let first_digest = digest(&defaults, None, Some(first), None, None);
    let second_digest = digest(&defaults, None, Some(second), None, None);

    assert_eq!(first_digest.as_hex(), second_digest.as_hex());
}

#[test]
fn policy_digest_is_deterministic_lowercase_blake3_hex() {
    let defaults = embedded_defaults();
    let first = digest(&defaults, None, None, None, None);
    let second = digest(&defaults, None, None, None, None);
    let hex = first.as_hex();

    assert_eq!(first.as_hex(), second.as_hex());
    assert_eq!(hex.len(), 64);
    assert!(hex.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit()));
}
