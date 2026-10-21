use std::{collections::BTreeMap, path::Path};

mod line_lookup;
use toml_edit::{DocumentMut, Item, Table, Value};

use crate::{Finding, LaneReport, RuleId, RuleIdError};

use line_lookup::find_lint_line;

const RULE_WEAKENING: &str = "BYPASS_CARGO_LINTS_WEAKENING";
const EXPECTED_LEVELS: &[ExpectedLint] = &[
    ExpectedLint::deny("rust", "warnings"),
    ExpectedLint::deny("rust", "future_incompatible"),
    ExpectedLint::deny("rust", "rust_2018_idioms"),
    ExpectedLint::forbid("rust", "unsafe_code"),
    ExpectedLint::deny("rust", "unsafe_op_in_unsafe_fn"),
    ExpectedLint::deny("rust", "unused_must_use"),
    ExpectedLint::deny("rust", "unused_results"),
    ExpectedLint::deny("rust", "let_underscore_drop"),
    ExpectedLint::deny("rust", "elided_lifetimes_in_paths"),
    ExpectedLint::deny("rust", "explicit_outlives_requirements"),
    ExpectedLint::deny("rust", "unreachable_pub"),
    ExpectedLint::deny("rust", "trivial_casts"),
    ExpectedLint::deny("rust", "trivial_numeric_casts"),
    ExpectedLint::deny("rust", "variant_size_differences"),
    ExpectedLint::deny("rust", "unused_extern_crates"),
    ExpectedLint::deny("rust", "unused_import_braces"),
    ExpectedLint::deny("rust", "keyword_idents_2024"),
    ExpectedLint::deny("rustdoc", "bare_urls"),
    ExpectedLint::deny("clippy", "all"),
    ExpectedLint::deny("clippy", "cargo"),
    ExpectedLint::deny("clippy", "pedantic"),
    ExpectedLint::deny("clippy", "nursery"),
    ExpectedLint::deny("clippy", "allow_attributes"),
    ExpectedLint::deny("clippy", "allow_attributes_without_reason"),
    ExpectedLint::deny("clippy", "unwrap_used"),
    ExpectedLint::deny("clippy", "expect_used"),
    ExpectedLint::deny("clippy", "unwrap_in_result"),
    ExpectedLint::deny("clippy", "panic"),
    ExpectedLint::deny("clippy", "panic_in_result_fn"),
    ExpectedLint::deny("clippy", "todo"),
    ExpectedLint::deny("clippy", "unimplemented"),
    ExpectedLint::deny("clippy", "unreachable"),
    ExpectedLint::deny("clippy", "dbg_macro"),
    ExpectedLint::deny("clippy", "print_stdout"),
    ExpectedLint::deny("clippy", "print_stderr"),
    ExpectedLint::deny("clippy", "indexing_slicing"),
    ExpectedLint::deny("clippy", "string_slice"),
    ExpectedLint::deny("clippy", "get_unwrap"),
    ExpectedLint::deny("clippy", "arithmetic_side_effects"),
    ExpectedLint::deny("clippy", "as_conversions"),
    ExpectedLint::deny("clippy", "cast_possible_truncation"),
    ExpectedLint::deny("clippy", "cast_possible_wrap"),
    ExpectedLint::deny("clippy", "cast_sign_loss"),
    ExpectedLint::deny("clippy", "cast_precision_loss"),
    ExpectedLint::deny("clippy", "integer_division"),
    ExpectedLint::deny("clippy", "integer_division_remainder_used"),
    ExpectedLint::deny("clippy", "modulo_arithmetic"),
    ExpectedLint::deny("clippy", "float_arithmetic"),
    ExpectedLint::deny("clippy", "result_large_err"),
    ExpectedLint::deny("clippy", "result_unit_err"),
    ExpectedLint::deny("clippy", "map_err_ignore"),
    ExpectedLint::deny("clippy", "large_enum_variant"),
    ExpectedLint::deny("clippy", "cognitive_complexity"),
    ExpectedLint::deny("clippy", "too_many_arguments"),
    ExpectedLint::deny("clippy", "too_many_lines"),
    ExpectedLint::deny("clippy", "type_complexity"),
    ExpectedLint::deny("clippy", "excessive_nesting"),
    ExpectedLint::deny("clippy", "await_holding_lock"),
    ExpectedLint::deny("clippy", "await_holding_refcell_ref"),
    ExpectedLint::deny("clippy", "future_not_send"),
    ExpectedLint::deny("clippy", "large_futures"),
    ExpectedLint::deny("clippy", "disallowed_methods"),
    ExpectedLint::deny("clippy", "disallowed_macros"),
    ExpectedLint::deny("clippy", "disallowed_types"),
    ExpectedLint::deny("clippy", "disallowed_fields"),
    ExpectedLint::deny("clippy", "multiple_crate_versions"),
    ExpectedLint::deny("clippy", "wildcard_dependencies"),
    ExpectedLint::deny("clippy", "negative_feature_names"),
    ExpectedLint::deny("clippy", "redundant_feature_names"),
    ExpectedLint::deny("clippy", "let_underscore_must_use"),
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

    const fn forbid(category: &'static str, key: &'static str) -> Self {
        Self { category, key, required: LintLevel::Forbid }
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
    lint_overrides: Option<&'a BTreeMap<String, String>>,
}

pub fn scan_cargo_lints_weakening(
    root: &Path,
    manifest_path: &Path,
    report: &mut LaneReport,
) -> Result<bool, RuleIdError> {
    scan_cargo_lints_weakening_with_overrides(root, manifest_path, None, report)
}

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
    let Ok(document) = content.parse::<DocumentMut>() else {
        return None;
    };
    Some(document)
}

fn manifest_has_lints(document: &DocumentMut) -> bool {
    LINT_TABLES.iter().any(|table_ref| {
        lint_root_table(document, table_ref.prefix).is_some_and(|table| !table.is_empty())
    })
}

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

fn root_package_member_branch(document: &DocumentMut) -> bool {
    document.get("workspace").and_then(Item::as_table).is_some()
        && document.get("package").and_then(Item::as_table).is_some()
}

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
