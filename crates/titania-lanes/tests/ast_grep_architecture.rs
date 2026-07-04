//! Contract tests for the embedded ast-grep architecture rule catalog.
//!
//! The rules are loaded at compile time via `include_str!` from
//! `crates/titania-lanes/rules/architecture.yml`.  Tests assert structural
//! properties of the YAML (rule IDs, language, severity, message, pattern,
//! effect, path exclusion) so that any drift is caught before the
//! `AstGrep` lane runs.
//!
//! Fixture files under `tests/fixtures/ast_grep/architecture/` exercise each
//! rule in isolation; the YAML patterns are expected to match them.

use std::collections::BTreeSet;

// ---------------------------------------------------------------------------
// Compile-time YAML load — fails if the catalog is missing or malformed.
// ---------------------------------------------------------------------------

/// Raw YAML text of the embedded ast-grep architecture rules.
const ARCHITECTURE_YAML: &str = include_str!("../rules/architecture.yml");

// ---------------------------------------------------------------------------
// Required rule IDs and their expected metadata.
// ---------------------------------------------------------------------------

/// The four architecture rule IDs that MUST appear in the catalog.
const REQUIRED_RULE_IDS: &[&str] = &[
    "ARCHITECTURE_IMPORT_CORE_INFRA",
    "ARCHITECTURE_IMPORT_CORE_FS",
    "ARCHITECTURE_IMPORT_CORE_TIME",
    "ARCHITECTURE_IMPORT_CORE_RANDOM",
];

/// Path segments that MUST be excluded from production scanning.
const PROD_EXCLUSIONS: &[&str] = &["tests", "benches", "examples", "build.rs"];

/// Fixture names that MUST exist for each rule: a violation and an allowed (clean) file.
const FIXTURE_NAMES: &[&[&str]] = &[
    &[
        "architecture_import_core_infra_violation.rs",
        "architecture_import_core_infra_direct_violation.rs",
        "allowed_no_core_infra_import.rs",
    ],
    &[
        "architecture_import_core_fs_violation.rs",
        "architecture_import_core_fs_direct_violation.rs",
        "allowed_no_core_fs_import.rs",
    ],
    &[
        "architecture_import_core_time_violation.rs",
        "architecture_import_core_time_direct_violation.rs",
        "allowed_no_core_time_import.rs",
    ],
    &[
        "architecture_import_core_random_violation.rs",
        "architecture_import_core_random_direct_violation.rs",
        "allowed_no_core_random_import.rs",
    ],
];

// ---------------------------------------------------------------------------
// Minimal YAML helpers — no external serde dependency.
// ---------------------------------------------------------------------------

/// Returns the value string for a YAML key at a given indentation level.
fn yaml_scalar_at<'a>(yaml: &'a str, key: &str, indent: usize) -> Option<&'a str> {
    let prefix = format!("{:indent$}{key}: ", "", indent = indent);
    for line in yaml.lines() {
        if line.starts_with(&prefix) {
            let val = line[prefix.len()..].trim();
            return Some(strip_yaml_quotes(val));
        }
    }
    None
}

/// Strip optional YAML single/double quotes from a scalar value.
fn strip_yaml_quotes(s: &str) -> &str {
    if let Some(inner) = s.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        return inner;
    }
    if let Some(inner) = s.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
        return inner;
    }
    s
}

/// Extract the text block belonging to a single rule (after `id: <id>`).
fn extract_rule_block<'a>(yaml: &'a str, rule_id: &str) -> Option<&'a str> {
    let marker = format!("\nid: {rule_id}");
    let start = yaml.find(&marker)?;
    let after_start = start + marker.len();
    let rest = &yaml[after_start..];

    // Find next rule boundary (a line starting with "id: " at indent 0).
    let end_offset = rest
        .lines()
        .enumerate()
        .find(|(_, line)| line.starts_with("id: "))
        .map(|(i, _)| {
            let mut offset = 0;
            for prev in rest.lines().take(i) {
                offset += prev.len() + 1;
            }
            offset
        })
        .unwrap_or(rest.len());

    Some(&yaml[after_start..after_start + end_offset])
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Every required architecture rule ID appears in the YAML exactly once, and no
/// extra rule IDs are present.
#[test]
fn ast_grep_architecture_all_required_rule_ids_present_exactly_once() {
    let mut found = BTreeSet::new();
    for &id in REQUIRED_RULE_IDS {
        let count = count_occurrences(ARCHITECTURE_YAML, &format!("id: {id}"));
        assert_eq!(count, 1, "Rule ID '{id}' must appear exactly once (found {count})",);
        assert!(found.insert(id.to_string()), "Duplicate rule ID '{id}'",);
    }

    // No extra rule IDs: every `id:` line must be one of the required set.
    let all_ids = collect_rule_ids(ARCHITECTURE_YAML);
    for id in &all_ids {
        assert!(REQUIRED_RULE_IDS.contains(&id.as_str()), "Unexpected rule ID '{id}' in catalog",);
    }
}

/// Every rule has language: Rust, a severity, a non-empty message,
/// and a pattern inside a rules[] array.
#[test]
fn ast_grep_architecture_every_rule_has_required_fields() {
    for &rule_id in REQUIRED_RULE_IDS {
        let block = extract_rule_block(ARCHITECTURE_YAML, rule_id)
            .unwrap_or_else(|| panic!("Rule block not found for '{rule_id}'"));

        // language: Rust
        assert_eq!(
            yaml_scalar_at(block, "language", 0),
            Some("Rust"),
            "'{rule_id}' must have language: Rust",
        );

        // severity (present, non-empty)
        assert!(
            yaml_scalar_at(block, "severity", 0).is_some(),
            "'{rule_id}' must have a severity field",
        );

        // message (present, non-empty)
        let msg =
            yaml_scalar_at(block, "message", 0).expect("'{rule_id}' must have a message field");
        assert!(!msg.is_empty(), "'{rule_id}' message must be non-empty",);

        // Rule body with at least one pattern.
        assert!(
            block.contains("rule:") || block.contains("rules:"),
            "'{rule_id}' must have a rule body",
        );
        assert!(
            block.contains("pattern:"),
            "'{rule_id}' must contain at least one rule with 'pattern:'",
        );
    }
}

/// Every architecture rule carries effect: Reject.
#[test]
fn ast_grep_architecture_all_rules_reject_effect() {
    for &rule_id in REQUIRED_RULE_IDS {
        let block = extract_rule_block(ARCHITECTURE_YAML, rule_id)
            .unwrap_or_else(|| panic!("Rule block not found for '{rule_id}'"));
        let lower = block.to_lowercase();
        assert!(lower.contains("reject"), "'{rule_id}' must have effect: Reject (found: {lower})",);
    }
}

/// Every architecture rule has a repair hint in metadata.
#[test]
fn ast_grep_architecture_all_rules_have_repair_hints() {
    for &rule_id in REQUIRED_RULE_IDS {
        let block = extract_rule_block(ARCHITECTURE_YAML, rule_id)
            .unwrap_or_else(|| panic!("Rule block not found for '{rule_id}'"));
        assert!(
            block.contains("repair_hint:"),
            "'{rule_id}' must contain a repair_hint in metadata",
        );
    }
}

/// Direct-import regexes catch non-wildcard imports such as `tokio::task`,
/// `std::fs::read_to_string`, `std::net::TcpStream`, and `rand::Rng`.
#[test]
fn ast_grep_architecture_direct_import_patterns_present() {
    assert!(
        ARCHITECTURE_YAML.contains("tokio|axum|sqlx|reqwest"),
        "Infra rule must catch direct imports, not only wildcard imports",
    );
    assert!(
        ARCHITECTURE_YAML.contains("std::(fs|env|net)"),
        "Filesystem rule must catch direct fs/env/net imports",
    );
    assert!(
        ARCHITECTURE_YAML.contains("std::time::(SystemTime|Instant)"),
        "Time rule must catch direct SystemTime and Instant imports",
    );
    assert!(
        ARCHITECTURE_YAML.contains("rand::(thread_rng|Rng)"),
        "Random rule must catch direct rand imports",
    );
}

/// Production-only path exclusions are present: tests, benches, examples,
/// and build.rs must be excluded from architecture scanning.
#[test]
fn ast_grep_architecture_production_only_path_exclusions_present() {
    for exclusion in PROD_EXCLUSIONS {
        assert!(
            ARCHITECTURE_YAML.contains("ignores:") || ARCHITECTURE_YAML.contains("exclude:"),
            "Architecture catalog must contain 'ignores:' or 'exclude:' keys for path filtering",
        );
        // The exclusion string itself must appear somewhere in the YAML.
        assert!(
            ARCHITECTURE_YAML.contains(exclusion),
            "Path exclusion '{exclusion}' must be present in architecture catalog",
        );
    }
}

/// Fixture files for each architecture rule exist under fixtures/.
#[test]
fn ast_grep_architecture_fixture_files_exist_for_each_rule() {
    let base = env!("CARGO_MANIFEST_DIR");
    let fixtures_dir = format!("{base}/tests/fixtures/ast_grep/architecture");

    for (rule_idx, rule_id) in REQUIRED_RULE_IDS.iter().enumerate() {
        for &fixture_name in FIXTURE_NAMES[rule_idx] {
            let fixture_path = format!("{fixtures_dir}/{fixture_name}");
            assert!(
                std::path::Path::new(&fixture_path).exists(),
                "Fixture file '{fixture_path}' must exist for rule '{rule_id}'",
            );

            let content = std::fs::read_to_string(&fixture_path)
                .unwrap_or_else(|e| panic!("Cannot read '{fixture_path}': {e}"));
            assert!(!content.is_empty(), "Fixture '{fixture_path}' must not be empty",);
        }
    }
}

/// The allowed fixtures contain no architecture-violating imports.
#[test]
fn ast_grep_architecture_allowed_fixtures_have_no_violations() {
    let base = env!("CARGO_MANIFEST_DIR");
    let fixtures_dir = format!("{base}/tests/fixtures/ast_grep/architecture");

    // Infra imports: tokio, axum, sqlx, reqwest wildcard imports
    let allowed_infra = read_fixture(&fixtures_dir, "allowed_no_core_infra_import.rs");
    assert!(
        !allowed_infra.contains("use tokio::")
            && !allowed_infra.contains("use axum::")
            && !allowed_infra.contains("use sqlx::")
            && !allowed_infra.contains("use reqwest::"),
        "allowed_no_core_infra_import.rs must not import core infra crates",
    );

    // FS/env/net wildcard imports
    let allowed_fs = read_fixture(&fixtures_dir, "allowed_no_core_fs_import.rs");
    assert!(
        !allowed_fs.contains("use std::fs::")
            && !allowed_fs.contains("use std::env::")
            && !allowed_fs.contains("use std::net::"),
        "allowed_no_core_fs_import.rs must not import std fs/env/net",
    );

    // Time imports
    let allowed_time = read_fixture(&fixtures_dir, "allowed_no_core_time_import.rs");
    assert!(
        !allowed_time.contains("use std::time::SystemTime")
            && !allowed_time.contains("use std::time::Instant"),
        "allowed_no_core_time_import.rs must not import time sources",
    );

    // Random/entropy imports
    let allowed_random = read_fixture(&fixtures_dir, "allowed_no_core_random_import.rs");
    assert!(
        !allowed_random.contains("use rand::thread_rng")
            && !allowed_random.contains("use rand::Rng"),
        "allowed_no_core_random_import.rs must not import rand entropy APIs",
    );
}

/// Violation fixtures actually contain the patterns they are named for.
#[test]
fn ast_grep_architecture_violation_fixtures_contain_their_patterns() {
    let base = env!("CARGO_MANIFEST_DIR");
    let fixtures_dir = format!("{base}/tests/fixtures/ast_grep/architecture");

    // Core-infra violation: must contain a tokio/axum/sqlx/reqwest import
    let infra = read_fixture(&fixtures_dir, "architecture_import_core_infra_violation.rs");
    assert!(
        infra.contains("use tokio::")
            || infra.contains("use axum::")
            || infra.contains("use sqlx::")
            || infra.contains("use reqwest::"),
        "architecture_import_core_infra_violation.rs must contain a core infra import",
    );
    let infra_direct =
        read_fixture(&fixtures_dir, "architecture_import_core_infra_direct_violation.rs");
    assert!(
        infra_direct.contains("use tokio::task"),
        "architecture_import_core_infra_direct_violation.rs must contain a direct infra import",
    );

    // Core-fs violation: must contain a std::fs/std::env/std::net import
    let fs = read_fixture(&fixtures_dir, "architecture_import_core_fs_violation.rs");
    assert!(
        fs.contains("use std::fs::")
            || fs.contains("use std::env::")
            || fs.contains("use std::net::"),
        "architecture_import_core_fs_violation.rs must contain a std fs/env/net import",
    );
    let fs_direct = read_fixture(&fixtures_dir, "architecture_import_core_fs_direct_violation.rs");
    assert!(
        fs_direct.contains("use std::fs::read_to_string")
            && fs_direct.contains("use std::env::var")
            && fs_direct.contains("use std::net::TcpStream"),
        "architecture_import_core_fs_direct_violation.rs must contain direct std fs/env/net imports",
    );

    // Core-time violation: must contain a SystemTime or Instant import
    let time = read_fixture(&fixtures_dir, "architecture_import_core_time_violation.rs");
    assert!(
        time.contains("SystemTime") || time.contains("Instant"),
        "architecture_import_core_time_violation.rs must import SystemTime or Instant",
    );
    let time_direct =
        read_fixture(&fixtures_dir, "architecture_import_core_time_direct_violation.rs");
    assert!(
        time_direct.contains("use std::time::Instant"),
        "architecture_import_core_time_direct_violation.rs must import Instant",
    );

    // Core-random violation: must contain rand::thread_rng import
    let random = read_fixture(&fixtures_dir, "architecture_import_core_random_violation.rs");
    assert!(
        random.contains("rand::thread_rng"),
        "architecture_import_core_random_violation.rs must import rand::thread_rng",
    );
    let random_direct =
        read_fixture(&fixtures_dir, "architecture_import_core_random_direct_violation.rs");
    assert!(
        random_direct.contains("use rand::Rng"),
        "architecture_import_core_random_direct_violation.rs must import rand::Rng",
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn count_occurrences(haystack: &str, needle: &str) -> usize {
    let mut count = 0;
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(needle) {
        count += 1;
        start += pos + 1;
    }
    count
}

fn collect_rule_ids(yaml: &str) -> Vec<String> {
    yaml.lines()
        .filter(|l| l.trim().starts_with("id: "))
        .map(|l| {
            let val = l.trim().strip_prefix("id: ").unwrap().trim();
            strip_yaml_quotes(val).to_string()
        })
        .collect()
}

fn read_fixture(dir: &str, name: &str) -> String {
    let path = format!("{dir}/{name}");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("Cannot read '{path}': {e}"))
}
