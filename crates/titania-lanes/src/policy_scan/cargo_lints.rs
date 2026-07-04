//! Cargo manifest lint-weakening scanner.
//!
//! Detects `[lints.*]` and `[workspace.lints.*]` entries that lower required
//! strict lint levels. Manifests without lint tables are not applicable and
//! produce no findings.

mod line_lookup;

use std::path::Path;

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
    ExpectedLint::warn("rust", "deprecated"),
    ExpectedLint::warn("rust", "future_incompatible"),
    ExpectedLint::warn("rust", "nonstandard_style"),
    ExpectedLint::warn("rustdoc", "broken_intra_doc_links"),
    ExpectedLint::warn("rustdoc", "private_intra_doc_links"),
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

    const fn warn(category: &'static str, key: &'static str) -> Self {
        Self { category, key, required: LintLevel::Warn }
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

struct FindingContext<'a> {
    manifest_path: &'a Path,
    content: &'a str,
    document: &'a DocumentMut,
    rule: &'a RuleId,
}

/// Scan one Cargo manifest for strict-lint weakening.
///
/// `manifest_path` is resolved relative to `root` and is also used as the
/// finding path. Missing files, malformed TOML, and manifests without lint
/// tables are clean for this scanner.
///
/// # Errors
/// Returns [`RuleIdError`] if the embedded finding rule identifier is invalid.
pub fn scan_cargo_lints_weakening(
    root: &Path,
    manifest_path: &Path,
    report: &mut LaneReport,
) -> Result<bool, RuleIdError> {
    std::fs::read_to_string(root.join(manifest_path))
        .map_or(Ok(false), |content| scan_manifest_content(manifest_path, &content, report))
}

/// Scan manifest content after the caller has loaded it.
///
/// # Errors
/// Returns [`RuleIdError`] if the finding rule literal is invalid.
fn scan_manifest_content(
    manifest_path: &Path,
    content: &str,
    report: &mut LaneReport,
) -> Result<bool, RuleIdError> {
    match parse_manifest(content) {
        Some(document) if manifest_has_lints(&document) => {
            report.record_scan();
            scan_document_lints(manifest_path, content, &document, report)
        }
        Some(_) | None => Ok(false),
    }
}

fn parse_manifest(content: &str) -> Option<DocumentMut> {
    content.parse::<DocumentMut>().ok()
}

fn manifest_has_lints(document: &DocumentMut) -> bool {
    LINT_TABLES.iter().any(|table_ref| {
        lint_root_table(document, table_ref.prefix).is_some_and(|table| !table.is_empty())
    })
}

/// Scan parsed manifest lint tables.
///
/// # Errors
/// Returns [`RuleIdError`] if the finding rule literal is invalid.
fn scan_document_lints(
    manifest_path: &Path,
    content: &str,
    document: &DocumentMut,
    report: &mut LaneReport,
) -> Result<bool, RuleIdError> {
    let rule = RuleId::new(RULE_WEAKENING)?;
    let finding_context = FindingContext { manifest_path, content, document, rule: &rule };
    let findings = LINT_TABLES
        .iter()
        .flat_map(|table_ref| table_findings(&finding_context, table_ref))
        .collect::<Vec<_>>();
    let has_findings = !findings.is_empty();

    if has_findings {
        report.extend_finding(findings);
    } else {
        report.record_pass();
    }

    Ok(has_findings)
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
    if !actual.is_weaker_than(expected.required) {
        return None;
    }
    Some(Finding::new(
        context.rule.clone(),
        context.manifest_path.display().to_string(),
        find_lint_line(context.content, table_ref.prefix, expected.category, expected.key),
        format!(
            "{} is {} in {} (required {}) - lint weakened",
            lint_name(table_ref, expected),
            actual.as_str(),
            context.manifest_path.display(),
            expected.required.as_str(),
        ),
    ))
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
