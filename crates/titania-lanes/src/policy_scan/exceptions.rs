//! Strict-ai policy exception application for policy-scan findings.
//!
//! Parsing and validation stay in `titania-policy`; this module only loads the
//! checked-in exception file, converts parser diagnostics to lane findings, and
//! filters exact rule/path matches from already-typed policy findings.

use std::{io, path::Path};

use titania_policy::{Exception, ExceptionError, parse_exceptions};

use crate::{Finding, LaneReport, RuleId, RuleIdError};

/// Workspace-relative strict-ai exception file path.
pub const EXCEPTIONS_PATH: &str = ".titania/profiles/strict-ai/exceptions.toml";

const RULE_READ_ERROR: &str = "POLICY_EXCEPTION_READ_ERROR";

/// Load checked strict-ai policy exceptions from the target project.
///
/// Missing exception files mean there are no active exceptions. Malformed or
/// expired files emit file-level policy findings and return an empty exception
/// set so no policy violation is accidentally suppressed.
///
/// # Errors
/// Returns [`RuleIdError`] if a diagnostic rule id from `titania-policy` is invalid.
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

/// Parse checked strict-ai policy exception TOML content.
///
/// This helper is public for deterministic behavior tests that should not touch
/// process-global time or the filesystem.
///
/// # Errors
/// Returns [`RuleIdError`] if an exception parser diagnostic code is not a valid rule id.
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

/// Return `true` when a finding has a matching, non-expired strict-ai exception.
#[must_use]
pub fn finding_is_excepted(finding: &Finding, exceptions: &[Exception]) -> bool {
    exceptions.iter().any(|exception| exception_matches(finding, exception))
}

fn exception_matches(finding: &Finding, exception: &Exception) -> bool {
    finding.rule().as_str() == exception.rule_id.as_str()
        && finding.path() == exception.path.as_str()
}

/// Convert a policy exception parser error into a file-level lane finding.
///
/// # Errors
/// Returns [`RuleIdError`] if the parser diagnostic code is not a valid rule id.
fn exception_error_finding(error: &ExceptionError) -> Result<Finding, RuleIdError> {
    Ok(Finding::new(RuleId::new(error.code())?, EXCEPTIONS_PATH, 0, exception_error_message(error)))
}

/// Convert an exception-file read failure into a file-level lane finding.
///
/// # Errors
/// Returns [`RuleIdError`] if the read-error diagnostic code is not a valid rule id.
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
