//! Rule explanation catalog for `titania-check explain`.

use std::borrow::Cow;

use crate::OutputError;

const CATALOG: &str = include_str!("../rules/explain.tsv");

/// A single catalog entry for a known rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleExplanation {
    /// Stable v1 rule identifier.
    pub rule_id: Cow<'static, str>,
    /// Short human explanation of the violation.
    pub description: Cow<'static, str>,
    /// Source lane or normalizer that emits the rule.
    pub source: Cow<'static, str>,
    /// Detection pattern or trigger.
    pub pattern: Cow<'static, str>,
    /// Gate effect.
    pub effect: Cow<'static, str>,
    /// Suggested repair class.
    pub repair: Cow<'static, str>,
    /// Minimal violating source sample.
    pub example_violation: Cow<'static, str>,
    /// Minimal repaired source sample.
    pub example_repair: Cow<'static, str>,
}

#[derive(Debug, Clone, Copy)]
struct Row {
    rule_id: &'static str,
    source: &'static str,
    effect: &'static str,
    repair: &'static str,
    pattern: &'static str,
    description: &'static str,
}

/// Return a rule explanation catalog entry.
///
/// # Errors
/// Returns [`OutputError::UnknownRule`] when `rule_id` is syntactically valid
/// but absent from the finite catalog and not a dynamic `CLIPPY_*` rule.
pub fn explain_rule(rule_id: &str) -> Result<RuleExplanation, OutputError> {
    CATALOG
        .lines()
        .filter_map(parse_row)
        .find(|row| row.rule_id == rule_id)
        .map(row_entry)
        .or_else(|| dynamic_clippy_rule(rule_id))
        .ok_or_else(|| OutputError::unknown_rule(rule_id))
}

fn parse_row(line: &'static str) -> Option<Row> {
    let mut fields = line.split('\t');
    let row = Row {
        rule_id: fields.next()?,
        source: fields.next()?,
        effect: fields.next()?,
        repair: fields.next()?,
        pattern: fields.next()?,
        description: fields.next()?,
    };
    fields.next().is_none().then_some(row)
}

fn row_entry(row: Row) -> RuleExplanation {
    RuleExplanation {
        rule_id: Cow::Borrowed(row.rule_id),
        source: Cow::Borrowed(row.source),
        effect: Cow::Borrowed(row.effect),
        repair: Cow::Borrowed(row.repair),
        pattern: Cow::Borrowed(row.pattern),
        description: Cow::Borrowed(row.description),
        example_violation: example_violation(row.rule_id, row.pattern),
        example_repair: example_repair(row.rule_id, row.repair),
    }
}

fn example_violation(rule_id: &str, pattern: &str) -> Cow<'static, str> {
    match rule_id {
        "FUNC_LOOPS_FOR" => Cow::Borrowed("for item in items { process(item); }"),
        _ => Cow::Owned(format!("code matching `{pattern}`")),
    }
}

fn example_repair(rule_id: &str, repair: &str) -> Cow<'static, str> {
    match rule_id {
        "FUNC_LOOPS_FOR" => Cow::Borrowed("items.iter().for_each(|item| process(item));"),
        _ => Cow::Owned(format!("apply `{repair}`")),
    }
}

fn dynamic_clippy_rule(rule_id: &str) -> Option<RuleExplanation> {
    let lint = rule_id.strip_prefix("CLIPPY_").filter(|lint| !lint.is_empty())?;
    let pattern = format!("clippy::{}", lint.to_ascii_lowercase());
    Some(RuleExplanation {
        rule_id: Cow::Owned(rule_id.to_owned()),
        description: Cow::Owned(format!("Explains reportable Clippy diagnostic `{pattern}`.")),
        source: Cow::Borrowed("clippy"),
        pattern: Cow::Owned(pattern.clone()),
        effect: Cow::Borrowed("Reject"),
        repair: Cow::Borrowed("RequiresHumanReview"),
        example_violation: Cow::Owned(format!("code triggering `{pattern}`")),
        example_repair: Cow::Borrowed("change the code so Clippy no longer emits the lint"),
    })
}
