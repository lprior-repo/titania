//! Contract tests for the strict workspace lint configuration.

use std::collections::BTreeMap;

const ROOT_CARGO: &str = include_str!("../../../Cargo.toml");
const CLIPPY: &str = include_str!("../../../clippy.toml");

const REQUIRED_RUST_LINTS: &[(&str, &str)] = &[
    ("unsafe_code", "forbid"),
    ("unused_must_use", "deny"),
    ("unreachable_pub", "deny"),
    ("missing_docs", "deny"),
    ("unsafe_op_in_unsafe_fn", "deny"),
];

const REQUIRED_RUST_TABLE_LINTS: &[(&str, &str, &str)] = &[("rust_2018_idioms", "deny", "-1")];

const REQUIRED_CLIPPY_LINTS: &[(&str, &str)] = &[
    ("unwrap_used", "deny"),
    ("expect_used", "deny"),
    ("panic", "deny"),
    ("todo", "deny"),
    ("unimplemented", "deny"),
    ("indexing_slicing", "deny"),
    ("string_slice", "deny"),
    ("get_unwrap", "deny"),
    ("arithmetic_side_effects", "deny"),
    ("dbg_macro", "deny"),
    ("as_conversions", "deny"),
    ("await_holding_lock", "deny"),
    ("missing_errors_doc", "deny"),
    ("panic_in_result_fn", "deny"),
    ("print_stdout", "deny"),
    ("print_stderr", "deny"),
    ("integer_division", "deny"),
    ("integer_division_remainder_used", "deny"),
    ("modulo_arithmetic", "deny"),
    ("float_arithmetic", "deny"),
    ("allow_attributes", "deny"),
    ("allow_attributes_without_reason", "deny"),
    ("result_large_err", "deny"),
    ("result_unit_err", "deny"),
    ("map_err_ignore", "deny"),
    ("missing_panics_doc", "deny"),
    ("missing_safety_doc", "deny"),
    ("large_enum_variant", "deny"),
    ("cognitive_complexity", "deny"),
    ("too_many_arguments", "deny"),
    ("too_many_lines", "deny"),
    ("type_complexity", "deny"),
    ("excessive_nesting", "deny"),
    ("await_holding_refcell_ref", "deny"),
    ("future_not_send", "deny"),
    ("large_futures", "deny"),
    ("disallowed_methods", "deny"),
    ("disallowed_macros", "deny"),
    ("disallowed_types", "deny"),
    ("disallowed_fields", "deny"),
    ("multiple_crate_versions", "deny"),
    ("wildcard_dependencies", "deny"),
    ("negative_feature_names", "deny"),
    ("redundant_feature_names", "deny"),
];

const REQUIRED_CLIPPY_TABLE_LINTS: &[(&str, &str, &str)] = &[
    ("all", "deny", "-1"),
    ("pedantic", "deny", "-1"),
    ("nursery", "deny", "-1"),
    ("cargo", "deny", "-1"),
];

const REQUIRED_CLIPPY_THRESHOLDS: &[(&str, &str)] = &[
    ("too-many-lines-threshold", "60"),
    ("too-many-arguments-threshold", "5"),
    ("cognitive-complexity-threshold", "8"),
    ("excessive-nesting-threshold", "2"),
    ("type-complexity-threshold", "120"),
];

#[test]
fn workspace_lints() {
    let rust_lints = table(ROOT_CARGO, "workspace.lints.rust");
    let clippy_lints = table(ROOT_CARGO, "workspace.lints.clippy");
    let clippy_thresholds = flat_entries(CLIPPY);

    assert_scalar_lints(&rust_lints, REQUIRED_RUST_LINTS);
    assert_table_lints(&rust_lints, REQUIRED_RUST_TABLE_LINTS);
    assert_scalar_lints(&clippy_lints, REQUIRED_CLIPPY_LINTS);
    assert_table_lints(&clippy_lints, REQUIRED_CLIPPY_TABLE_LINTS);
    assert_entries(&clippy_thresholds, REQUIRED_CLIPPY_THRESHOLDS);
}

#[test]
fn workspace_lints_reject_weakened_fixture() {
    let weakened = ROOT_CARGO.replace("unwrap_used = \"deny\"", "unwrap_used = \"allow\"");
    let clippy_lints = table(&weakened, "workspace.lints.clippy");

    assert_eq!(scalar_lint(&clippy_lints, "unwrap_used"), Some("allow"));
    assert_ne!(scalar_lint(&clippy_lints, "unwrap_used"), Some("deny"));
}

#[test]
fn clippy_thresholds_reject_weakened_fixture() {
    let weakened = CLIPPY.replace("too-many-lines-threshold = 60", "too-many-lines-threshold = 80");
    let clippy_thresholds = flat_entries(&weakened);

    assert_eq!(clippy_thresholds.get("too-many-lines-threshold"), Some(&"80"));
    assert_ne!(clippy_thresholds.get("too-many-lines-threshold"), Some(&"40"));
}

fn assert_scalar_lints(entries: &BTreeMap<&str, &str>, required: &[(&str, &str)]) {
    required.iter().for_each(|(key, level)| {
        assert_eq!(scalar_lint(entries, key), Some(*level), "{key} must be {level}");
    });
}

fn assert_table_lints(entries: &BTreeMap<&str, &str>, required: &[(&str, &str, &str)]) {
    required.iter().for_each(|(key, level, priority)| {
        assert_eq!(
            table_lint(entries, key),
            Some((*level, *priority)),
            "{key} must be {level}/{priority}"
        );
    });
}

fn assert_entries(entries: &BTreeMap<&str, &str>, required: &[(&str, &str)]) {
    required.iter().for_each(|(key, value)| {
        assert_eq!(entries.get(key), Some(value), "{key} must be {value}");
    });
}

fn scalar_lint<'a>(entries: &'a BTreeMap<&str, &'a str>, key: &str) -> Option<&'a str> {
    entries.get(key).and_then(|value| quoted(value))
}

fn table_lint<'a>(entries: &'a BTreeMap<&str, &'a str>, key: &str) -> Option<(&'a str, &'a str)> {
    let value = entries.get(key)?;
    let fields = inline_table(value)?;
    Some((quoted(fields.get("level")?)?, fields.get("priority")?))
}

fn table<'a>(text: &'a str, section: &str) -> BTreeMap<&'a str, &'a str> {
    text.lines()
        .scan(false, |inside, line| Some(section_line_entries(inside, section, line)))
        .flatten()
        .collect()
}

fn section_line_entries<'a>(
    inside: &mut bool,
    section: &str,
    line: &'a str,
) -> Option<(&'a str, &'a str)> {
    let trimmed = line.split('#').next()?.trim();
    if trimmed.starts_with('[') {
        *inside = trimmed == format!("[{section}]");
        return None;
    }
    inside.then(|| split_entry(trimmed)).flatten()
}

fn flat_entries(text: &str) -> BTreeMap<&str, &str> {
    text.lines().filter_map(|line| split_entry(line.split('#').next()?.trim())).collect()
}

fn split_entry(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once('=')?;
    Some((key.trim(), value.trim()))
}

fn quoted(value: &str) -> Option<&str> {
    value.trim().strip_prefix('"')?.strip_suffix('"')
}

fn inline_table(value: &str) -> Option<BTreeMap<&str, &str>> {
    let inner = value.trim().strip_prefix('{')?.strip_suffix('}')?;
    Some(inner.split(',').filter_map(|part| split_entry(part.trim())).collect())
}
