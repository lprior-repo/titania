//! Contract tests for the embedded ast-grep bypass rule catalog.
//!
//! The rules are loaded at compile time via `include_str!` from
//! `crates/titania-lanes/rules/bypass.yml`.  Tests assert structural
//! properties of the YAML (rule IDs, language, severity, message, pattern,
//! effect, path exclusion, repair hints) so that any drift is caught before
//! the `AstGrep` lane runs.
//!
//! Fixture files under `tests/fixtures/ast_grep/bypass/` exercise each
//! rule in isolation; the YAML patterns are expected to match them.

use std::collections::BTreeSet;

// ---------------------------------------------------------------------------
// Compile-time YAML load — fails if the catalog is missing or malformed.
// ---------------------------------------------------------------------------

/// Raw YAML text of the embedded ast-grep bypass rules.
const BYPASS_YAML: &str = include_str!("../rules/bypass.yml");

// ---------------------------------------------------------------------------
// Required rule IDs and their expected repair hints.
// ---------------------------------------------------------------------------

/// The rule IDs that MUST appear in the bypass catalog.
const REQUIRED_RULE_IDS: &[&str] = &[
    "BYPASS_ALLOW_ATTR",
    "BYPASS_EXPECT_ATTR",
    "BYPASS_CFG_ATTR_ALLOW",
    "BYPASS_CRATE_ALLOW",
    "BYPASS_CRATE_EXPECT",
    "BYPASS_INLINE_SUPPRESSION",
    "BYPASS_GENERATED_INCLUDE",
];

/// Expected repair hints that MUST appear in at least one rule's metadata.
const REQUIRED_REPAIR_HINTS: &[&str] =
    &["RemoveBypassAttribute", "RemoveInlineSuppression", "RequiresHumanReview"];

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

/// Every required bypass rule ID appears in the YAML exactly once, and no
/// extra rule IDs are present.
#[test]
fn ast_grep_bypass_all_required_rule_ids_present_exactly_once() {
    let mut found = BTreeSet::new();
    for &id in REQUIRED_RULE_IDS {
        let count = count_occurrences(BYPASS_YAML, &format!("id: {id}"));
        assert_eq!(count, 1, "Rule ID '{id}' must appear exactly once (found {count})");
        assert!(found.insert(id.to_string()), "Duplicate rule ID '{id}'");
    }

    // No extra rule IDs: every `id:` line must be one of the required set.
    let all_ids = collect_rule_ids(BYPASS_YAML);
    for id in &all_ids {
        assert!(
            REQUIRED_RULE_IDS.contains(&id.as_str()),
            "Unexpected rule ID '{id}' in bypass catalog"
        );
    }
}

/// Every rule has language: Rust, a severity, a non-empty message,
/// and a pattern inside a rule body.
#[test]
fn ast_grep_bypass_every_rule_has_required_fields() {
    for &rule_id in REQUIRED_RULE_IDS {
        let block = extract_rule_block(BYPASS_YAML, rule_id)
            .unwrap_or_else(|| panic!("Rule block not found for '{rule_id}'"));

        // language: Rust
        assert_eq!(
            yaml_scalar_at(block, "language", 0),
            Some("Rust"),
            "'{rule_id}' must have language: Rust"
        );

        // severity (present, non-empty)
        assert!(
            yaml_scalar_at(block, "severity", 0).is_some(),
            "'{rule_id}' must have a severity field"
        );

        // message (present, non-empty)
        let msg =
            yaml_scalar_at(block, "message", 0).expect("'{rule_id}' must have a message field");
        assert!(!msg.is_empty(), "'{rule_id}' message must be non-empty");

        // Rule body with at least one ast-grep matcher.
        assert!(
            block.contains("rule:") || block.contains("rules:"),
            "'{rule_id}' must have a rule body"
        );
        assert!(
            block.contains("pattern:") || block.contains("regex:"),
            "'{rule_id}' must contain at least one pattern or regex matcher"
        );
    }
}

/// Every bypass rule carries effect: Reject.
#[test]
fn ast_grep_bypass_all_rules_reject_effect() {
    for &rule_id in REQUIRED_RULE_IDS {
        let block = extract_rule_block(BYPASS_YAML, rule_id)
            .expect("Rule block must exist for '{rule_id}'");
        assert!(
            block.contains("effect:") && block.to_lowercase().contains("reject"),
            "'{rule_id}' must carry effect: Reject"
        );
    }
}

/// Repair hints in metadata include RemoveBypassAttribute,
/// RemoveInlineSuppression, or RequiresHumanReview.
#[test]
fn ast_grep_bypass_repair_hints_present() {
    let mut found_hints = BTreeSet::new();

    for &rule_id in REQUIRED_RULE_IDS {
        let block = extract_rule_block(BYPASS_YAML, rule_id)
            .expect("Rule block must exist for '{rule_id}'");
        if let Some(hint) = yaml_scalar_at(block, "repair_hint", 2) {
            let _inserted = found_hints.insert(hint.to_string());
        }
    }

    // All three expected hints must appear somewhere across the six rules.
    for hint in REQUIRED_REPAIR_HINTS {
        assert!(
            found_hints.contains(*hint),
            "Repair hint '{hint}' must appear in at least one bypass rule's metadata"
        );
    }
}

/// Production-only path exclusions are present: tests, benches, examples,
/// and build.rs must be excluded from bypass scanning.
#[test]
fn ast_grep_bypass_production_only_path_exclusions_present() {
    for exclusion in PROD_EXCLUSIONS {
        assert!(
            BYPASS_YAML.contains("exclude:")
                || BYPASS_YAML.contains("files:")
                || BYPASS_YAML.contains("ignores:"),
            "Catalog must contain 'exclude:', 'files:', or 'ignores:' keys for path filtering"
        );
        assert!(
            BYPASS_YAML.contains(exclusion),
            "Path exclusion '{exclusion}' must be present in catalog"
        );
    }
}

/// Fixture files for each bypass violation type exist under fixtures/.
#[test]
fn ast_grep_bypass_fixture_files_exist_for_each_rule() {
    let base = env!("CARGO_MANIFEST_DIR");
    let fixtures_dir = format!("{base}/tests/fixtures/ast_grep/bypass");

    for &rule_id in REQUIRED_RULE_IDS {
        let fixture_path = format!("{fixtures_dir}/{}_violation.rs", rule_id.to_lowercase());
        assert!(
            std::path::Path::new(&fixture_path).exists(),
            "Fixture file '{fixture_path}' must exist for rule '{rule_id}'"
        );

        let content = std::fs::read_to_string(&fixture_path)
            .unwrap_or_else(|e| panic!("Cannot read '{fixture_path}': {e}"));
        assert!(!content.is_empty(), "Fixture '{fixture_path}' must not be empty");
    }
}

/// The clean fixture contains no bypass attributes or inline suppressions.
#[test]
fn ast_grep_bypass_clean_fixture_has_no_bypass_patterns() {
    let base = env!("CARGO_MANIFEST_DIR");
    let fixture = format!("{base}/tests/fixtures/ast_grep/bypass/clean_no_bypass.rs");
    let content =
        std::fs::read_to_string(&fixture).unwrap_or_else(|e| panic!("Cannot read fixture: {e}"));

    // No #[allow(...)], #![allow(...)], #[expect(...)], or #![expect(...)].
    assert!(
        !content.contains("#[allow(") && !content.contains("#![allow("),
        "Clean fixture must not contain #[allow(] or #![allow(]"
    );
    assert!(
        !content.contains("#[expect(") && !content.contains("#![expect("),
        "Clean fixture must not contain #[expect(] or #![expect(]"
    );
    assert!(
        !content.contains("ast-grep-ignore"),
        "Clean fixture must not contain 'ast-grep-ignore'"
    );
    assert!(!content.contains("sg-ignore"), "Clean fixture must not contain 'sg-ignore'");
    assert!(
        !content.contains("cfg_attr("),
        "Clean fixture must not contain '#[cfg_attr' with allow"
    );
}

/// Violation fixtures actually contain the patterns they are named for.
#[test]
fn ast_grep_bypass_violation_fixtures_contain_their_patterns() {
    let base = env!("CARGO_MANIFEST_DIR");
    let fixtures_dir = format!("{base}/tests/fixtures/ast_grep/bypass");

    // allow-attr fixture: must contain #[allow(
    let allow_attr = read_fixture(&fixtures_dir, "bypass_allow_attr_violation.rs");
    assert!(
        allow_attr.contains("#[allow("),
        "bypass_allow_attr_violation.rs must contain #[allow("
    );

    // expect-attr fixture: must contain #[expect(
    let expect_attr = read_fixture(&fixtures_dir, "bypass_expect_attr_violation.rs");
    assert!(
        expect_attr.contains("#[expect("),
        "bypass_expect_attr_violation.rs must contain #[expect("
    );

    // cfg-attr-allow fixture: must contain #[cfg_attr
    let cfg_attr = read_fixture(&fixtures_dir, "bypass_cfg_attr_allow_violation.rs");
    assert!(
        cfg_attr.contains("cfg_attr"),
        "bypass_cfg_attr_allow_violation.rs must contain #[cfg_attr"
    );

    // crate-allow fixture: must contain #![allow(
    let crate_allow = read_fixture(&fixtures_dir, "bypass_crate_allow_violation.rs");
    assert!(
        crate_allow.contains("#![allow("),
        "bypass_crate_allow_violation.rs must contain #![allow("
    );

    // crate-expect fixture: must contain #![expect(
    let crate_expect = read_fixture(&fixtures_dir, "bypass_crate_expect_violation.rs");
    assert!(
        crate_expect.contains("#![expect("),
        "bypass_crate_expect_violation.rs must contain #![expect("
    );

    // inline-suppression fixture: must contain ast-grep-ignore or sg-ignore
    let inline_sup = read_fixture(&fixtures_dir, "bypass_inline_suppression_violation.rs");
    assert!(
        inline_sup.contains("ast-grep-ignore") || inline_sup.contains("sg-ignore"),
        "bypass_inline_suppression_violation.rs must contain ast-grep-ignore or sg-ignore"
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
