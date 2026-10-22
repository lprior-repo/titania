use std::collections::{BTreeSet, HashMap};

const ARCHITECTURE_YAML: &str = include_str!("../rules/architecture.yml");

const REQUIRED_RULE_IDS: &[&str] = &[
    "ARCHITECTURE_IMPORT_CORE_INFRA",
    "ARCHITECTURE_IMPORT_CORE_FS",
    "ARCHITECTURE_IMPORT_CORE_TIME",
    "ARCHITECTURE_IMPORT_CORE_RANDOM",
    "ARCHITECTURE_IMPORT_CORE_PROCESS",
    "ARCHITECTURE_EFFECT_CORE_ASYNC",
    "ARCHITECTURE_EFFECT_CORE_STATE",
];

const PROD_EXCLUSIONS: &[&str] = &["tests", "benches", "examples", "build.rs"];

const FIXTURE_NAMES: &[&[&str]] = &[
    &[
        "architecture_import_core_infra_violation.rs",
        "architecture_import_core_infra_direct_violation.rs",
        "architecture_import_core_infra_fq_violation.rs",
        "allowed_no_core_infra_import.rs",
    ],
    &[
        "architecture_import_core_fs_violation.rs",
        "architecture_import_core_fs_direct_violation.rs",
        "architecture_import_core_fs_grouped_violation.rs",
        "architecture_import_core_fs_module_violation.rs",
        "architecture_import_core_fs_fq_violation.rs",
        "architecture_import_core_fs_alias_violation.rs",
        "architecture_import_core_fs_macro_violation.rs",
        "allowed_no_core_fs_import.rs",
    ],
    &[
        "architecture_import_core_time_violation.rs",
        "architecture_import_core_time_direct_violation.rs",
        "architecture_import_core_time_grouped_violation.rs",
        "architecture_import_core_time_fq_violation.rs",
        "allowed_no_core_time_import.rs",
    ],
    &[
        "architecture_import_core_random_violation.rs",
        "architecture_import_core_random_direct_violation.rs",
        "architecture_import_core_random_grouped_violation.rs",
        "architecture_import_core_random_fq_violation.rs",
        "allowed_no_core_random_import.rs",
    ],
    &[
        "architecture_import_core_process_violation.rs",
        "architecture_import_core_process_fq_violation.rs",
        "allowed_no_core_process_import.rs",
    ],
    &["architecture_effect_core_async_violation.rs", "allowed_no_core_async_effect.rs"],
    &["architecture_effect_core_state_violation.rs", "allowed_no_core_state_effect.rs"],
];

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

fn strip_yaml_quotes(s: &str) -> &str {
    if let Some(inner) = s.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        return inner;
    }
    if let Some(inner) = s.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
        return inner;
    }
    s
}

fn extract_rule_block<'a>(yaml: &'a str, rule_id: &str) -> Option<&'a str> {
    let marker = format!("\nid: {rule_id}");
    let start = yaml.find(&marker)?;
    let after_start = start + marker.len();
    let rest = &yaml[after_start..];

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

fn yaml_metadata_value(block: &str, key: &str) -> Option<String> {
    let lines: Vec<&str> = block.lines().collect();

    let meta_idx = lines.iter().position(|l| l.trim_start() == "metadata:")?;

    let mut meta_lines = String::new();
    for line in &lines[meta_idx + 1..] {
        let trimmed = line.trim_start();
        let is_top_level_key = trimmed.starts_with("id: ")
            || trimmed.starts_with("language: ")
            || trimmed.starts_with("severity: ")
            || trimmed.starts_with("message: ")
            || trimmed.starts_with("files:")
            || trimmed.starts_with("ignores:")
            || trimmed.starts_with("rule:")
            || trimmed.starts_with("metadata:");

        if is_top_level_key {
            break;
        }
        meta_lines.push_str(line);
        meta_lines.push('\n');
    }

    for line in meta_lines.lines() {
        if line.contains(key) {
            let value = line.splitn(2, ':').nth(1)?;
            return Some(strip_yaml_quotes(value.trim()).to_owned());
        }
    }
    None
}

#[test]
fn ast_grep_architecture_all_required_rule_ids_present_exactly_once() {
    let mut found = BTreeSet::new();
    for &id in REQUIRED_RULE_IDS {
        let count = count_occurrences(ARCHITECTURE_YAML, &format!("id: {id}"));
        assert_eq!(count, 1, "Rule ID '{id}' must appear exactly once (found {count})",);
        assert!(found.insert(id.to_string()), "Duplicate rule ID '{id}'",);
    }

    let all_ids = collect_rule_ids(ARCHITECTURE_YAML);
    for id in &all_ids {
        assert!(REQUIRED_RULE_IDS.contains(&id.as_str()), "Unexpected rule ID '{id}' in catalog",);
    }
}

#[test]
fn ast_grep_architecture_every_rule_has_required_fields() {
    for &rule_id in REQUIRED_RULE_IDS {
        let block = extract_rule_block(ARCHITECTURE_YAML, rule_id)
            .unwrap_or_else(|| panic!("Rule block not found for '{rule_id}'"));

        assert_eq!(
            yaml_scalar_at(block, "language", 0),
            Some("Rust"),
            "'{rule_id}' must have language: Rust",
        );

        assert!(
            yaml_scalar_at(block, "severity", 0).is_some(),
            "'{rule_id}' must have a severity field",
        );

        let msg =
            yaml_scalar_at(block, "message", 0).expect("'{rule_id}' must have a message field");
        assert!(!msg.is_empty(), "'{rule_id}' message must be non-empty",);

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

#[test]
fn ast_grep_architecture_all_rules_reject_effect() {
    for &rule_id in REQUIRED_RULE_IDS {
        let block = extract_rule_block(ARCHITECTURE_YAML, rule_id)
            .unwrap_or_else(|| panic!("Rule block not found for '{rule_id}'"));
        let effect = yaml_metadata_value(block, "effect")
            .unwrap_or_else(|| panic!("'{rule_id}' must have effect in metadata"));
        assert_eq!(
            effect, "Reject",
            "'{rule_id}' metadata.effect must be exactly 'Reject' (got: '{effect}')"
        );
    }
}

#[test]
fn ast_grep_architecture_repair_hints_per_rule() {
    let expected: HashMap<&str, &str> = [
        ("ARCHITECTURE_IMPORT_CORE_INFRA", "ReplaceDependency"),
        ("ARCHITECTURE_IMPORT_CORE_FS", "RequiresHumanReview"),
        ("ARCHITECTURE_IMPORT_CORE_TIME", "RequiresHumanReview"),
        ("ARCHITECTURE_IMPORT_CORE_RANDOM", "RequiresHumanReview"),
        ("ARCHITECTURE_IMPORT_CORE_PROCESS", "RequiresHumanReview"),
        ("ARCHITECTURE_EFFECT_CORE_ASYNC", "ReplaceDependency"),
        ("ARCHITECTURE_EFFECT_CORE_STATE", "RequiresHumanReview"),
    ]
    .into_iter()
    .collect();
    for &rule_id in REQUIRED_RULE_IDS {
        let block = extract_rule_block(ARCHITECTURE_YAML, rule_id)
            .unwrap_or_else(|| panic!("Rule block not found for '{rule_id}'"));
        let hint = yaml_metadata_value(block, "repair_hint")
            .unwrap_or_else(|| panic!("'{rule_id}' must have repair_hint in metadata"));
        let exp = expected
            .get(rule_id)
            .unwrap_or_else(|| panic!("No expected repair_hint for '{rule_id}'"));
        assert_eq!(hint, *exp, "'{rule_id}' metadata.repair_hint must be '{exp}' (got: '{hint}')");
    }
}

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
        ARCHITECTURE_YAML.contains(r"std::(fs|env|net)\\b"),
        "Filesystem rule must catch module imports such as use std::fs",
    );
    assert!(
        ARCHITECTURE_YAML.contains(r"std::\\{"),
        "Filesystem rule must catch grouped std imports",
    );
    assert!(
        ARCHITECTURE_YAML.contains("std::time::(SystemTime|Instant)"),
        "Time rule must catch direct SystemTime and Instant imports",
    );
    assert!(
        ARCHITECTURE_YAML.contains(r"std::time::\\{"),
        "Time rule must catch grouped SystemTime and Instant imports",
    );
    assert!(
        ARCHITECTURE_YAML.contains("rand::thread_rng"),
        "Random rule must catch direct rand imports",
    );
    assert!(
        ARCHITECTURE_YAML.contains(r"rand::\\{"),
        "Random rule must catch grouped rand imports",
    );
}

#[test]
fn ast_grep_architecture_fully_qualified_patterns_present() {
    assert!(
        ARCHITECTURE_YAML.contains(r"\\b(tokio|axum|sqlx|reqwest)::"),
        "Infra rule must catch fully-qualified infra paths with no import",
    );
    assert!(
        ARCHITECTURE_YAML.contains(r"\\bstd::(fs|env|net)::"),
        "Filesystem rule must catch fully-qualified std paths with no import",
    );
    assert!(
        ARCHITECTURE_YAML.contains(r"\\bstd::time::(SystemTime|Instant)\\b"),
        "Time rule must catch fully-qualified clock reads with no import",
    );
    assert!(
        ARCHITECTURE_YAML.contains(r"\\brand::thread_rng\\b"),
        "Random rule must catch fully-qualified entropy use with no import",
    );
    assert!(
        ARCHITECTURE_YAML.contains(r"\\bstd::process::"),
        "Process rule must catch fully-qualified process spawns",
    );
    assert!(ARCHITECTURE_YAML.contains(r"\\.await\\b"), "Async rule must catch await expressions",);
    assert!(
        ARCHITECTURE_YAML.contains(r"\\bstatic\\s+mut\\b"),
        "State rule must catch static mut declarations",
    );
}

#[test]
fn ast_grep_architecture_production_only_path_exclusions_present() {
    for exclusion in PROD_EXCLUSIONS {
        assert!(
            ARCHITECTURE_YAML.contains("ignores:") || ARCHITECTURE_YAML.contains("exclude:"),
            "Architecture catalog must contain 'ignores:' or 'exclude:' keys for path filtering",
        );
        assert!(
            ARCHITECTURE_YAML.contains(exclusion),
            "Path exclusion '{exclusion}' must be present in architecture catalog",
        );
    }
}

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

#[test]
fn ast_grep_architecture_allowed_fixtures_have_no_violations() {
    let base = env!("CARGO_MANIFEST_DIR");
    let fixtures_dir = format!("{base}/tests/fixtures/ast_grep/architecture");

    let allowed_infra = read_fixture(&fixtures_dir, "allowed_no_core_infra_import.rs");
    assert!(
        !allowed_infra.contains("use tokio::")
            && !allowed_infra.contains("use axum::")
            && !allowed_infra.contains("use sqlx::")
            && !allowed_infra.contains("use reqwest::"),
        "allowed_no_core_infra_import.rs must not import core infra crates",
    );

    let allowed_fs = read_fixture(&fixtures_dir, "allowed_no_core_fs_import.rs");
    assert!(
        !allowed_fs.contains("use std::fs::")
            && !allowed_fs.contains("use std::env::")
            && !allowed_fs.contains("use std::net::"),
        "allowed_no_core_fs_import.rs must not import std fs/env/net",
    );

    let allowed_time = read_fixture(&fixtures_dir, "allowed_no_core_time_import.rs");
    assert!(
        !allowed_time.contains("use std::time::SystemTime")
            && !allowed_time.contains("use std::time::Instant"),
        "allowed_no_core_time_import.rs must not import time sources",
    );

    let allowed_random = read_fixture(&fixtures_dir, "allowed_no_core_random_import.rs");
    assert!(
        !allowed_random.contains("use rand::thread_rng")
            && !allowed_random.contains("use rand::Rng"),
        "allowed_no_core_random_import.rs must not import rand entropy APIs",
    );

    let allowed_process = read_fixture(&fixtures_dir, "allowed_no_core_process_import.rs");
    assert!(
        !allowed_process.contains("std::process") && !allowed_process.contains("tokio::process"),
        "allowed_no_core_process_import.rs must not import process APIs",
    );

    let allowed_async = read_fixture(&fixtures_dir, "allowed_no_core_async_effect.rs");
    assert!(
        !allowed_async.contains("async fn")
            && !allowed_async.contains(".await")
            && !allowed_async.contains("tokio::"),
        "allowed_no_core_async_effect.rs must stay synchronous",
    );

    let allowed_state = read_fixture(&fixtures_dir, "allowed_no_core_state_effect.rs");
    assert!(
        !allowed_state.contains("static mut")
            && !allowed_state.contains("std::sync::Mutex")
            && !allowed_state.contains("std::sync::RwLock")
            && !allowed_state.contains("std::cell::RefCell"),
        "allowed_no_core_state_effect.rs must not use ambient mutable state",
    );
}

#[test]
fn ast_grep_architecture_violation_fixtures_contain_their_patterns() {
    let base = env!("CARGO_MANIFEST_DIR");
    let fixtures_dir = format!("{base}/tests/fixtures/ast_grep/architecture");

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
    let fs_grouped =
        read_fixture(&fixtures_dir, "architecture_import_core_fs_grouped_violation.rs");
    assert!(
        fs_grouped.contains("use std::{env, fs, net}"),
        "architecture_import_core_fs_grouped_violation.rs must contain grouped std imports",
    );
    let fs_module = read_fixture(&fixtures_dir, "architecture_import_core_fs_module_violation.rs");
    assert!(
        fs_module.contains("use std::fs;"),
        "architecture_import_core_fs_module_violation.rs must contain module std::fs import",
    );

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
    let time_grouped =
        read_fixture(&fixtures_dir, "architecture_import_core_time_grouped_violation.rs");
    assert!(
        time_grouped.contains("use std::time::{Duration, Instant, SystemTime}"),
        "architecture_import_core_time_grouped_violation.rs must contain grouped time imports",
    );

    let random = read_fixture(&fixtures_dir, "architecture_import_core_random_violation.rs");
    assert!(
        random.contains("rand::thread_rng"),
        "architecture_import_core_random_violation.rs must import rand::thread_rng",
    );
    let random_direct =
        read_fixture(&fixtures_dir, "architecture_import_core_random_direct_violation.rs");
    assert!(
        random_direct.contains("use rand::thread_rng"),
        "architecture_import_core_random_direct_violation.rs must import rand::thread_rng",
    );
    let random_grouped =
        read_fixture(&fixtures_dir, "architecture_import_core_random_grouped_violation.rs");
    assert!(
        random_grouped.contains("use rand::{Rng, thread_rng}"),
        "architecture_import_core_random_grouped_violation.rs must contain grouped rand imports",
    );

    let infra_fq = read_fixture(&fixtures_dir, "architecture_import_core_infra_fq_violation.rs");
    assert!(
        infra_fq.contains("tokio::spawn"),
        "architecture_import_core_infra_fq_violation.rs must use tokio with no import",
    );
    let fs_fq = read_fixture(&fixtures_dir, "architecture_import_core_fs_fq_violation.rs");
    assert!(
        fs_fq.contains("std::fs::read_to_string")
            && fs_fq.contains("std::env::var")
            && fs_fq.contains("std::net::TcpStream"),
        "architecture_import_core_fs_fq_violation.rs must use std fs/env/net with no import",
    );
    let fs_alias = read_fixture(&fixtures_dir, "architecture_import_core_fs_alias_violation.rs");
    assert!(
        fs_alias.contains("use std::fs as filesystem"),
        "architecture_import_core_fs_alias_violation.rs must alias the fs import",
    );
    let fs_macro = read_fixture(&fixtures_dir, "architecture_import_core_fs_macro_violation.rs");
    assert!(
        fs_macro.contains("macro_rules!") && fs_macro.contains("std::fs::read_to_string"),
        "architecture_import_core_fs_macro_violation.rs must hide fs I/O in a macro body",
    );
    let time_fq = read_fixture(&fixtures_dir, "architecture_import_core_time_fq_violation.rs");
    assert!(
        time_fq.contains("std::time::SystemTime::now")
            && time_fq.contains("std::time::Instant::now"),
        "architecture_import_core_time_fq_violation.rs must read the clock with no import",
    );
    let random_fq = read_fixture(&fixtures_dir, "architecture_import_core_random_fq_violation.rs");
    assert!(
        random_fq.contains("rand::thread_rng"),
        "architecture_import_core_random_fq_violation.rs must draw entropy with no import",
    );

    let process = read_fixture(&fixtures_dir, "architecture_import_core_process_violation.rs");
    assert!(
        process.contains("use std::process::Command"),
        "architecture_import_core_process_violation.rs must import std::process",
    );
    let process_fq =
        read_fixture(&fixtures_dir, "architecture_import_core_process_fq_violation.rs");
    assert!(
        process_fq.contains("std::process::Command::new"),
        "architecture_import_core_process_fq_violation.rs must spawn with no import",
    );

    let async_fx = read_fixture(&fixtures_dir, "architecture_effect_core_async_violation.rs");
    assert!(
        async_fx.contains("async fn")
            && async_fx.contains(".await")
            && async_fx.contains("tokio::spawn"),
        "architecture_effect_core_async_violation.rs must contain async effects",
    );

    let state_fx = read_fixture(&fixtures_dir, "architecture_effect_core_state_violation.rs");
    assert!(
        state_fx.contains("static mut") && state_fx.contains("std::sync::Mutex"),
        "architecture_effect_core_state_violation.rs must contain ambient mutable state",
    );
}

#[test]
fn ast_grep_architecture_core_manifest_has_no_effect_dependencies() {
    let base = env!("CARGO_MANIFEST_DIR");
    let manifest_path = format!("{base}/../titania-core/Cargo.toml");
    let manifest = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("Cannot read '{manifest_path}': {e}"));

    for banned in ["tokio", "axum", "sqlx", "reqwest", "rand", "futures", "async-std", "smol"] {
        let direct = format!("{banned} =");
        let quoted = format!("\"{banned}\"");
        assert!(
            !manifest.lines().any(|line| {
                let trimmed = line.trim();
                !trimmed.starts_with('#')
                    && (trimmed.starts_with(&direct) || trimmed.contains(&quoted))
            }),
            "titania-core/Cargo.toml must not depend on '{banned}'",
        );
    }
}

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
