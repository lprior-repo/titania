//! Explain catalog contract tests for tn-ja8.1.

use std::collections::HashSet;

use titania_output::{OutputError, explain::explain_rule};

const CATALOG: &str = include_str!("../rules/explain.tsv");
const REQUIRED: &[&str] = &[
    "FUNC_LOOPS_FOR",
    "FUNC_UNWRAP_USED",
    "FUNC_EXPECT_USED",
    "FUNC_UNWRAP_OR",
    "FUNC_NESTING_DEPTH",
    "FUNC_RECURSION_DIRECT",
    "BYPASS_PUB_ALLOW",
    "DYLINT_INFRA_FAILURE",
    "BYPASS_CARGO_CONFIG_PARENT",
    "BYPASS_ENV_CARGO_ENCODED_RUSTFLAGS",
    "BYPASS_ENV_RUSTC_WORKSPACE_WRAPPER",
    "POLICY_EXCEPTION_READ_ERROR",
    "POLICY_EXCEPTION_EXPIRED",
    "POLICY_EXCEPTION_INVALID_FIELD",
    "POLICY_EXCEPTION_PARSE_ERROR",
    "POLICY_EXCEPTION_MISSING_FIELD",
    "HOLZMAN_PANIC_PANIC",
    "HOLZMAN_PANIC_ASSERT",
    "HOLZMAN_PANIC_ASSERT_EQ",
    "HOLZMAN_PANIC_ASSERT_NE",
    "HOLZMAN_PANIC_TODO",
    "HOLZMAN_PANIC_UNIMPLEMENTED",
    "HOLZMAN_PANIC_UNREACHABLE",
    "HOLZMAN_PANIC_DBG",
    "PANIC_SURFACE_001",
    "FORBIDDEN_001",
    "DENY_MULTIPLE_VERSIONS",
    "DENY_UNKNOWN_REGISTRY",
    "DENY_UNKNOWN_GIT",
    "DENY_UNKNOWN",
    "DENY_INFRA_FAILURE",
    "CARGO_FMT_001",
    "CARGO_COMPILE_001",
    "CARGO_CLIPPY_001",
    "CARGO_TEST_001",
    "CARGO_BUILD_001",
    "SRC_LINE_LIMIT",
    "FN_LINE_LIMIT",
    "SRC_LEN_LEDGER",
    "MUTANTS_RESIDUE",
    "COMPILE_SPLIT",
    "CLIPPY_UNKNOWN",
];

#[test]
fn explain_func_loops_for_returns_catalog_entry() {
    let entry = explain_rule("FUNC_LOOPS_FOR").expect("FUNC_LOOPS_FOR exists");
    assert_eq!(entry.rule_id, "FUNC_LOOPS_FOR");
    assert_eq!(entry.pattern, "for $LOOP in $ITER { ... }");
    assert_eq!(entry.repair, "UseIteratorPipeline");
    assert!(entry.example_violation.contains("for item in items"));
    assert!(entry.example_repair.contains("items.iter().for_each"));
}

#[test]
fn catalog_contains_required_finite_rule_ids() {
    REQUIRED.iter().for_each(|id| assert!(explain_rule(id).is_ok(), "missing {id}"));
}

#[test]
fn panic_policy_rules_are_dylint_owned() {
    let entry = explain_rule("HOLZMAN_PANIC_ASSERT").expect("panic rule exists");
    assert_eq!(entry.source, "dylint");
}

#[test]
fn dynamic_clippy_rule_covers_arbitrary_normalized_lint() {
    let entry = explain_rule("CLIPPY_NEEDLESS_BOOL").expect("dynamic clippy rule exists");
    assert_eq!(entry.source, "clippy");
    assert!(entry.pattern.contains("clippy::needless_bool"));
}

#[test]
fn required_ids_have_no_duplicates() {
    let mut seen = HashSet::new();
    REQUIRED.iter().for_each(|rule| assert!(seen.insert(rule), "duplicate {rule}"));
}

#[test]
fn catalog_has_no_duplicate_rule_ids() {
    let mut seen = HashSet::new();
    catalog_rule_ids().for_each(|rule| assert!(seen.insert(rule), "duplicate catalog row {rule}"));
}

#[test]
fn explain_unknown_rule_returns_unknown_rule() {
    assert!(matches!(explain_rule("DOES_NOT_EXIST"), Err(OutputError::UnknownRule { .. })));
}

fn catalog_rule_ids() -> impl Iterator<Item = &'static str> {
    CATALOG
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| line.split('\t').next())
}
