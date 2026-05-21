use crate::error::XtaskCommandError;

pub(crate) const STATUS_FIELDS: [&str; 4] = ["command", "status", "message", "next_steps"];
const RENDER_FAILURE_SEPARATOR: char = '\0';

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    JsonLines,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeferredReason {
    NotImplementedInThisBead,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredStatus {
    pub command: String,
    pub status: String,
    pub message: String,
    pub next_steps: Vec<String>,
}

impl StructuredStatus {
    #[must_use]
    pub fn with_renderer_failure_for_test<const N: usize>(
        command: &str,
        status: &str,
        message: &str,
        next_steps: [&str; N],
        reason: &str,
    ) -> Self {
        let command_with_failure = format!("{command}{RENDER_FAILURE_SEPARATOR}{reason}");
        Self {
            command: command_with_failure,
            status: status.to_string(),
            message: message.to_string(),
            next_steps: next_steps.into_iter().map(str::to_string).collect(),
        }
    }
}

pub fn render_structured_status(
    status: &StructuredStatus,
    format: OutputFormat,
) -> Result<String, XtaskCommandError> {
    let command = rendered_command(status)?;
    match format {
        OutputFormat::JsonLines => render_json_line(command, status),
    }
}

fn rendered_command(status: &StructuredStatus) -> Result<&str, XtaskCommandError> {
    if let Some((command, reason)) = status.command.split_once(RENDER_FAILURE_SEPARATOR) {
        Err(XtaskCommandError::OutputRenderFailed {
            command: command.to_string(),
            reason: reason.to_string(),
        })
    } else {
        Ok(status.command.as_str())
    }
}

fn render_json_line(command: &str, status: &StructuredStatus) -> Result<String, XtaskCommandError> {
    validate_renderable_status(command, status)?;
    let command_text = json_text(command, command)?;
    let status_text = json_text(command, &status.status)?;
    let message_text = json_text(command, &status.message)?;
    let next_steps = json_text(command, &status.next_steps)?;
    Ok(format!(
        "{{\"command\":{command_text},\"status\":{status_text},\"message\":{message_text},\"next_steps\":{next_steps}}}\n"
    ))
}

fn validate_renderable_status(
    command: &str,
    status: &StructuredStatus,
) -> Result<(), XtaskCommandError> {
    if status.next_steps.is_empty() || status.message.is_empty() {
        Err(XtaskCommandError::OutputRenderFailed {
            command: command.to_string(),
            reason: "structured status fields must be non-empty".to_string(),
        })
    } else {
        Ok(())
    }
}

fn json_text<T: serde::Serialize + ?Sized>(
    command: &str,
    value: &T,
) -> Result<String, XtaskCommandError> {
    serde_json::to_string(value).map_err(|error| XtaskCommandError::OutputRenderFailed {
        command: command.to_string(),
        reason: error.to_string(),
    })
}
