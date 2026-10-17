use std::collections::BTreeSet;

const FUNCTIONAL_YAML: &str = include_str!("../rules/functional.yml");

const REQUIRED_RULE_IDS: &[&str] = &[
    "BOUNDED_EXECUTION_UNBOUNDED_LOOP",
    "BOUNDED_EXECUTION_UNBOUNDED_ITERATOR",
    "FUNC_NESTING_DEPTH",
    "FUNC_RECURSION_DIRECT",
    "FUNC_PRINT_STDOUT",
    "FUNC_PRINT_STDERR",
    "FUNC_WILDCARD_IMPORT",
    "FUNC_UNWRAP_OR",
    "FUNC_RESULT_STRING",
];

const PROD_EXCLUSIONS: &[&str] = &["tests", "benches", "examples", "build.rs"];

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

#[test]
fn ast_grep_functional_all_required_rule_ids_present_exactly_once() {
    let mut found = BTreeSet::new();
    for &id in REQUIRED_RULE_IDS {
        let count = count_occurrences(FUNCTIONAL_YAML, &format!("id: {id}"));
        assert_eq!(count, 1, "Rule ID '{id}' must appear exactly once (found {count})",);
        assert!(found.insert(id.to_string()), "Duplicate rule ID '{id}'",);
    }

    let all_ids = collect_rule_ids(FUNCTIONAL_YAML);
    for id in &all_ids {
        assert!(REQUIRED_RULE_IDS.contains(&id.as_str()), "Unexpected rule ID '{id}' in catalog",);
    }
}

#[test]
fn ast_grep_functional_every_rule_has_required_fields() {
    for &rule_id in REQUIRED_RULE_IDS {
        let block = extract_rule_block(FUNCTIONAL_YAML, rule_id)
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
fn ast_grep_functional_wildcard_import_is_informational() {
    let block = extract_rule_block(FUNCTIONAL_YAML, "FUNC_WILDCARD_IMPORT")
        .expect("FUNC_WILDCARD_IMPORT block must exist");

    let lower = block.to_lowercase();
    assert!(
        lower.contains("informational"),
        "FUNC_WILDCARD_IMPORT must have an 'Informational' effect or status",
    );
}

#[test]
fn ast_grep_functional_production_only_path_exclusions_present() {
    for exclusion in PROD_EXCLUSIONS {
        assert!(
            FUNCTIONAL_YAML.contains("exclude:") || FUNCTIONAL_YAML.contains("files:"),
            "Catalog must contain 'exclude:' or 'files:' keys for path filtering",
        );
        assert!(
            FUNCTIONAL_YAML.contains(exclusion),
            "Path exclusion '{exclusion}' must be present in catalog",
        );
    }
}

#[test]
fn ast_grep_functional_fixture_files_exist_for_each_rule() {
    let base = env!("CARGO_MANIFEST_DIR");
    let fixtures_dir = format!("{base}/tests/fixtures/ast_grep/functional");

    for &rule_id in REQUIRED_RULE_IDS {
        let fixture_name = violation_fixture_name(rule_id);
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

fn violation_fixture_name(rule_id: &str) -> String {
    match rule_id {
        "BOUNDED_EXECUTION_UNBOUNDED_LOOP" => {
            String::from("bounded_execution_unbounded_loop_violation.rs")
        }
        "BOUNDED_EXECUTION_UNBOUNDED_ITERATOR" => {
            String::from("bounded_execution_unbounded_iterator_violation.rs")
        }
        _ => format!("{}_violation.rs", rule_id.to_lowercase()),
    }
}

#[test]
fn ast_grep_functional_allowed_iterator_pipeline_fixture_has_no_loops() {
    let base = env!("CARGO_MANIFEST_DIR");
    let fixture = format!("{base}/tests/fixtures/ast_grep/functional/allowed_iterator_pipeline.rs");
    let content =
        std::fs::read_to_string(&fixture).unwrap_or_else(|e| panic!("Cannot read fixture: {e}"));

    assert!(!content.contains("loop {"), "Allowed fixture must not contain 'loop' blocks",);
    assert!(
        !content.contains("while "),
        "Allowed fixture must not contain condition 'while' loops",
    );
    assert!(
        !content.contains("repeat(")
            && !content.contains("repeat_with(")
            && !content.contains("successors(")
            && !content.contains("from_fn(")
            && !content.contains(".cycle()"),
        "Allowed fixture must not contain unbounded iterator sources",
    );
}

#[test]
fn ast_grep_functional_allowed_bounded_for_fixture_has_no_unbounded_surface() {
    let base = env!("CARGO_MANIFEST_DIR");
    let fixture = format!("{base}/tests/fixtures/ast_grep/functional/allowed_bounded_for.rs");
    let content =
        std::fs::read_to_string(&fixture).unwrap_or_else(|e| panic!("Cannot read fixture: {e}"));

    assert!(content.contains("for "), "Allowed bounded-for fixture must contain a 'for' loop",);
    assert!(content.contains('?'), "Allowed bounded-for fixture must propagate with '?'",);
    assert!(!content.contains("loop {"), "Bounded-for fixture must not contain 'loop' blocks",);
    assert!(
        !content.contains("repeat(") && !content.contains(".cycle()"),
        "Bounded-for fixture must not contain unbounded iterator sources",
    );
}

#[test]
fn ast_grep_functional_allowed_no_print_fixture_has_no_print_calls() {
    let base = env!("CARGO_MANIFEST_DIR");
    let fixture = format!("{base}/tests/fixtures/ast_grep/functional/allowed_no_print.rs");
    let content =
        std::fs::read_to_string(&fixture).unwrap_or_else(|e| panic!("Cannot read fixture: {e}"));

    assert!(!content.contains("println!"), "Allowed fixture must not contain println! calls",);
    assert!(!content.contains("print!"), "Allowed fixture must not contain print! calls",);
    assert!(!content.contains("eprintln!"), "Allowed fixture must not contain eprintln! calls",);
}

#[test]
fn ast_grep_functional_violation_fixtures_contain_their_patterns() {
    let base = env!("CARGO_MANIFEST_DIR");
    let fixtures_dir = format!("{base}/tests/fixtures/ast_grep/functional");

    assert_loop_fixture_contains(
        &fixtures_dir,
        "bounded_execution_unbounded_loop_violation.rs",
        "loop {",
    );
    assert_loop_fixture_contains(
        &fixtures_dir,
        "bounded_execution_unbounded_loop_violation.rs",
        "while ",
    );

    let unbounded_iter =
        read_fixture(&fixtures_dir, "bounded_execution_unbounded_iterator_violation.rs");
    assert!(
        unbounded_iter.contains("repeat(") || unbounded_iter.contains(".cycle()"),
        "'bounded_execution_unbounded_iterator_violation.rs' must contain repeat( or .cycle()",
    );

    let stdout = read_fixture(&fixtures_dir, "func_print_stdout_violation.rs");
    assert!(
        stdout.contains("println!") || stdout.contains("print!"),
        "func_print_stdout_violation.rs must contain println!/print!",
    );

    let stderr = read_fixture(&fixtures_dir, "func_print_stderr_violation.rs");
    assert!(stderr.contains("eprintln!"), "func_print_stderr_violation.rs must contain eprintln!",);

    let wildcard = read_fixture(&fixtures_dir, "func_wildcard_import_violation.rs");
    assert!(
        wildcard.contains("::*"),
        "func_wildcard_import_violation.rs must contain wildcard import",
    );
    let unwrap = read_fixture(&fixtures_dir, "func_unwrap_or_violation.rs");
    assert!(unwrap.contains(".unwrap_or"), "func_unwrap_or_violation.rs must contain .unwrap_or",);

    let result_string = read_fixture(&fixtures_dir, "func_result_string_violation.rs");
    assert!(
        result_string.contains("Result<") && result_string.contains(", String>"),
        "func_result_string_violation.rs must contain Result<T, String>",
    );
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

fn assert_loop_fixture_contains(dir: &str, name: &str, keyword: &str) {
    let content = read_fixture(dir, name);
    assert!(content.contains(keyword), "'{name}' must contain pattern '{keyword}'",);
}
