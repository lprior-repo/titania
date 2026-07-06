//! Path-scope filters mirroring embedded YAML `files` and `ignores` selectors.

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
    path.starts_with("crates/titania-lanes/src/ast_grep_lane/rules/")
}

fn is_rule_ignored(rule_id: &str, path: &str) -> bool {
    // Exclude Verus proof files from FUNC_WILDCARD_IMPORT (prelude is Verus std)
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
