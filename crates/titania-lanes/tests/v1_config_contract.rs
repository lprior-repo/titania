#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_macros,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    clippy::missing_panics_doc,
    clippy::panic_in_result_fn,
    clippy::cognitive_complexity,
    clippy::doc_markdown,
    clippy::excessive_nesting,
    clippy::many_single_char_names,
    clippy::integer_division,
    clippy::integer_division_remainder_used,
    clippy::missing_errors_doc,
    clippy::type_complexity,
    clippy::needless_borrow,
    clippy::useless_vec,
    clippy::map_identity,
    reason = "Tests are exempt from the strict production deny list per project doctrine."
)]
//! v1_config_contract: integration test.
use std::collections::BTreeMap;

const ROOT_CARGO: &str = include_str!("../../../Cargo.toml");
const CLIPPY: &str = include_str!("../../../clippy.toml");

const REQUIRED_RUST_LINTS: &[(&str, &str)] = &[
    ("unsafe_code", "forbid"),
    ("unsafe_op_in_unsafe_fn", "deny"),
    ("unused_must_use", "deny"),
    ("unused_results", "deny"),
    ("let_underscore_drop", "deny"),
    ("elided_lifetimes_in_paths", "deny"),
    ("explicit_outlives_requirements", "deny"),
    ("missing_debug_implementations", "deny"),
    ("missing_docs", "deny"),
    ("unreachable_pub", "deny"),
    ("trivial_casts", "deny"),
    ("trivial_numeric_casts", "deny"),
    ("variant_size_differences", "deny"),
    ("unused_extern_crates", "deny"),
    ("unused_import_braces", "deny"),
    ("keyword_idents_2024", "deny"),
];

const REQUIRED_RUST_TABLE_LINTS: &[(&str, &str, &str)] = &[
    ("warnings", "deny", "-1"),
    ("future_incompatible", "deny", "-1"),
    ("rust_2018_idioms", "deny", "-1"),
];

const REQUIRED_CLIPPY_LINTS: &[(&str, &str)] = &[
    ("unwrap_used", "deny"),
    ("expect_used", "deny"),
    ("unwrap_in_result", "deny"),
    ("panic", "deny"),
    ("panic_in_result_fn", "deny"),
    ("todo", "deny"),
    ("unimplemented", "deny"),
    ("unreachable", "deny"),
    ("dbg_macro", "deny"),
    ("print_stdout", "deny"),
    ("print_stderr", "deny"),
    ("indexing_slicing", "deny"),
    ("string_slice", "deny"),
    ("get_unwrap", "deny"),
    ("arithmetic_side_effects", "deny"),
    ("as_conversions", "deny"),
    ("cast_possible_truncation", "deny"),
    ("cast_possible_wrap", "deny"),
    ("cast_sign_loss", "deny"),
    ("cast_precision_loss", "deny"),
    ("integer_division", "deny"),
    ("integer_division_remainder_used", "deny"),
    ("modulo_arithmetic", "deny"),
    ("float_arithmetic", "deny"),
    ("let_underscore_must_use", "deny"),
    ("await_holding_lock", "deny"),
    ("result_large_err", "deny"),
    ("result_unit_err", "deny"),
    ("map_err_ignore", "deny"),
    ("missing_errors_doc", "deny"),
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
    ("allow_attributes", "deny"),
    ("allow_attributes_without_reason", "deny"),
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
    ("too-many-lines-threshold", "40"),
    ("too-many-arguments-threshold", "5"),
    ("cognitive-complexity-threshold", "8"),
    ("excessive-nesting-threshold", "2"),
    ("type-complexity-threshold", "120"),
    ("single-char-binding-names-threshold", "2"),
    ("enum-variant-size-threshold", "128"),
    ("large-error-threshold", "64"),
    ("future-size-threshold", "4096"),
    ("stack-size-threshold", "262144"),
    ("array-size-threshold", "4096"),
    ("too-large-for-stack", "128"),
    ("pass-by-value-size-limit", "128"),
    ("trivial-copy-size-limit", "16"),
    ("vec-box-size-threshold", "1024"),
    ("unnecessary-box-size", "64"),
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
    let weakened = CLIPPY.replace("too-many-lines-threshold = 40", "too-many-lines-threshold = 80");
    let clippy_thresholds = flat_entries(&weakened);

    assert_eq!(clippy_thresholds.get("too-many-lines-threshold"), Some(&"80"));
    assert_ne!(clippy_thresholds.get("too-many-lines-threshold"), Some(&"40"));
}

fn assert_scalar_lints(entries: &BTreeMap<&str, &str>, required: &[(&str, &str)]) {
    for (key, level) in required {
        assert!(
            scalar_lint(entries, key) == Some(*level),
            "{key} must be {level}"
        );
    }
}

fn assert_table_lints(entries: &BTreeMap<&str, &str>, required: &[(&str, &str, &str)]) {
    for (key, level, priority) in required {
        assert!(
            table_lint(entries, key) == Some((*level, *priority)),
            "{key} must be {level}/{priority}"
        );
    }
}

fn assert_entries(entries: &BTreeMap<&str, &str>, required: &[(&str, &str)]) {
    for (key, value) in required {
        assert!(entries.get(key) == Some(value), "{key} must be {value}");
    }
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
