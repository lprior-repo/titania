use std::path::Path;

use titania_core::WorkspacePath;

use super::{RuleDef, RuleScope};

pub(super) fn rule_applies(rule: &RuleDef, workspace_path: &WorkspacePath) -> bool {
    let path = workspace_path.as_str();
    is_rust_source(path)
        && !is_ignored_production_path(path)
        && !is_meta_code_path(path)
        && !is_rule_ignored(rule.id, path)
        && scope_matches(rule.scope, path)
}

fn is_rust_source(path: &str) -> bool {
    Path::new(path).extension().is_some_and(|ext| ext == "rs")
}

fn is_ignored_production_path(path: &str) -> bool {
    path.starts_with("tests/")
        || path.contains("/tests/")
        || path.starts_with("benches/")
        || path.contains("/benches/")
        || path.starts_with("examples/")
        || path.contains("/examples/")
        || path.starts_with("fixtures/")
        || path.contains("/fixtures/")
        || path == "build.rs"
        || path.ends_with("/build.rs")
}

fn is_meta_code_path(path: &str) -> bool {
    path.split('/').collect::<Vec<_>>().windows(2).any(|pair| pair == ["ast_grep_lane", "rules"])
}

fn is_rule_ignored(rule_id: &str, path: &str) -> bool {
    if rule_id == "FUNC_WILDCARD_IMPORT" && path.starts_with("verification/verus/") {
        return true;
    }
    false
}

fn scope_matches(scope: RuleScope, path: &str) -> bool {
    match scope {
        RuleScope::ProductionRust => true,
        RuleScope::CoreDomainRust => is_core_domain_path(path),
    }
}

fn is_core_domain_path(path: &str) -> bool {
    path.starts_with("src/core/")
        || path.starts_with("src/domain/")
        || path.strip_prefix("crates/").is_some_and(crate_path_is_core_domain)
}

fn crate_path_is_core_domain(rest: &str) -> bool {
    rest.split_once('/').is_some_and(|(crate_name, crate_rest)| {
        (crate_name.ends_with("-core") || crate_name.ends_with("-domain"))
            && crate_rest.starts_with("src/")
    })
}

#[cfg(test)]
mod scope_tests {
    use super::{is_core_domain_path, is_meta_code_path};

    #[test]
    fn meta_code_path_matches_segment_sequence_anywhere() {
        assert!(is_meta_code_path("crates/titania-lanes/src/ast_grep_lane/rules/detectors.rs"));
        assert!(is_meta_code_path("ast_grep_lane/rules/detectors.rs"));
        assert!(is_meta_code_path("some/other/layout/ast_grep_lane/rules/filters.rs"));
        assert!(is_meta_code_path("/abs/checkout/ast_grep_lane/rules/architecture.yml.rs"));
    }

    #[test]
    fn meta_code_path_rejects_merely_similar_paths() {
        assert!(!is_meta_code_path("ast_grep_lane/rules_evil/detectors.rs"));
        assert!(!is_meta_code_path("crates/titania-lanes/src/ast_grep_lane/rules_evil/x.rs"));
        assert!(!is_meta_code_path("crates/titania-lanes/src/ast_grep_lane/rulesx/y.rs"));
        assert!(!is_meta_code_path("crates/titania-lanes/src/ast_grep_lane/detectors.rs"));
        assert!(!is_meta_code_path("crates/example-core/src/ast_grep_lane/mod.rs"));
    }

    #[test]
    fn tn_bvv7_designated_core_paths_are_in_scope() {
        assert!(is_core_domain_path("src/core/classify.rs"));
        assert!(is_core_domain_path("src/domain/receipt.rs"));
        assert!(is_core_domain_path("crates/titania-core/src/lib.rs"));
        assert!(is_core_domain_path("crates/titania-core/src/finding/repair_catalog.rs"));
        assert!(is_core_domain_path("crates/billing-domain/src/invoice.rs"));
    }

    #[test]
    fn tn_bvv7_shell_paths_are_out_of_scope() {
        assert!(!is_core_domain_path("crates/titania-lanes/src/ast_grep_lane.rs"));
        assert!(!is_core_domain_path("crates/titania-check/src/main.rs"));
        assert!(!is_core_domain_path("crates/titania-core/tests/effect_boundary.rs"));
        assert!(!is_core_domain_path("src/main.rs"));
        assert!(!is_core_domain_path("crates/titania-core/build.rs"));
    }
}
