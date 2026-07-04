//! Parser and validator for `.titania/profiles/strict-ai/exceptions.toml`.

use serde::Deserialize;

/// A checked policy exception entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exception {
    /// Rule identifier being excepted.
    pub rule_id: String,
    /// Workspace-relative path constrained by the exception.
    pub path: String,
    /// Team or owner accountable for the exception.
    pub owner: String,
    /// Human-readable justification.
    pub reason: String,
    /// ISO-8601 date (`YYYY-MM-DD`) when the exception expires.
    pub expires_on: String,
    /// Review ticket or approval identifier.
    pub review: String,
}

#[derive(Debug, Deserialize)]
struct ExceptionsDocument {
    #[serde(default)]
    exceptions: Vec<ExceptionWire>,
}

#[derive(Debug, Deserialize)]
struct ExceptionWire {
    rule_id: Option<String>,
    path: Option<String>,
    owner: Option<String>,
    reason: Option<String>,
    expires_on: Option<String>,
    review: Option<String>,
}

/// Errors returned while parsing or validating `exceptions.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExceptionError {
    /// TOML parsing failed.
    ParseError {
        /// Human-readable parser message.
        message: Box<str>,
    },
    /// A required field is absent or empty.
    MissingField {
        /// Missing field name.
        field: &'static str,
    },
    /// The exception's expiry is older than the supplied policy date.
    ExceptionExpired {
        /// Rule identifier from the expired exception.
        rule_id: Box<str>,
        /// Expiry date from the exception.
        expires_on: Box<str>,
        /// Policy evaluation date.
        today: Box<str>,
    },
}

impl ExceptionError {
    /// Return the stable diagnostic code for this error.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::ExceptionExpired { .. } => "POLICY_EXCEPTION_EXPIRED",
            Self::ParseError { .. } => "POLICY_EXCEPTION_PARSE_ERROR",
            Self::MissingField { .. } => "POLICY_EXCEPTION_MISSING_FIELD",
        }
    }
}

/// Parse and validate `.titania/profiles/strict-ai/exceptions.toml` content.
///
/// `today` must be an ISO date (`YYYY-MM-DD`) supplied by the caller's clock
/// boundary. Keeping the parser pure avoids hidden time access in policy core.
///
/// # Errors
/// Returns [`ExceptionError::ParseError`] for malformed TOML,
/// [`ExceptionError::MissingField`] for missing/empty required fields, and
/// [`ExceptionError::ExceptionExpired`] when `expires_on < today`.
pub fn parse_exceptions(content: &str, today: &str) -> Result<Vec<Exception>, ExceptionError> {
    toml::from_str::<ExceptionsDocument>(content)
        .map_err(|error| ExceptionError::ParseError {
            message: error.to_string().into_boxed_str(),
        })?
        .exceptions
        .into_iter()
        .map(|wire| validate_exception(wire, today))
        .collect()
}

/// Validate one decoded exception entry.
///
/// # Errors
/// Returns [`ExceptionError::MissingField`] when a required field is absent
/// or empty, and [`ExceptionError::ExceptionExpired`] when the exception is
/// stale for the supplied policy date.
fn validate_exception(wire: ExceptionWire, today: &str) -> Result<Exception, ExceptionError> {
    let rule_id = required(wire.rule_id, "rule_id")?;
    let path = required(wire.path, "path")?;
    let owner = required(wire.owner, "owner")?;
    let reason = required(wire.reason, "reason")?;
    let expires_on = required(wire.expires_on, "expires_on")?;
    let review = required(wire.review, "review")?;
    if expires_on.as_str() < today {
        return Err(ExceptionError::ExceptionExpired {
            rule_id: rule_id.into_boxed_str(),
            expires_on: expires_on.into_boxed_str(),
            today: today.to_owned().into_boxed_str(),
        });
    }
    Ok(Exception { rule_id, path, owner, reason, expires_on, review })
}

/// Extract a required non-empty string field.
///
/// # Errors
/// Returns [`ExceptionError::MissingField`] when the value is absent or blank.
fn required(value: Option<String>, field: &'static str) -> Result<String, ExceptionError> {
    value
        .filter(|candidate| !candidate.trim().is_empty())
        .ok_or(ExceptionError::MissingField { field })
}
