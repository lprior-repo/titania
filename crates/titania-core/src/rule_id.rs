//! Rule identifier. A namespaced, uppercase-ASCII identifier with at least
//! one underscore, e.g. `FUNC_LOOPS_FOR`, `CLIPPY_UNWRAP_USED`,
//! `ARCHITECTURE_IMPORT_CORE_FS`.
//!
//! Construction is total: [`RuleId::new`] validates and returns the value
//! or a [`RuleIdError`].

use core::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::RuleIdError;

/// A validated rule identifier string.
///
/// Once constructed, the inner string is guaranteed to be:
/// - non-empty,
/// - all uppercase ASCII (`A-Z`, `0-9`),
/// - containing at least one underscore (`_`).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RuleId(String);

impl RuleId {
    /// Maximum length of a rule identifier.
    pub const MAX_LEN: usize = 96;

    /// Construct a [`RuleId`] from any input. Lowercase letters and mixed
    /// input are rejected — call [`RuleId::normalize`] first if you have
    /// untrusted casing.
    ///
    /// # Errors
    /// - [`RuleIdError::Empty`] if `s` is empty.
    /// - [`RuleIdError::NoUnderscore`] if `s` has no underscore.
    /// - [`RuleIdError::NotUppercase`] if `s` contains any character that
    ///   is not uppercase ASCII (`A-Z`, `0-9`).
    /// - [`RuleIdError::TooLong`] if `s` exceeds [`RuleId::MAX_LEN`] characters.
    pub fn new(s: &str) -> Result<Self, RuleIdError> {
        check_rule_id(s)?;
        Ok(Self(s.to_owned()))
    }

    /// Normalize input to a rule identifier, then validate it with [`RuleId::new`].
    ///
    /// # Errors
    /// Returns [`RuleIdError`] when normalized input is empty, lacks an underscore,
    /// or contains no legal rule-id characters after filtering.
    pub fn normalize(s: &str) -> Result<Self, RuleIdError> {
        let buf = normalize_rule_id(s);
        Self::new(&buf)
    }

    /// Borrow the inner string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Prefix before the first underscore (e.g. `FUNC` in `FUNC_LOOPS_FOR`).
    #[must_use]
    pub fn prefix(&self) -> &str {
        split_prefix(&self.0)
    }

    /// Whether this rule id has the given prefix.
    #[must_use]
    pub fn has_prefix(&self, p: &str) -> bool {
        self.prefix() == p
    }
}

/// Validate a rule identifier string.
/// # Errors
/// - [`RuleIdError::Empty`] if `s` is empty.
/// - [`RuleIdError::NoUnderscore`] if `s` lacks an underscore.
/// - [`RuleIdError::NotUppercase`] if `s` contains invalid characters.
/// - [`RuleIdError::TooLong`] if `s` exceeds [`RuleId::MAX_LEN`] characters.
fn check_rule_id(s: &str) -> Result<(), RuleIdError> {
    check_non_empty(s)?;
    check_has_underscore(s)?;
    check_uppercase(s)?;
    if s.len() > RuleId::MAX_LEN {
        return Err(RuleIdError::TooLong(s.len()));
    }
    Ok(())
}

/// # Errors
/// [`RuleIdError::Empty`] if the string is empty.
const fn check_non_empty(s: &str) -> Result<(), RuleIdError> {
    if s.is_empty() {
        return Err(RuleIdError::Empty);
    }
    Ok(())
}

/// # Errors
/// [`RuleIdError::NoUnderscore`] if `s` lacks an underscore.
fn check_has_underscore(s: &str) -> Result<(), RuleIdError> {
    if !s.contains('_') {
        return Err(RuleIdError::NoUnderscore);
    }
    Ok(())
}

/// # Errors
/// [`RuleIdError::NotUppercase`] at the first non-uppercase character.
fn check_uppercase(s: &str) -> Result<(), RuleIdError> {
    match s.char_indices().find(|(_idx, ch)| !is_rule_id_char(*ch)) {
        Some((idx, bad_char)) => Err(RuleIdError::NotUppercase(bad_char, idx)),
        None => Ok(()),
    }
}

/// Returns `true` if `ch` is a valid rule-identifier character.
#[must_use]
const fn is_rule_id_char(ch: char) -> bool {
    matches!(ch, 'A'..='Z' | '0'..='9' | '_')
}

fn normalize_rule_id(s: &str) -> String {
    s.chars().filter_map(filter_and_upper).collect()
}

const fn filter_and_upper(ch: char) -> Option<char> {
    if ch.is_ascii_lowercase() {
        Some(ch.to_ascii_uppercase())
    } else if is_rule_id_char(ch) {
        Some(ch)
    } else {
        None
    }
}

/// Split the rule id at the first underscore, returning the prefix as `&str`.
///
/// Safe because our invariant guarantees the string is all ASCII;
/// char indices equal byte indices.
fn split_prefix(s: &str) -> &str {
    s.split_once('_').map_or(s, |(prefix, _)| prefix)
}

impl fmt::Display for RuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for RuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RuleId({})", self.0)
    }
}

impl FromStr for RuleId {
    type Err = RuleIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl Serialize for RuleId {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for RuleId {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let s = <std::borrow::Cow<'_, str> as Deserialize>::deserialize(de)?;
        Self::new(&s).map_err(serde::de::Error::custom)
    }
}
