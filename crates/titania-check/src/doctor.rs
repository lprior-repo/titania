use titania_core::GateScope;
use titania_output::{
    OutputError,
    doctor::{DoctorReport, DoctorStatus, ToolRow},
};

use crate::{CliDisposition, args::EmitFormat};

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

#[must_use]
pub fn render_human(report: &DoctorReport) -> String {
    let rows = report.tools.iter().map(|tool| to_human_row(report.scope, tool)).collect::<String>();
    format!(
        "titania-check doctor — scope: {scope}\n\n{header}{rows}\nStatus: {status}\n",
        scope = report.scope.name(),
        header = format_headers(),
        status = report.status.as_str(),
    )
}

fn format_headers() -> String {
    format!("{:<20} {:<10} {:<10} {:<22} {}\n", "Tool", "Required", "Installed", "Version", "Path")
}

fn to_human_row(scope: GateScope, tool: &ToolRow) -> String {
    let required = required_label(scope, tool);
    let installed = installed_label(tool);
    let version = tool.version.as_deref().map_or("—", |version| version);
    let path = tool
        .path
        .as_ref()
        .map_or_else(|| "—".to_owned(), |path| path.to_string_lossy().into_owned());

    format!(
        "{:<20} {:<10} {:<10} {:<22} {path}\n",
        human_label(tool.name),
        required,
        installed,
        version
    )
}

fn required_label(scope: GateScope, tool: &ToolRow) -> &'static str {
    if tool.name == "ast-grep" {
        return "embedded";
    }
    if tool.name == "sccache" {
        return "optional";
    }
    if tool.name == "cargo-deny" && matches!(scope, GateScope::Edit) {
        return "no (edit)";
    }
    if tool.required { "yes" } else { "no" }
}

fn installed_label(tool: &ToolRow) -> &'static str {
    if tool.name == "ast-grep" {
        "—"
    } else if tool.installed {
        "yes"
    } else {
        "no"
    }
}

fn human_label(name: &'static str) -> &'static str {
    display_name(name)
}

fn display_name(name: &str) -> &str {
    match name {
        "clippy-driver" => "clippy",
        "cargo-dylint" => "dylint",
        "libtitania_dylint" => "dylint-lib",
        _ => name,
    }
}

#[must_use]
pub fn render_json(report: &DoctorReport) -> String {
    let tools = report.tools.iter().map(to_json_row).collect::<Vec<_>>();
    let missing = report
        .missing_required
        .iter()
        .map(|name| serde_json::Value::String(display_name(name).to_owned()))
        .collect::<Vec<_>>();
    let root = serde_json::Map::from_iter([
        ("scope".to_owned(), serde_json::Value::String(report.scope.name().to_owned())),
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
        ("name".to_owned(), serde_json::Value::String(display_name(tool.name).to_owned())),
        ("required".to_owned(), serde_json::Value::Bool(tool.required)),
        ("installed".to_owned(), serde_json::Value::Bool(tool.installed)),
        ("version".to_owned(), version),
        ("path".to_owned(), path),
    ]);
    serde_json::Value::Object(row)
}
