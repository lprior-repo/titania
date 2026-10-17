use core::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::RuleIdError;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RuleId(String);

impl RuleId {
    pub const MAX_LEN: usize = 96;

    pub fn new(s: &str) -> Result<Self, RuleIdError> {
        check_rule_id(s)?;
        Ok(Self(s.to_owned()))
    }

    pub fn normalize(s: &str) -> Result<Self, RuleIdError> {
        let buf = normalize_rule_id(s);
        Self::new(&buf)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn prefix(&self) -> &str {
        split_prefix(&self.0)
    }

    #[must_use]
    pub fn has_prefix(&self, p: &str) -> bool {
        self.prefix() == p
    }
}

fn check_rule_id(s: &str) -> Result<(), RuleIdError> {
    check_non_empty(s)?;
    check_has_underscore(s)?;
    check_uppercase(s)?;
    if s.len() > RuleId::MAX_LEN {
        return Err(RuleIdError::TooLong(s.len()));
    }
    Ok(())
}

const fn check_non_empty(s: &str) -> Result<(), RuleIdError> {
    if s.is_empty() {
        return Err(RuleIdError::Empty);
    }
    Ok(())
}

fn check_has_underscore(s: &str) -> Result<(), RuleIdError> {
    if !s.contains('_') {
        return Err(RuleIdError::NoUnderscore);
    }
    Ok(())
}

fn check_uppercase(s: &str) -> Result<(), RuleIdError> {
    match s.char_indices().find(|(_idx, ch)| !is_rule_id_char(*ch)) {
        Some((idx, bad_char)) => Err(RuleIdError::NotUppercase(bad_char, idx)),
        None => Ok(()),
    }
}

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
