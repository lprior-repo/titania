//! Report output: JSON schema serialization and formatting.

use xtask_core::Report;

/// Serialize a report to pretty JSON.
///
/// # Errors
/// Returns an error if the report cannot be serialized to JSON.
pub fn to_json(report: &Report) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report)
}
