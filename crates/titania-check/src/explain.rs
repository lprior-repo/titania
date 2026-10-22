use titania_output::{
    OutputError,
    explain::{RuleExplanation, explain_rule},
};

pub fn render(rule_id: &str) -> Result<String, OutputError> {
    explain_rule(rule_id).map(|entry| render_entry(&entry))
}

fn render_entry(entry: &RuleExplanation) -> String {
    let repair_block = entry
        .repair
        .as_deref()
        .map_or_else(String::new, |repair| format!("  Repair: {repair}{}\n", entry.repair_note));
    let example_repair_block =
        entry.example_repair.as_deref().map_or_else(String::new, |example_repair| {
            format!("\n\n  Example repair:\n    {example_repair}")
        });
    format!(
        concat!(
            "{rule_id}\n",
            "  {description}\n\n",
            "  Pattern: {pattern}\n",
            "  Effect: {effect}\n",
            "{repair_block}",
            "\n  Example violation:\n",
            "    {example_violation}",
            "{example_repair_block}"
        ),
        rule_id = entry.rule_id.as_ref(),
        description = entry.description.as_ref(),
        pattern = entry.pattern.as_ref(),
        effect = entry.effect.as_ref(),
        repair_block = repair_block,
        example_violation = entry.example_violation.as_ref(),
        example_repair_block = example_repair_block,
    )
}
