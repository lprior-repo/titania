use std::{io, path::Path};

use titania_policy::{Exception, ExceptionError, parse_exceptions};

use crate::{Finding, LaneReport, RuleId, RuleIdError};

pub const EXCEPTIONS_PATH: &str = ".titania/profiles/strict-ai/exceptions.toml";

const RULE_READ_ERROR: &str = "POLICY_EXCEPTION_READ_ERROR";

pub fn load_exceptions(
    root: &Path,
    today: &str,
    report: &mut LaneReport,
) -> Result<Vec<Exception>, RuleIdError> {
    match std::fs::read_to_string(root.join(EXCEPTIONS_PATH)) {
        Ok(content) => parse_exception_content(&content, today, report),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => {
            report.push(read_error_finding(&error)?);
            Ok(Vec::new())
        }
    }
}

pub fn parse_exception_content(
    content: &str,
    today: &str,
    report: &mut LaneReport,
) -> Result<Vec<Exception>, RuleIdError> {
    match parse_exceptions(content, today) {
        Ok(exceptions) => Ok(exceptions),
        Err(error) => {
            report.push(exception_error_finding(&error)?);
            Ok(Vec::new())
        }
    }
}

#[must_use]
pub fn finding_is_excepted(finding: &Finding, exceptions: &[Exception]) -> bool {
    exceptions.iter().any(|exception| exception_matches(finding, exception))
}

fn exception_matches(finding: &Finding, exception: &Exception) -> bool {
    finding.rule().as_str() == exception.rule_id.as_str()
        && finding.path() == exception.path.as_str()
}

fn exception_error_finding(error: &ExceptionError) -> Result<Finding, RuleIdError> {
    Ok(Finding::new(RuleId::new(error.code())?, EXCEPTIONS_PATH, 0, exception_error_message(error)))
}

fn read_error_finding(error: &io::Error) -> Result<Finding, RuleIdError> {
    Ok(Finding::new(
        RuleId::new(RULE_READ_ERROR)?,
        EXCEPTIONS_PATH,
        0,
        format!("cannot read strict-ai exceptions file: {error}"),
    ))
}

fn exception_error_message(error: &ExceptionError) -> String {
    match error {
        ExceptionError::ParseError { message } => format!("exceptions.toml parse error: {message}"),
        ExceptionError::MissingField { field } => {
            format!("exceptions.toml missing required field {field}")
        }
        ExceptionError::InvalidField { field, message } => {
            format!("exceptions.toml invalid field {field}: {message}")
        }
        ExceptionError::ExceptionExpired { rule_id, expires_on, today } => {
            format!("exception for {rule_id} expired on {expires_on}; policy date is {today}")
        }
    }
}
