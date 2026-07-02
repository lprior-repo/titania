use std::collections::BTreeMap;

const ROOT_CARGO: &str = include_str!("../../../Cargo.toml");
const CLIPPY: &str = include_str!("../../../clippy.toml");

const REQUIRED_RUST_LINTS: &[(&str, &str)] = &[
    ("unsafe_code", "forbid"),
    ("unused_must_use", "deny"),
    ("unreachable_pub", "allow"),
    ("non_exhaustive_omitted_patterns", "deny"),
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
    ("let_underscore_must_use", "deny"),
    ("await_holding_lock", "deny"),
    ("unwrap_or_default", "deny"),
    ("exit", "deny"),
    ("default_numeric_fallback", "deny"),
    ("missing_errors_doc", "deny"),
];

const REQUIRED_CLIPPY_TABLE_LINTS: &[(&str, &str, &str)] = &[
    ("all", "deny", "-1"),
    ("pedantic", "allow", "1"),
    ("nursery", "allow", "1"),
    ("cargo", "allow", "1"),
    ("multiple_crate_versions", "allow", "1"),
];

const REQUIRED_CLIPPY_THRESHOLDS: &[(&str, &str)] = &[
    ("too-many-lines-threshold", "40"),
    ("too-many-arguments-threshold", "5"),
    ("max-fn-params-bools", "1"),
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
