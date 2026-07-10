//! Cargo manifest lint-weakening scanner.
//!
//! Detects `[lints.*]` and `[workspace.lints.*]` entries that lower required
//! strict lint levels, plus missing workspace-lint inheritance on root and
//! member manifests. A root manifest (path is exactly `Cargo.toml`) must
//! workspace-lint inheritance on root and member manifests. A root manifest
//! (path is exactly `Cargo.toml`) must declare a non-empty `[workspace.lints]`
//! table; a member manifest (subdirectory) must declare `[lints]` with
//! `workspace = true`.
//!
//! The required lint levels can be lowered at runtime by a checked-in
//! `.titania/profiles/strict-ai/policy.toml` `[lints]` override map (see
//! v1-spec.md §9.7). Overrides only ever *weaken* the required level; an
//! override that is stronger than the default, references an unknown lint,
//! or carries an unparsable level is silently ignored so the embedded
//! binary defaults always act as a floor.

use std::{collections::BTreeMap, path::Path};

mod line_lookup;
use toml_edit::{DocumentMut, Item, Table, Value};

use crate::{Finding, LaneReport, RuleId, RuleIdError};

use line_lookup::find_lint_line;

const RULE_WEAKENING: &str = "BYPASS_CARGO_LINTS_WEAKENING";
const EXPECTED_LEVELS: &[ExpectedLint] = &[
    ExpectedLint::deny("clippy", "panic"),
    ExpectedLint::deny("clippy", "pedantic"),
    ExpectedLint::deny("clippy", "unwrap_used"),
    ExpectedLint::deny("clippy", "expect_used"),
    ExpectedLint::deny("clippy", "todo"),
    ExpectedLint::deny("clippy", "unimplemented"),
    ExpectedLint::deny("clippy", "indexing_slicing"),
    ExpectedLint::deny("clippy", "string_slice"),
    ExpectedLint::deny("clippy", "dbg_macro"),
    ExpectedLint::deny("clippy", "as_conversions"),
    ExpectedLint::deny("rust", "unsafe_code"),
    ExpectedLint::deny("rust", "unreachable_pub"),
    ExpectedLint::deny("rust", "missing_debug_implementations"),
    ExpectedLint::deny("rust", "future_incompatible"),
    ExpectedLint::deny("rustdoc", "broken_intra_doc_links"),
    ExpectedLint::deny("rustdoc", "private_intra_doc_links"),
];

const LINT_TABLES: &[LintTableRef] = &[
    LintTableRef { prefix: "lints", include_prefix_in_message: false },
    LintTableRef { prefix: "workspace.lints", include_prefix_in_message: true },
];

#[derive(Debug, Clone, Copy)]
struct LintTableRef {
    prefix: &'static str,
    include_prefix_in_message: bool,
}

#[derive(Debug, Clone, Copy)]
struct ExpectedLint {
    category: &'static str,
    key: &'static str,
    required: LintLevel,
}

impl ExpectedLint {
    const fn deny(category: &'static str, key: &'static str) -> Self {
        Self { category, key, required: LintLevel::Deny }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LintLevel {
    Allow,
    Warn,
    Deny,
    Forbid,
}

impl LintLevel {
    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "allow" => Some(Self::Allow),
            "warn" => Some(Self::Warn),
            "deny" => Some(Self::Deny),
            "forbid" => Some(Self::Forbid),
            _ => None,
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Warn => "warn",
            Self::Deny => "deny",
            Self::Forbid => "forbid",
        }
    }

    const fn rank(self) -> u8 {
        match self {
            Self::Allow => 0,
            Self::Warn => 1,
            Self::Deny => 2,
            Self::Forbid => 3,
        }
    }

    const fn is_weaker_than(self, required: Self) -> bool {
        self.rank() < required.rank()
    }
}

#[derive(Debug, Clone, Copy)]
struct InheritanceDirective {
    section: &'static str,
    directive: &'static str,
    summary: &'static str,
}

impl InheritanceDirective {
    const fn new(section: &'static str, directive: &'static str, summary: &'static str) -> Self {
        Self { section, directive, summary }
    }

    fn line(&self, content: &str) -> u32 {
        find_lint_line(content, self.section, self.directive, "")
    }
}

struct FindingContext<'a> {
    manifest_path: &'a Path,
    content: &'a str,
    document: &'a DocumentMut,
    rule: &'a RuleId,
    /// Optional `policy.toml` `[lints]` override map. `None` keeps the
    /// compile-time default required levels; `Some(map)` may lower them.
    lint_overrides: Option<&'a BTreeMap<String, String>>,
}

/// Scan one Cargo manifest for strict-lint weakening and missing
/// workspace-lint inheritance using only the binary defaults.
///
/// See [`scan_cargo_lints_weakening_with_overrides`] for the full contract;
/// this is a thin shim that passes `None` for the override map.
///
/// # Errors
/// Returns [`RuleIdError`] if the embedded finding rule identifier is invalid.
pub fn scan_cargo_lints_weakening(
    root: &Path,
    manifest_path: &Path,
    report: &mut LaneReport,
) -> Result<bool, RuleIdError> {
    scan_cargo_lints_weakening_with_overrides(root, manifest_path, None, report)
}

/// Scan one Cargo manifest for strict-lint weakening and missing
/// workspace-lint inheritance, optionally applying a checked-in
/// `policy.toml` `[lints]` override map.
///
/// `manifest_path` is resolved relative to `root` and is also used as the
/// finding path. Missing files and malformed TOML are clean for this
/// scanner. A root manifest (path is exactly `Cargo.toml`) is clean only
/// when it declares a non-empty `[workspace.lints]` table; a member
/// manifest (subdirectory) is clean only when it declares `[lints]` with
/// `workspace = true`.
///
/// When `lint_overrides` is `Some`, each key is `{category}::{key}` (e.g.
/// `"clippy::needless_return"`) and the value is one of `allow`, `warn`,
/// `deny`, or `forbid`. An override only ever *lowers* the required
/// level: a stronger override, an unknown lint, or an unparsable level
/// is silently ignored so the embedded defaults always act as a floor.
///
/// # Errors
/// Returns [`RuleIdError`] if the embedded finding rule identifier is invalid.
pub fn scan_cargo_lints_weakening_with_overrides(
    root: &Path,
    manifest_path: &Path,
    lint_overrides: Option<&BTreeMap<String, String>>,
    report: &mut LaneReport,
) -> Result<bool, RuleIdError> {
    std::fs::read_to_string(root.join(manifest_path)).map_or(Ok(false), |content| {
        scan_manifest_content(manifest_path, &content, lint_overrides, report)
    })
}

/// Scan manifest content after the caller has loaded it.
///
/// `lint_overrides` is the optional `policy.toml` `[lints]` map; when
/// present, an entry `{category}::{key}` → level may lower the required
/// level for a single lint. See
/// [`scan_cargo_lints_weakening_with_overrides`] for the full contract.
///
/// # Errors
/// Returns [`RuleIdError`] if the finding rule identifier is invalid.
fn scan_manifest_content(
    manifest_path: &Path,
    content: &str,
    lint_overrides: Option<&BTreeMap<String, String>>,
    report: &mut LaneReport,
) -> Result<bool, RuleIdError> {
    let Some(document) = parse_manifest(content) else {
        return Ok(false);
    };
    let rule = RuleId::new(RULE_WEAKENING)?;
    let inheritance_findings =
        missing_inheritance_finding(manifest_path, content, &document, &rule);
    let has_lint_tables = manifest_has_lints(&document);
    let has_inheritance_findings = !inheritance_findings.is_empty();
    if has_inheritance_findings || has_lint_tables {
        report.record_scan();
    }
    let weakening_findings = if has_lint_tables {
        scan_document_lints(manifest_path, content, &document, &rule, lint_overrides)
    } else {
        Vec::new()
    };
    let has_findings = !inheritance_findings.is_empty() || !weakening_findings.is_empty();
    if has_findings {
        report.extend_finding(inheritance_findings);
        report.extend_finding(weakening_findings);
    } else if has_lint_tables {
        report.record_pass();
    }
    Ok(has_findings)
}

fn parse_manifest(content: &str) -> Option<DocumentMut> {
    content.parse::<DocumentMut>().ok()
}

fn manifest_has_lints(document: &DocumentMut) -> bool {
    LINT_TABLES.iter().any(|table_ref| {
        lint_root_table(document, table_ref.prefix).is_some_and(|table| !table.is_empty())
    })
}

/// Classify a manifest path as either root (`Cargo.toml` at the workspace
/// root) or a workspace member (any other relative manifest path).
fn is_root_manifest(manifest_path: &Path) -> bool {
    manifest_path.parent().is_none_or(|parent| parent.as_os_str().is_empty())
}

fn root_lint_pins_required(document: &DocumentMut, expected: &ExpectedLint) -> bool {
    lint_item(document, "workspace.lints", expected.category, expected.key)
        .and_then(lint_level_from_item)
        .is_some_and(|actual| !actual.is_weaker_than(expected.required))
}

fn has_member_inheritance(document: &DocumentMut) -> bool {
    document
        .get("lints")
        .and_then(Item::as_table)
        .and_then(|table| table.get("workspace"))
        .and_then(Item::as_value)
        .and_then(Value::as_bool)
        == Some(true)
}

/// Emit v1 §9.1 inheritance findings.
///
/// Root manifests must pin every canonical entry under
/// `[workspace.lints.*]` at or above the required level: one typed
/// `BYPASS_CARGO_LINTS_WEAKENING` finding per absent or below-required
/// entry, plus a sentinel when the table itself is missing.
///
/// Member manifests must declare `[lints] workspace = true`.
fn missing_inheritance_finding(
    manifest_path: &Path,
    content: &str,
    document: &DocumentMut,
    rule: &RuleId,
) -> Vec<Finding> {
    if is_root_manifest(manifest_path) {
        return root_inheritance_finding(manifest_path, content, document, rule);
    }
    if has_member_inheritance(document) {
        return Vec::new();
    }
    vec![member_inheritance_finding(manifest_path, content, rule)]
}

/// Classify a root manifest and emit the appropriate inheritance findings.
///
/// A root manifest with no `[workspace]` table is a single-package crate
/// root and is out of scope for §9.1 inheritance. A virtual root with
/// both `[workspace]` and `[package]` is a self-hosted member and
/// §9.10 step 0 mandates `[lints] workspace = true`. Otherwise the
/// canonical-table rules apply and every missing canonical entry
/// emits its own typed finding.
fn root_inheritance_finding(
    manifest_path: &Path,
    content: &str,
    document: &DocumentMut,
    rule: &RuleId,
) -> Vec<Finding> {
    if document.get("workspace").and_then(Item::as_table).is_none() {
        return Vec::new();
    }
    let self_hosted_member = root_package_member_branch(document);
    if self_hosted_member && !has_member_inheritance(document) {
        return vec![member_inheritance_finding(manifest_path, content, rule)];
    }
    missing_root_inheritance_finding(manifest_path, content, document, rule)
}

/// v1 §9.10 step 0 requires `[lints] workspace = true` for every
/// workspace member crate.
///
/// The workspace root is conventionally a member of itself only when it
/// ALSO declares `[package]` (the `members=["."]` self-hosted root
/// case — see `titania/template/Cargo.toml`); a pure virtual root with
/// only `[workspace]` is table-only.
fn root_package_member_branch(document: &DocumentMut) -> bool {
    document.get("workspace").and_then(Item::as_table).is_some()
        && document.get("package").and_then(Item::as_table).is_some()
}

/// Build the per-root canonical-table findings or sentinel when the
/// table itself is missing.
///
/// Returns one typed `BYPASS_CARGO_LINTS_WEAKENING` finding per absent
/// or below-required canonical entry so editors can triage every gap
/// individually.
fn missing_root_inheritance_finding(
    manifest_path: &Path,
    content: &str,
    document: &DocumentMut,
    rule: &RuleId,
) -> Vec<Finding> {
    if lint_root_table(document, "workspace.lints").is_none() {
        return vec![inheritance_finding(rule, manifest_path, content, ROOT_LINTS_TABLE)];
    }
    missing_root_lint_entries(manifest_path, content, document, rule)
}

fn missing_root_lint_entries(
    manifest_path: &Path,
    content: &str,
    document: &DocumentMut,
    rule: &RuleId,
) -> Vec<Finding> {
    EXPECTED_LEVELS
        .iter()
        .filter_map(|expected| {
            missing_root_entry_finding(manifest_path, content, document, rule, expected)
        })
        .collect()
}

fn missing_root_entry_finding(
    manifest_path: &Path,
    content: &str,
    document: &DocumentMut,
    rule: &RuleId,
    expected: &ExpectedLint,
) -> Option<Finding> {
    if root_lint_pins_required(document, expected) {
        return None;
    }
    let actual = lint_item(document, "workspace.lints", expected.category, expected.key)
        .and_then(lint_level_from_item);
    let category_section = format!("workspace.lints.{}", expected.category);
    let line = find_lint_line(content, "workspace.lints", expected.category, expected.key);
    Some(Finding::new(
        rule.clone(),
        manifest_path.display().to_string(),
        line,
        format!(
            "[{}] does not pin {} = {} (required {} or stronger) - canonical lint policy missing",
            category_section,
            expected.key,
            actual.map_or("missing", LintLevel::as_str),
            expected.required.as_str(),
        ),
    ))
}

const MEMBER_INHERITANCE: InheritanceDirective = InheritanceDirective::new(
    "lints",
    "workspace",
    "member manifest missing required [lints] workspace = true inheritance",
);
const ROOT_LINTS_TABLE: InheritanceDirective = InheritanceDirective::new(
    "workspace.lints",
    "root",
    "root manifest missing required [workspace.lints] table",
);

fn member_inheritance_finding(manifest_path: &Path, content: &str, rule: &RuleId) -> Finding {
    inheritance_finding(rule, manifest_path, content, MEMBER_INHERITANCE)
}

fn inheritance_finding(
    rule: &RuleId,
    manifest_path: &Path,
    content: &str,
    directive: InheritanceDirective,
) -> Finding {
    Finding::new(
        rule.clone(),
        manifest_path.display().to_string(),
        directive.line(content),
        format!(
            "{} ({}); bypasses strict-ai workspace-lint inheritance",
            directive.summary,
            manifest_path.display(),
        ),
    )
}

/// Collect weakening findings for parsed manifest lint tables.
///
/// Returns the list of weakening findings; the caller is responsible for
/// recording pass/fail counters and pushing findings to the report.
///
/// # Errors
/// Returns [`RuleIdError`] if the finding rule identifier is invalid
/// (only reachable via internal misuse; the public entry validates first).
fn scan_document_lints(
    manifest_path: &Path,
    content: &str,
    document: &DocumentMut,
    rule: &RuleId,
    lint_overrides: Option<&BTreeMap<String, String>>,
) -> Vec<Finding> {
    let finding_context = FindingContext { manifest_path, content, document, rule, lint_overrides };
    LINT_TABLES.iter().flat_map(|table_ref| table_findings(&finding_context, table_ref)).collect()
}

fn table_findings(context: &FindingContext<'_>, table_ref: &LintTableRef) -> Vec<Finding> {
    EXPECTED_LEVELS
        .iter()
        .filter_map(|expected| weakening_finding(context, table_ref, expected))
        .collect()
}

fn weakening_finding(
    context: &FindingContext<'_>,
    table_ref: &LintTableRef,
    expected: &ExpectedLint,
) -> Option<Finding> {
    let actual = lint_item(context.document, table_ref.prefix, expected.category, expected.key)
        .and_then(lint_level_from_item)?;
    let required = effective_required_level(context.lint_overrides, expected);
    if !actual.is_weaker_than(required) {
        return None;
    }
    Some(Finding::new(
        context.rule.clone(),
        context.manifest_path.display().to_string(),
        line_lookup::find_lint_line(
            context.content,
            table_ref.prefix,
            expected.category,
            expected.key,
        ),
        format!(
            "{} is {} in {} (required {}) - lint weakened",
            lint_name(table_ref, expected),
            actual.as_str(),
            context.manifest_path.display(),
            required.as_str(),
        ),
    ))
}

/// Resolve the effective required level for one expected lint, applying
/// the `policy.toml` `[lints]` override map when present.
///
/// Overrides only ever *lower* the required level: a stronger override,
/// an unparsable level, or an unknown lint key leaves the default
/// unchanged. This guarantees the binary defaults act as a strict floor.
fn effective_required_level(
    overrides: Option<&BTreeMap<String, String>>,
    expected: &ExpectedLint,
) -> LintLevel {
    let Some(map) = overrides else {
        return expected.required;
    };
    let key = format!("{}::{}", expected.category, expected.key);
    let Some(value) = map.get(&key) else {
        return expected.required;
    };
    let Some(parsed) = LintLevel::parse(value) else {
        return expected.required;
    };
    if parsed.is_weaker_than(expected.required) { parsed } else { expected.required }
}

fn lint_root_table<'a>(document: &'a DocumentMut, prefix: &str) -> Option<&'a Table> {
    match prefix {
        "workspace.lints" => document
            .get("workspace")
            .and_then(Item::as_table)
            .and_then(|workspace| workspace.get("lints"))
            .and_then(Item::as_table),
        _ => document.get("lints").and_then(Item::as_table),
    }
}

fn lint_item<'a>(
    document: &'a DocumentMut,
    table_prefix: &str,
    category: &str,
    key: &str,
) -> Option<&'a Item> {
    lint_root_table(document, table_prefix)
        .and_then(|lints| lints.get(category))
        .and_then(Item::as_table)
        .and_then(|category_table| category_table.get(key))
}

fn lint_level_from_item(item: &Item) -> Option<LintLevel> {
    item.as_value().and_then(lint_level_from_value).or_else(|| table_level(item))
}

fn table_level(item: &Item) -> Option<LintLevel> {
    item.as_table().and_then(|table| table.get("level")).and_then(lint_level_from_item)
}

fn lint_level_from_value(value: &Value) -> Option<LintLevel> {
    value.as_str().or_else(|| inline_level(value)).and_then(LintLevel::parse)
}

fn inline_level(value: &Value) -> Option<&str> {
    value.as_inline_table().and_then(|table| table.get("level")).and_then(Value::as_str)
}

fn lint_name(table_ref: &LintTableRef, expected: &ExpectedLint) -> String {
    if table_ref.include_prefix_in_message {
        return format!("{}.{}.{}", table_ref.prefix, expected.category, expected.key);
    }
    format!("{}.{}", expected.category, expected.key)
}
