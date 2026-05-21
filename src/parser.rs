use std::ffi::OsString;

use crate::command_family::CommandFamily;
use crate::error::XtaskCommandError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XtaskCommand {
    Required(CommandFamily),
    Legacy(&'static str),
    Help,
    Version,
}

struct ParsedCommandName<'a>(&'a str);

pub fn parse_xtask_command(
    args: impl IntoIterator<Item = OsString>,
) -> Result<XtaskCommand, XtaskCommandError> {
    let tokens = collect_args(args);
    let command = top_level_command(&tokens)?;
    classify_top_level_command(command, &tokens)
}

fn collect_args(args: impl IntoIterator<Item = OsString>) -> Vec<String> {
    args.into_iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect()
}

fn top_level_command(tokens: &[String]) -> Result<ParsedCommandName<'_>, XtaskCommandError> {
    tokens
        .get(1)
        .map(String::as_str)
        .map(ParsedCommandName)
        .ok_or_else(|| XtaskCommandError::MissingRequiredInput {
            command: "xtask".to_string(),
            input: "command".to_string(),
        })
}

fn classify_top_level_command(
    command: ParsedCommandName<'_>,
    tokens: &[String],
) -> Result<XtaskCommand, XtaskCommandError> {
    let command = command.0;
    if command == "--help" || command == "-h" {
        return Ok(XtaskCommand::Help);
    }
    if command == "--version" || command == "-V" {
        return Ok(XtaskCommand::Version);
    }
    if let Some(legacy) = parse_legacy(command) {
        return Ok(XtaskCommand::Legacy(legacy));
    }
    parse_required_command(command, tokens)
}

fn parse_required_command(
    command: &str,
    tokens: &[String],
) -> Result<XtaskCommand, XtaskCommandError> {
    let Some(family) = CommandFamily::parse(command) else {
        return Err(XtaskCommandError::UnknownCommand {
            command: command.to_string(),
        });
    };
    validate_required_options(command, tokens)?;
    Ok(XtaskCommand::Required(family))
}

fn parse_legacy(command: &str) -> Option<&'static str> {
    match command {
        "ui-snapshot" => Some("ui-snapshot"),
        "ui-tokens" => Some("ui-tokens"),
        "ui-overlap-check" => Some("ui-overlap-check"),
        "ai-fast" => Some("ai-fast"),
        "ai-deep" => Some("ai-deep"),
        "ai-release" => Some("ai-release"),
        "forbidden-scan" => Some("forbidden-scan"),
        "proof-plan" => Some("proof-plan"),
        "proof-check" => Some("proof-check"),
        "proof-evidence" => Some("proof-evidence"),
        "proof-drift" => Some("proof-drift"),
        "loom" => Some("loom"),
        "list-crates" => Some("list-crates"),
        "proof" => Some("proof"),
        "contracts" => Some("contracts"),
        _ => None,
    }
}

fn validate_required_options(command: &str, tokens: &[String]) -> Result<(), XtaskCommandError> {
    validate_bead_option(command, tokens)?;
    validate_format_option(command, tokens)
}

fn validate_bead_option(command: &str, tokens: &[String]) -> Result<(), XtaskCommandError> {
    let mut iter = tokens.iter();
    while let Some(token) = iter.next() {
        if token == "--bead" {
            let Some(value) = iter.next() else {
                return Err(XtaskCommandError::MissingRequiredInput {
                    command: command.to_string(),
                    input: "bead".to_string(),
                });
            };
            if value.is_empty() {
                return Err(XtaskCommandError::InvalidInput {
                    command: command.to_string(),
                    input: "bead".to_string(),
                    reason: "bead id must not be empty".to_string(),
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contracts_remains_routed_to_legacy_cli() {
        let parsed = parse_xtask_command(["xtask".into(), "contracts".into(), "--check".into()]);

        assert_eq!(parsed, Ok(XtaskCommand::Legacy("contracts")));
    }

    #[test]
    fn unknown_top_level_command_still_fails_closed() {
        let parsed = parse_xtask_command(["xtask".into(), "not-a-command".into()]);

        assert_eq!(
            parsed,
            Err(XtaskCommandError::UnknownCommand {
                command: "not-a-command".to_string()
            })
        );
    }
}

fn validate_format_option(command: &str, tokens: &[String]) -> Result<(), XtaskCommandError> {
    let mut iter = tokens.iter();
    while let Some(token) = iter.next() {
        if token == "--format" {
            let Some(value) = iter.next() else {
                return Err(XtaskCommandError::MissingRequiredInput {
                    command: command.to_string(),
                    input: "format".to_string(),
                });
            };
            if value != "jsonl" {
                return Err(XtaskCommandError::InvalidInput {
                    command: command.to_string(),
                    input: "format".to_string(),
                    reason: format!("unsupported output format: {value}"),
                });
            }
        }
    }
    Ok(())
}
