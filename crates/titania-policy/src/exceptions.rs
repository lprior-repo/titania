use serde::Deserialize;
use titania_core::{RuleId, WorkspacePath};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exception {
    pub rule_id: RuleId,
    pub path: WorkspacePath,
    pub owner: Box<str>,
    pub reason: Box<str>,
    pub expires_on: Box<str>,
    pub review: Box<str>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExceptionsDocument {
    #[serde(default)]
    exceptions: Vec<ExceptionWire>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExceptionWire {
    rule_id: Option<String>,
    path: Option<String>,
    owner: Option<String>,
    reason: Option<String>,
    expires_on: Option<String>,
    review: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct PolicyDate {
    year: u16,
    month: u8,
    day: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExceptionError {
    ParseError { message: Box<str> },
    MissingField { field: &'static str },
    InvalidField { field: &'static str, message: Box<str> },
    ExceptionExpired { rule_id: Box<str>, expires_on: Box<str>, today: Box<str> },
}

impl ExceptionError {
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::ExceptionExpired { .. } => "POLICY_EXCEPTION_EXPIRED",
            Self::InvalidField { .. } => "POLICY_EXCEPTION_INVALID_FIELD",
            Self::ParseError { .. } => "POLICY_EXCEPTION_PARSE_ERROR",
            Self::MissingField { .. } => "POLICY_EXCEPTION_MISSING_FIELD",
        }
    }
}

pub fn parse_exceptions(content: &str, today: &str) -> Result<Vec<Exception>, ExceptionError> {
    let today_date = parse_date(today, "today")?;
    toml_edit::de::from_str::<ExceptionsDocument>(content)
        .map_err(|error| ExceptionError::ParseError {
            message: error.to_string().into_boxed_str(),
        })?
        .exceptions
        .into_iter()
        .map(|wire| validate_exception(wire, today, today_date))
        .collect()
}

fn validate_exception(
    wire: ExceptionWire,
    today_text: &str,
    today: PolicyDate,
) -> Result<Exception, ExceptionError> {
    let rule_id_text = required(wire.rule_id, "rule_id")?;
    let path_text = required(wire.path, "path")?;
    let owner = required_boxed(wire.owner, "owner")?;
    let reason = required_boxed(wire.reason, "reason")?;
    let expires_on = required_boxed(wire.expires_on, "expires_on")?;
    let review = required_boxed(wire.review, "review")?;
    let rule_id = validated_rule_id(&rule_id_text)?;
    let path = validated_path(&path_text)?;
    let expires_on_date = parse_date(&expires_on, "expires_on")?;
    reject_expired(&rule_id, &expires_on, expires_on_date, today_text, today)?;
    Ok(Exception { rule_id, path, owner, reason, expires_on, review })
}

fn validated_rule_id(value: &str) -> Result<RuleId, ExceptionError> {
    RuleId::new(value).map_err(|error| ExceptionError::InvalidField {
        field: "rule_id",
        message: error.to_string().into_boxed_str(),
    })
}

fn validated_path(value: &str) -> Result<WorkspacePath, ExceptionError> {
    WorkspacePath::new(value).map_err(|error| ExceptionError::InvalidField {
        field: "path",
        message: error.to_string().into_boxed_str(),
    })
}

fn reject_expired(
    rule_id: &RuleId,
    expires_on_text: &str,
    expires_on: PolicyDate,
    today_text: &str,
    today: PolicyDate,
) -> Result<(), ExceptionError> {
    if expires_on >= today {
        return Ok(());
    }
    Err(ExceptionError::ExceptionExpired {
        rule_id: rule_id.as_str().into(),
        expires_on: expires_on_text.into(),
        today: today_text.into(),
    })
}

fn required(value: Option<String>, field: &'static str) -> Result<String, ExceptionError> {
    value
        .filter(|candidate| !candidate.trim().is_empty())
        .ok_or(ExceptionError::MissingField { field })
}

fn required_boxed(value: Option<String>, field: &'static str) -> Result<Box<str>, ExceptionError> {
    required(value, field).map(String::into_boxed_str)
}

fn parse_date(value: &str, field: &'static str) -> Result<PolicyDate, ExceptionError> {
    let (year_text, month_text, day_text) = split_date(value, field)?;
    require_date_widths(value, year_text, month_text, day_text, field)?;
    require_digits(year_text, field)?;
    require_digits(month_text, field)?;
    require_digits(day_text, field)?;
    let date = PolicyDate {
        year: parse_date_part(year_text, field)?,
        month: parse_date_part(month_text, field)?,
        day: parse_date_part(day_text, field)?,
    };
    validate_date_range(date, field)?;
    Ok(date)
}

type DateParts<'a> = (&'a str, &'a str, &'a str);

fn split_date<'a>(value: &'a str, field: &'static str) -> Result<DateParts<'a>, ExceptionError> {
    let mut parts = value.split('-');
    let year = parts.next().ok_or_else(|| invalid_date(field))?;
    let month = parts.next().ok_or_else(|| invalid_date(field))?;
    let day = parts.next().ok_or_else(|| invalid_date(field))?;
    parts.next().is_none().then_some(()).ok_or_else(|| invalid_date(field))?;
    Ok((year, month, day))
}

fn require_date_widths(
    value: &str,
    year: &str,
    month: &str,
    day: &str,
    field: &'static str,
) -> Result<(), ExceptionError> {
    if value.len() == 10 && year.len() == 4 && month.len() == 2 && day.len() == 2 {
        return Ok(());
    }
    Err(invalid_date(field))
}

fn require_digits(value: &str, field: &'static str) -> Result<(), ExceptionError> {
    if value.chars().all(|ch| ch.is_ascii_digit()) {
        return Ok(());
    }
    Err(invalid_date(field))
}

fn parse_date_part<T>(value: &str, field: &'static str) -> Result<T, ExceptionError>
where
    T: core::str::FromStr,
{
    value.parse().map_err(|_error| invalid_date(field))
}

fn validate_date_range(date: PolicyDate, field: &'static str) -> Result<(), ExceptionError> {
    let Some(limit) = day_limit(date) else {
        return Err(invalid_date(field));
    };
    if (1..=limit).contains(&date.day) {
        return Ok(());
    }
    Err(invalid_date(field))
}

#[must_use]
const fn day_limit(date: PolicyDate) -> Option<u8> {
    match date.month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 => Some(february_limit(date.year)),
        _ => None,
    }
}

#[must_use]
const fn february_limit(year: u16) -> u8 {
    if is_leap_year(year) { 29 } else { 28 }
}

#[must_use]
const fn is_leap_year(year: u16) -> bool {
    (is_divisible_by(year, 4) && !is_divisible_by(year, 100)) || is_divisible_by(year, 400)
}

#[must_use]
const fn is_divisible_by(value: u16, divisor: u16) -> bool {
    matches!(value.checked_rem(divisor), Some(0))
}

#[must_use]
fn invalid_date(field: &'static str) -> ExceptionError {
    ExceptionError::InvalidField {
        field,
        message: "date must be a canonical zero-padded YYYY-MM-DD calendar date".into(),
    }
}
