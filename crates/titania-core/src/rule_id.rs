//! Rule identifier. A namespaced, uppercase-ASCII identifier with at least
//! one underscore, e.g. `FUNC_LOOPS_FOR`, `CLIPPY_UNWRAP_USED`,
//! `ARCHITECTURE_IMPORT_CORE_FS`.
//!
//! Construction validates and returns a [`RuleId`] or a [`RuleIdError`].
//! Once constructed, the invariants are type-enforced.

use core::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::RuleIdError;

/// A validated rule identifier string.
///
/// Once constructed, the inner string is guaranteed to be:
/// - non-empty,
/// - at most [`RuleId::MAX_LEN`] bytes,
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
    /// - [`RuleIdError::TooLong`] if `s` exceeds [`RuleId::MAX_LEN`] bytes.
    /// - [`RuleIdError::NoUnderscore`] if `s` has no underscore.
    /// - [`RuleIdError::NotUppercase`] if `s` contains any character that
    ///   is not uppercase ASCII (`A-Z`, `0-9`).
    pub fn new(s: &str) -> Result<Self, RuleIdError> {
        (!s.is_empty()).then_some(()).ok_or(RuleIdError::Empty)?;
        (s.len() <= Self::MAX_LEN)
            .then_some(())
            .ok_or(RuleIdError::TooLong { max: Self::MAX_LEN, got: s.len() })?;
        s.char_indices().try_for_each(|(i, c)| validate_rule_char(c, i))?;
        s.contains('_').then_some(()).ok_or(RuleIdError::NoUnderscore)?;
        Ok(Self(s.to_owned()))
    }

    /// Normalize input to a rule identifier: uppercase ASCII, drop illegal
    /// characters, then validate. Returns the same errors as [`RuleId::new`].
    ///
    /// # Errors
    /// Returns [`RuleIdError`] when normalized input is empty, too long, lacks
    /// an underscore, or contains no legal rule-id characters after filtering.
    pub fn normalize(s: &str) -> Result<Self, RuleIdError> {
        let buf: String = s.chars().filter_map(normalize_rule_char).collect();
        Self::new(&buf)
    }

    /// Borrow the inner string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Prefix before the first underscore (e.g. `FUNC` in `FUNC_LOOPS_FOR`).
    ///
    /// The type invariant guarantees that `self.0` contains only uppercase
    /// ASCII, digits, and `_`. `split_once('_')` returns well-formed `&str`
    /// slices without byte-index slicing, so no `string_slice` waiver is
    /// needed.
    #[must_use]
    pub fn prefix(&self) -> &str {
        match self.0.split_once('_') {
            Some((head, _)) => head,
            None => &self.0,
        }
    }

    /// Whether this rule id has the given prefix.
    #[must_use]
    pub fn has_prefix(&self, p: &str) -> bool {
        self.prefix() == p
    }
}

/// Validate a single rule-id character: `_`, `A-Z`, or `0-9`.
///
/// # Errors
/// Returns [`RuleIdError::NotUppercase`] when `c` is none of those.
fn validate_rule_char(c: char, i: usize) -> Result<(), RuleIdError> {
    (c == '_' || matches!(c, 'A'..='Z' | '0'..='9'))
        .then_some(())
        .ok_or(RuleIdError::NotUppercase(c, i))
}

/// Normalize one character for [`RuleId::normalize`]: uppercase ASCII
/// letters, digits, and underscores survive; lowercase is uppercased; all
/// other characters are dropped.
fn normalize_rule_char(ch: char) -> Option<char> {
    ch.is_ascii_lowercase()
        .then(|| ch.to_ascii_uppercase())
        .or_else(|| (ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_').then_some(ch))
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
