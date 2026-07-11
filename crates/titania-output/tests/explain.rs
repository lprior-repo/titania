//! Explain catalog contract tests for tn-ja8.1.

use std::collections::HashSet;

use titania_core::{RepairHint, RepairHintClass, catalog_rows};
use titania_output::{OutputError, explain::explain_rule};

const CATALOG: &str = include_str!("../../titania-core/src/finding/repair_catalog.tsv");
const REQUIRED: &[&str] = &[
    "FUNC_LOOPS_FOR",
    "FUNC_UNWRAP_USED",
    "FUNC_EXPECT_USED",
    "FUNC_UNWRAP_OR",
    "FUNC_NESTING_DEPTH",
    "FUNC_RECURSION_DIRECT",
    "BYPASS_PUB_ALLOW",
    "DYLINT_INFRA_FAILURE",
    "BYPASS_GENERATED_INCLUDE",
    "BYPASS_CARGO_CONFIG_PARENT",
    "BYPASS_CARGO_CONFIG_PARSE_ERROR",
    "BYPASS_CARGO_CONFIG_READ_ERROR",
    "BYPASS_ENV_CARGO_ENCODED_RUSTFLAGS",
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
fn explain_wildcard_import_collapse_to_human_review() {
    // After the tn-jy4y migration, the explain catalog's `repair` field
    // is derived from `titania_core::RepairHint::for_rule(id).class()`,
    // not the raw TSV literal. The TSV's `—` informational marker
    // collapses to `RequiresHumanReview` per the parser contract.
    let entry = explain_rule("FUNC_WILDCARD_IMPORT").expect("wildcard rule exists");
    assert_eq!(entry.effect, "Informational");
    assert_eq!(entry.repair, "RequiresHumanReview");
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
#[test]
fn explain_bypass_env_cargo_home_returns_policy_scan_row() {
    let entry = explain_rule("BYPASS_ENV_CARGO_HOME").expect("BYPASS_ENV_CARGO_HOME exists");
    assert_eq!(entry.rule_id, "BYPASS_ENV_CARGO_HOME");
    assert_eq!(entry.source, "policy-scan");
    assert_eq!(entry.effect, "Reject");
}

#[test]
fn explain_bypass_env_rustup_home_returns_policy_scan_row() {
    let entry = explain_rule("BYPASS_ENV_RUSTUP_HOME").expect("BYPASS_ENV_RUSTUP_HOME exists");
    assert_eq!(entry.rule_id, "BYPASS_ENV_RUSTUP_HOME");
    assert_eq!(entry.source, "policy-scan");
    assert_eq!(entry.effect, "Reject");
}

fn catalog_rule_ids() -> impl Iterator<Item = &'static str> {
    CATALOG
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| line.split('\t').next())
}

/// Catalog-vs-output parity test.
///
/// For every row in `repair_catalog.tsv`, `explain_rule(id).repair` must
/// agree with `RepairHint::for_rule(id).class().as_str()`. This is the
/// single-source-of-truth guarantee: the catalog lives in
/// `titania-core`, `titania-output::explain` consumes it via
/// `include_str!`, and `titania-lanes::Finding` populates its `repair`
/// field from `titania_core::RepairHint::for_rule`. If the three ever
/// drift apart, this test fails.
#[test]
fn catalog_class_matches_repair_hint_for_rule() {
    for row in catalog_rows() {
        let hint = RepairHint::for_rule(row.rule_id);
        let class_str = hint.class().as_str();
        // explain_rule returns RuleExplanation with `repair: &'static str`
        // (the catalog's class column literal). For `—` rows explain_rule
        // surfaces `RequiresHumanReview` per parser normalization.
        let entry = explain_rule(row.rule_id)
            .unwrap_or_else(|e| panic!("explain_rule({}) failed: {}", row.rule_id, e));
        assert_eq!(
            entry.repair, class_str,
            "row {}: explain_rule.repair = {:?}, RepairHint::for_rule().class() = {:?}",
            row.rule_id, entry.repair, class_str,
        );
        // Sanity: the class lookup itself returns one of the 7 valid classes.
        assert!(
            matches!(
                hint.class(),
                RepairHintClass::Patch
                    | RepairHintClass::UseIteratorPipeline
                    | RepairHintClass::FlattenNesting
                    | RepairHintClass::UseCheckedArithmetic
                    | RepairHintClass::RemoveAllowAttribute
                    | RepairHintClass::ReplaceDependency
                    | RepairHintClass::RequiresHumanReview,
            ),
            "row {} produced an invalid class variant",
            row.rule_id,
        );
    }
}

/// for_rule("") never panics and returns RequiresHumanReview.
#[test]
fn for_rule_empty_id_returns_human_review() {
    let hint = RepairHint::for_rule("");
    assert_eq!(hint.class(), RepairHintClass::RequiresHumanReview);
}

/// for_rule on a dynamic rule id never panics and returns RequiresHumanReview.
#[test]
fn for_rule_unknown_dynamic_id_returns_human_review() {
    let hint = RepairHint::for_rule("CLIPPY_SOMETHING_NOT_IN_CATALOG_XYZ");
    assert_eq!(hint.class(), RepairHintClass::RequiresHumanReview);
}
