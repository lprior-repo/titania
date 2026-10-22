use std::borrow::Cow;

use crate::OutputError;

const CATALOG: &str = include_str!("../../titania-core/src/finding/repair_catalog.tsv");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleExplanation {
    pub rule_id: Cow<'static, str>,
    pub description: Cow<'static, str>,
    pub source: Cow<'static, str>,
    pub pattern: Cow<'static, str>,
    pub effect: Cow<'static, str>,
    pub repair: Option<Cow<'static, str>>,
    pub repair_note: Cow<'static, str>,
    pub example_violation: Cow<'static, str>,
    pub example_repair: Option<Cow<'static, str>>,
}

#[derive(Debug, Clone, Copy)]
struct Row {
    rule_id: &'static str,
    source: &'static str,
    effect: &'static str,
    pattern: &'static str,
    description: &'static str,
}

pub fn explain_rule(rule_id: &str) -> Result<RuleExplanation, OutputError> {
    CATALOG
        .lines()
        .filter_map(parse_row)
        .find(|row| row.rule_id == rule_id)
        .map(row_entry)
        .or_else(|| dynamic_clippy_rule(rule_id))
        .or_else(|| dynamic_proof_kani_rule(rule_id))
        .or_else(|| dynamic_mutant_rule(rule_id))
        .ok_or_else(|| OutputError::unknown_rule(rule_id))
}

fn parse_row(line: &'static str) -> Option<Row> {
    let mut fields = line.split('\t');
    let rule_id = fields.next()?;
    let source = fields.next()?;
    let effect = fields.next()?;
    let _repair_class = fields.next()?;
    let row = Row { rule_id, source, effect, pattern: fields.next()?, description: fields.next()? };
    fields.next().is_none().then_some(row)
}

fn row_entry(row: Row) -> RuleExplanation {
    let repair = match titania_core::RepairHint::for_rule(row.rule_id).class() {
        titania_core::RepairHintClass::None => None,
        class => Some(Cow::Owned(class.as_str().to_owned())),
    };
    let example_repair = example_repair(row.rule_id, repair.as_deref());
    RuleExplanation {
        rule_id: Cow::Borrowed(row.rule_id),
        source: Cow::Borrowed(row.source),
        effect: Cow::Borrowed(row.effect),
        repair,
        repair_note: repair_note(row.rule_id),
        pattern: Cow::Borrowed(row.pattern),
        description: Cow::Borrowed(row.description),
        example_violation: example_violation(row.rule_id, row.pattern),
        example_repair,
    }
}

fn repair_note(rule_id: &str) -> Cow<'static, str> {
    match rule_id {
        "BOUNDED_EXECUTION_UNBOUNDED_LOOP" => {
            Cow::Borrowed(" — cite Bound::FiniteInput, Bound::Capped, or Bound::ProtocolTerminal")
        }
        _ => Cow::Borrowed(""),
    }
}

fn example_violation(rule_id: &str, pattern: &str) -> Cow<'static, str> {
    match rule_id {
        "BOUNDED_EXECUTION_UNBOUNDED_LOOP" => Cow::Borrowed("for item in items { process(item); }"),
        "BOUNDED_EXECUTION_UNBOUNDED_ITERATOR" => {
            Cow::Borrowed("std::iter::repeat(work).map(|job| run(job))")
        }
        _ => Cow::Owned(format!("code matching `{pattern}`")),
    }
}

fn example_repair(rule_id: &str, repair: Option<&str>) -> Option<Cow<'static, str>> {
    match rule_id {
        "BOUNDED_EXECUTION_UNBOUNDED_LOOP" => {
            Some(Cow::Borrowed("items.iter().for_each(|item| process(item));"))
        }
        "BOUNDED_EXECUTION_UNBOUNDED_ITERATOR" => Some(Cow::Borrowed(
            "std::iter::repeat(work).take(max).map(|job| run(job)) // Bound::Capped + cited test",
        )),
        _ => repair.map(|repair| Cow::Owned(format!("apply `{repair}`"))),
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
        repair: Some(Cow::Borrowed("RequiresHumanReview")),
        repair_note: Cow::Borrowed(""),
        example_violation: Cow::Owned(format!("code triggering `{pattern}`")),
        example_repair: Some(Cow::Borrowed("change the code so Clippy no longer emits the lint")),
    })
}

fn dynamic_proof_kani_rule(rule_id: &str) -> Option<RuleExplanation> {
    let suffix = rule_id.strip_prefix("PROOF_KANI_").filter(|s| !s.is_empty())?;
    let (effect, description) = match suffix {
        "PASS" => ("Informational", "Per-harness Kani verification success."),
        "FAIL" => ("Reject", "Per-harness Kani verification failure."),
        "BLOCKED" => ("Reject", "Kani harness blocked by cgroup OOM or wallclock timeout."),
        "NOT_RUN" => ("Informational", "Kani lane skipped because cargo-kani is missing."),
        "UNSUPPORTED" => {
            ("Informational", "Kani harness disabled by an unsupported-feature warning.")
        }
        "INFRA" => ("Reject", "Kani infrastructure failure (spawn, parse, or tool version drift)."),
        _ => {
            let lower = suffix.to_ascii_lowercase();
            let pattern = format!("kani::{lower}");
            return Some(RuleExplanation {
                rule_id: Cow::Owned(rule_id.to_owned()),
                description: Cow::Owned(format!(
                    "Per-harness Kani verification verdict for `{lower}`."
                )),
                source: Cow::Borrowed("kani"),
                pattern: Cow::Owned(pattern),
                effect: Cow::Borrowed("Reject"),
                repair: Some(Cow::Borrowed("RequiresHumanReview")),
                repair_note: Cow::Borrowed(""),
                example_violation: Cow::Owned(format!(
                    "Kani harness `{lower}` produced a failing verification verdict."
                )),
                example_repair: Some(Cow::Borrowed(
                    "add a regression test or fix the harness contract so the harness verifies",
                )),
            });
        }
    };
    let pattern = format!("kani::{}", suffix.to_ascii_lowercase());
    Some(RuleExplanation {
        rule_id: Cow::Owned(rule_id.to_owned()),
        description: Cow::Owned(description.to_owned()),
        source: Cow::Borrowed("kani"),
        pattern: Cow::Owned(pattern),
        effect: Cow::Borrowed(effect),
        repair: Some(Cow::Borrowed("RequiresHumanReview")),
        repair_note: Cow::Borrowed(""),
        example_violation: Cow::Owned(format!(
            "Kani harness `{suffix}` produced a non-SUCCESSFUL verdict."
        )),
        example_repair: Some(Cow::Borrowed(
            "add a regression test or fix the harness contract so the harness verifies",
        )),
    })
}

fn dynamic_mutant_rule(rule_id: &str) -> Option<RuleExplanation> {
    if !rule_id.starts_with("MUTANT_") {
        return None;
    }
    if let Some(suffix) = rule_id.strip_prefix("MUTANT_SURVIVED_").filter(|s| !s.is_empty()) {
        let pattern = format!("cargo-mutants survivor {suffix}");
        return Some(RuleExplanation {
            rule_id: Cow::Owned(rule_id.to_owned()),
            description: Cow::Owned(format!(
                "cargo-mutants survivor `{suffix}` not covered by the mutants baseline."
            )),
            source: Cow::Borrowed("mutants"),
            pattern: Cow::Owned(pattern),
            effect: Cow::Borrowed("Reject"),
            repair: Some(Cow::Borrowed("RequiresHumanReview")),
            repair_note: Cow::Borrowed(""),
            example_violation: Cow::Owned(format!(
                "mutation `{suffix}` survives cargo mutants without a baseline bypass"
            )),
            example_repair: Some(Cow::Borrowed(
                "add a regression test that kills the mutant, or add a mutant-accept/<owner>/<reason>/<expiry> entry to .titania/profiles/strict-ai/mutants.baseline.json",
            )),
        });
    }
    let (effect, description) = match rule_id {
        "MUTANT_SURVIVED" => {
            ("Reject", "cargo-mutants survivor not covered by the mutants baseline.")
        }
        "MUTANT_SURVIVED_INFRA" => (
            "Reject",
            "cargo-mutants infrastructure failure (spawn, list parse, or version drift).",
        ),
        "MUTANT_BASELINE_MISSING" => {
            ("Reject", "mutants baseline is missing; run scripts/dev/mutants-bootstrap.sh.")
        }
        _ => return None,
    };
    Some(RuleExplanation {
        rule_id: Cow::Owned(rule_id.to_owned()),
        description: Cow::Owned(description.to_owned()),
        source: Cow::Borrowed("mutants"),
        pattern: Cow::Owned(rule_id.to_owned()),
        effect: Cow::Borrowed(effect),
        repair: Some(Cow::Borrowed("RequiresHumanReview")),
        repair_note: Cow::Borrowed(""),
        example_violation: Cow::Owned(format!("mutant event {rule_id} observed")),
        example_repair: Some(Cow::Borrowed(
            "add a regression test that kills the mutant, or bootstrap a baseline entry with owner/reason/expiry",
        )),
    })
}
