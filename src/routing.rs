use std::path::PathBuf;

use crate::command_family::CommandFamily;
use crate::error::XtaskCommandError;
use crate::parser::XtaskCommand;
use crate::status::{DeferredReason, OutputFormat, StructuredStatus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XtaskEnvironment {
    pub workspace_root: PathBuf,
    pub bead_id: Option<String>,
    pub output_format: OutputFormat,
    pub unavailable_families: Vec<CommandFamily>,
}

pub fn route_command(
    command: XtaskCommand,
    env: &XtaskEnvironment,
) -> Result<StructuredStatus, XtaskCommandError> {
    match command {
        XtaskCommand::Required(family) => route_required_command(family, env),
        XtaskCommand::Legacy(name) => unavailable_legacy(name),
        XtaskCommand::Help => unavailable_shell_request("help", "help is rendered by clap"),
        XtaskCommand::Version => {
            unavailable_shell_request("version", "version is rendered by clap")
        }
    }
}

fn route_required_command(
    family: CommandFamily,
    env: &XtaskEnvironment,
) -> Result<StructuredStatus, XtaskCommandError> {
    if is_family_unavailable(family, env) {
        unavailable_required(family)
    } else {
        placeholder_status(family, DeferredReason::NotImplementedInThisBead)
    }
}

fn is_family_unavailable(family: CommandFamily, env: &XtaskEnvironment) -> bool {
    env.unavailable_families
        .iter()
        .any(|entry| entry == &family)
}

fn unavailable_required(family: CommandFamily) -> Result<StructuredStatus, XtaskCommandError> {
    let name = family.public_name();
    Err(XtaskCommandError::Unavailable {
        command: name.to_string(),
        reason: format!("{name} automation is not implemented in bead vb-kkvb"),
    })
}

fn unavailable_legacy(name: &str) -> Result<StructuredStatus, XtaskCommandError> {
    unavailable_shell_request(
        name,
        &format!("{name} legacy routing is handled by the xtask binary"),
    )
}

fn unavailable_shell_request(
    command: &str,
    reason: &str,
) -> Result<StructuredStatus, XtaskCommandError> {
    Err(XtaskCommandError::Unavailable {
        command: command.to_string(),
        reason: reason.to_string(),
    })
}

pub fn placeholder_status(
    command: CommandFamily,
    _reason: DeferredReason,
) -> Result<StructuredStatus, XtaskCommandError> {
    let name = command.public_name();
    Ok(StructuredStatus {
        command: name.to_string(),
        status: "deferred".to_string(),
        message: format!("{name} automation deferred: implementation is outside bead vb-kkvb"),
        next_steps: vec![format!("open follow-up bead for {name} engine integration")],
    })
}
