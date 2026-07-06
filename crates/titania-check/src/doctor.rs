//! Doctor report rendering for the `titania-check doctor` subcommand.

use titania_core::GateScope;
use titania_output::{
    OutputError,
    doctor::{DoctorReport, DoctorStatus, ToolRow},
};

use crate::{CliDisposition, args::EmitFormat};

/// Render a doctor report for CLI stdout.
///
/// # Errors
///
/// Returns [`OutputError`] if the doctor report producer detects a compiled
/// output-component misconfiguration.
pub(crate) fn render(scope: GateScope, emit: EmitFormat) -> Result<CliDisposition, OutputError> {
    let report = titania_output::doctor::doctor_report(scope)?;
    let stdout = match emit {
        EmitFormat::Human => render_human(&report),
        EmitFormat::Json => render_json(&report),
    };

    Ok(CliDisposition::report(stdout, report_code(report.status)))
}

const fn report_code(status: DoctorStatus) -> u8 {
    match status {
        DoctorStatus::Ok => 0,
        DoctorStatus::MissingRequiredTools => 3,
    }
}

/// Render a doctor report as the human-readable table.
#[must_use]
pub fn render_human(report: &DoctorReport) -> String {
    let rows = report.tools.iter().map(to_human_row).collect::<String>();
    format!(
        "titania-check doctor — scope: {scope}\n\n{header}{rows}\nStatus: {status}\n",
        scope = scope_name(report.scope),
        header = format_headers(),
        status = report.status.as_str(),
    )
}

fn format_headers() -> String {
    format!("{:<20} {:<10} {:<10} {:<22} {}\n", "Tool", "Required", "Installed", "Version", "Path")
}

fn to_human_row(tool: &ToolRow) -> String {
    let required = required_label(tool);
    let installed = installed_label(tool);
    let version = tool.version.as_deref().map_or("—", |version| version);
    let path = tool
        .path
        .as_ref()
        .map_or_else(|| "—".to_owned(), |path| path.to_string_lossy().into_owned());

    format!("{:<20} {:<10} {:<10} {:<22} {path}\n", tool.name, required, installed, version)
}

const fn required_label(tool: &ToolRow) -> &'static str {
    if tool.required { "yes" } else { "no" }
}

const fn installed_label(tool: &ToolRow) -> &'static str {
    if tool.installed { "yes" } else { "no" }
}

/// Render a doctor report as machine-readable JSON.
#[must_use]
pub fn render_json(report: &DoctorReport) -> String {
    let tools = report.tools.iter().map(to_json_row).collect::<Vec<_>>();
    let missing = report
        .missing_required
        .iter()
        .map(|name| serde_json::Value::String(name.clone()))
        .collect::<Vec<_>>();
    let root = serde_json::Map::from_iter([
        ("scope".to_owned(), serde_json::Value::String(scope_name(report.scope).to_owned())),
        ("tools".to_owned(), serde_json::Value::Array(tools)),
        ("missing_required".to_owned(), serde_json::Value::Array(missing)),
        ("status".to_owned(), serde_json::Value::String(report.status.as_str().to_owned())),
    ]);
    serde_json::Value::Object(root).to_string()
}

fn to_json_row(tool: &ToolRow) -> serde_json::Value {
    let version = tool
        .version
        .as_ref()
        .map_or(serde_json::Value::Null, |version| serde_json::Value::String(version.clone()));
    let path = tool.path.as_ref().map_or(serde_json::Value::Null, |path| {
        serde_json::Value::String(path.to_string_lossy().into_owned())
    });
    let row = serde_json::Map::from_iter([
        ("name".to_owned(), serde_json::Value::String(tool.name.to_owned())),
        ("required".to_owned(), serde_json::Value::Bool(tool.required)),
        ("installed".to_owned(), serde_json::Value::Bool(tool.installed)),
        ("version".to_owned(), version),
        ("path".to_owned(), path),
    ]);
    serde_json::Value::Object(row)
}

const fn scope_name(scope: GateScope) -> &'static str {
    match scope {
        GateScope::Edit => "edit",
        GateScope::Prepush => "prepush",
        GateScope::Release => "release",
        _ => "unknown",
    }
}
