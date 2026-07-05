//! Rule explanation rendering for the `explain` subcommand.

use titania_core::RuleId;
use titania_output::{
    OutputError,
    explain::{RuleExplanation, explain_rule},
};

/// Render a known rule explanation for CLI stdout.
///
/// # Errors
/// Returns a typed input diagnostic when the syntactically valid rule ID is not
/// present in the v1 rule catalog.
pub fn render(rule_id: &RuleId) -> Result<String, OutputError> {
    explain_rule(rule_id.as_str()).map(|entry| render_entry(&entry))
}

fn render_entry(entry: &RuleExplanation) -> String {
    format!(
        concat!(
            "{rule_id}\n",
            "  {description}\n\n",
            "  Pattern: {pattern}\n",
            "  Effect: {effect}\n",
            "  Repair: {repair}\n\n",
            "  Example violation:\n",
            "    {example_violation}\n\n",
            "  Example repair:\n",
            "    {example_repair}"
        ),
        rule_id = entry.rule_id.as_ref(),
        description = entry.description.as_ref(),
        pattern = entry.pattern.as_ref(),
        effect = entry.effect.as_ref(),
        repair = entry.repair.as_ref(),
        example_violation = entry.example_violation.as_ref(),
        example_repair = entry.example_repair.as_ref(),
    )
}
