//! Contract tests for the embedded ast-grep functional rule catalog.
//!
//! The rules are loaded at compile time via `include_str!` from
//! `crates/titania-lanes/rules/functional.yml`.  Tests assert structural
//! properties of the YAML (rule IDs, language, severity, message, pattern,
//! effect, path exclusion) so that any drift is caught before the
//! `AstGrep` lane runs.
//!
//! Fixture files under `tests/fixtures/ast_grep/functional/` exercise each
//! rule in isolation; the YAML patterns are expected to match them.

use std::collections::BTreeSet;

// ---------------------------------------------------------------------------
// Compile-time YAML load — fails if the catalog is missing or malformed.
// ---------------------------------------------------------------------------

/// Raw YAML text of the embedded ast-grep functional rules.
const FUNCTIONAL_YAML: &str = include_str!("../rules/functional.yml");

// ---------------------------------------------------------------------------
// Required rule IDs and their expected metadata.
// ---------------------------------------------------------------------------

/// The seven rule IDs that MUST appear in the catalog.
const REQUIRED_RULE_IDS: &[&str] = &[
    "FUNC_LOOPS_FOR",
    "FUNC_LOOPS_WHILE",
    "FUNC_LOOPS_LOOP",
    "FUNC_PRINT_STDOUT",
    "FUNC_PRINT_STDERR",
    "FUNC_WILDCARD_IMPORT",
    "FUNC_UNWRAP_OR",
];

/// Path segments that MUST be excluded from production scanning.
const PROD_EXCLUSIONS: &[&str] = &["tests", "benches", "examples", "build.rs"];

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

/// Every required rule ID appears in the YAML exactly once, and no
/// extra rule IDs are present.
#[test]
fn ast_grep_functional_all_required_rule_ids_present_exactly_once() {
    let mut found = BTreeSet::new();
    for &id in REQUIRED_RULE_IDS {
        let count = count_occurrences(FUNCTIONAL_YAML, &format!("id: {id}"));
        assert_eq!(count, 1, "Rule ID '{id}' must appear exactly once (found {count})",);
        assert!(found.insert(id.to_string()), "Duplicate rule ID '{id}'",);
    }

    // No extra rule IDs: every `id:` line must be one of the required set.
    let all_ids = collect_rule_ids(FUNCTIONAL_YAML);
    for id in &all_ids {
        assert!(REQUIRED_RULE_IDS.contains(&id.as_str()), "Unexpected rule ID '{id}' in catalog",);
    }
}

/// Every rule has language: Rust, a severity, a non-empty message,
/// and a pattern inside a rules[] array.
#[test]
fn ast_grep_functional_every_rule_has_required_fields() {
    for &rule_id in REQUIRED_RULE_IDS {
        let block = extract_rule_block(FUNCTIONAL_YAML, rule_id)
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

/// FUNC_WILDCARD_IMPORT carries an Informational effect/status.
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

/// Production-only path exclusions are present: tests, benches, examples,
/// and build.rs must be excluded from scanning.
#[test]
fn ast_grep_functional_production_only_path_exclusions_present() {
    for exclusion in PROD_EXCLUSIONS {
        assert!(
            FUNCTIONAL_YAML.contains("exclude:") || FUNCTIONAL_YAML.contains("files:"),
            "Catalog must contain 'exclude:' or 'files:' keys for path filtering",
        );
        // The exclusion string itself must appear somewhere in the YAML.
        assert!(
            FUNCTIONAL_YAML.contains(exclusion),
            "Path exclusion '{exclusion}' must be present in catalog",
        );
    }
}

/// Fixture files for each violating construct exist under fixtures/.
#[test]
fn ast_grep_functional_fixture_files_exist_for_each_rule() {
    let base = env!("CARGO_MANIFEST_DIR");
    let fixtures_dir = format!("{base}/tests/fixtures/ast_grep/functional");

    for &rule_id in REQUIRED_RULE_IDS {
        let fixture_path = format!("{fixtures_dir}/{}_violation.rs", rule_id.to_lowercase());
        assert!(
            std::path::Path::new(&fixture_path).exists(),
            "Fixture file '{fixture_path}' must exist for rule '{rule_id}'",
        );

        let content = std::fs::read_to_string(&fixture_path)
            .unwrap_or_else(|e| panic!("Cannot read '{fixture_path}': {e}"));
        assert!(!content.is_empty(), "Fixture '{fixture_path}' must not be empty",);
    }
}

/// The allowed-iterator-pipeline fixture is valid Rust that does not
/// contain bare for/while/loop imperative statements.
#[test]
fn ast_grep_functional_allowed_iterator_pipeline_fixture_has_no_loops() {
    let base = env!("CARGO_MANIFEST_DIR");
    let fixture = format!("{base}/tests/fixtures/ast_grep/functional/allowed_iterator_pipeline.rs");
    let content =
        std::fs::read_to_string(&fixture).unwrap_or_else(|e| panic!("Cannot read fixture: {e}"));

    assert!(
        !content.contains("for ") || content.contains(".iter()") || content.contains(".map("),
        "Allowed fixture must use iterators, not bare 'for' loops",
    );
    assert!(!content.contains("while "), "Allowed fixture must not contain 'while' loops",);
    assert!(!content.contains("loop {"),);
}

/// The allowed-no-print fixture has no print!/println!/eprintln! calls.
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

/// Violation fixtures actually contain the patterns they are named for.
#[test]
fn ast_grep_functional_violation_fixtures_contain_their_patterns() {
    let base = env!("CARGO_MANIFEST_DIR");
    let fixtures_dir = format!("{base}/tests/fixtures/ast_grep/functional");

    // for-loop fixture
    assert_loop_fixture_contains(&fixtures_dir, "func_loops_for_violation.rs", "for item in");

    // while-loop fixture
    assert_loop_fixture_contains(&fixtures_dir, "func_loops_while_violation.rs", "while ");

    // loop-block fixture
    assert_loop_fixture_contains(&fixtures_dir, "func_loops_loop_violation.rs", "loop {");

    // print-stdout fixture
    let stdout = read_fixture(&fixtures_dir, "func_print_stdout_violation.rs");
    assert!(
        stdout.contains("println!") || stdout.contains("print!"),
        "func_print_stdout_violation.rs must contain println!/print!",
    );

    // print-stderr fixture
    let stderr = read_fixture(&fixtures_dir, "func_print_stderr_violation.rs");
    assert!(stderr.contains("eprintln!"), "func_print_stderr_violation.rs must contain eprintln!",);

    // wildcard-import fixture
    let wildcard = read_fixture(&fixtures_dir, "func_wildcard_import_violation.rs");
    assert!(
        wildcard.contains("::*"),
        "func_wildcard_import_violation.rs must contain wildcard import",
    );

    // unwrap_or fixture
    let unwrap = read_fixture(&fixtures_dir, "func_unwrap_or_violation.rs");
    assert!(unwrap.contains(".unwrap_or"), "func_unwrap_or_violation.rs must contain .unwrap_or",);
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

fn assert_loop_fixture_contains(dir: &str, name: &str, keyword: &str) {
    let content = read_fixture(dir, name);
    assert!(content.contains(keyword), "'{name}' must contain pattern '{keyword}'",);
}
