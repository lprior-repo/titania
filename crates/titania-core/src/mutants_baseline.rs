use serde::{Deserialize, Serialize};

use crate::{error::MutantsBaselineError, proof_id::MutantId};

pub const MUTANTS_BASELINE_SCHEMA_VERSION: u32 = 1;

pub const ACCEPTED_BY_RULE_FAMILY: &str = "mutant-accept";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutantBaselineEntry {
    pub mutation_id: MutantId,
    pub accepted_by_rule: String,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_on_unix: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutantsBaseline {
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub computed_at: Option<String>,
    pub entries: Vec<MutantBaselineEntry>,
}

impl Default for MutantsBaseline {
    fn default() -> Self {
        Self::empty()
    }
}

impl MutantsBaseline {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            schema_version: MUTANTS_BASELINE_SCHEMA_VERSION,
            computed_at: None,
            entries: Vec::new(),
        }
    }

    #[must_use]
    pub const fn from_bypasses(entries: Vec<MutantBaselineEntry>) -> Self {
        Self { schema_version: MUTANTS_BASELINE_SCHEMA_VERSION, computed_at: None, entries }
    }

    pub fn parse_str(contents: &str, path: &str) -> Result<Self, MutantsBaselineError> {
        let path_label: Box<str> = Box::from(path);
        let baseline: Self =
            serde_json::from_str(contents).map_err(|error| MutantsBaselineError::JsonParse {
                path: path_label.clone(),
                reason: error.to_string().into_boxed_str(),
            })?;
        baseline.validate(path)?;
        Ok(baseline)
    }

    #[must_use]
    pub fn contains(&self, mutation_id: &MutantId, now_unix: u64) -> bool {
        self.entries.iter().any(|entry| entry_covers(entry, mutation_id, now_unix))
    }

    #[must_use]
    pub fn diff<'a>(&self, survivors: &'a [MutantId], now_unix: u64) -> Vec<&'a MutantId> {
        survivors.iter().filter(|id| !self.contains(id, now_unix)).collect()
    }

    #[must_use]
    pub fn entries(&self) -> &[MutantBaselineEntry] {
        &self.entries
    }

    #[must_use]
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    fn validate(&self, path: &str) -> Result<(), MutantsBaselineError> {
        check_schema_version(self.schema_version, path)?;
        validate_entries(&self.entries, path)
    }
}

fn check_schema_version(found: u32, path: &str) -> Result<(), MutantsBaselineError> {
    let expected = MUTANTS_BASELINE_SCHEMA_VERSION;
    if found != expected {
        return Err(MutantsBaselineError::UnsupportedSchemaVersion {
            path: Box::from(path),
            found,
            expected,
        });
    }
    Ok(())
}

fn validate_entries(
    entries: &[MutantBaselineEntry],
    path: &str,
) -> Result<(), MutantsBaselineError> {
    entries.iter().try_for_each(|entry| {
        validate_accepted_by_rule(&entry.accepted_by_rule, path)?;
        validate_reason(&entry.reason, path)
    })
}

fn validate_reason(reason: &str, path: &str) -> Result<(), MutantsBaselineError> {
    let trimmed_has_content = reason.bytes().any(|byte| !byte.is_ascii_whitespace());
    if trimmed_has_content {
        Ok(())
    } else {
        Err(MutantsBaselineError::InvalidReason {
            path: Box::from(path),
            reason: Box::from(reason),
        })
    }
}

fn validate_accepted_by_rule(
    accepted_by_rule: &str,
    path: &str,
) -> Result<(), MutantsBaselineError> {
    let reason: &'static str = match check_accepted_rule_shape(accepted_by_rule) {
        Ok(()) => return Ok(()),
        Err(reason) => reason,
    };
    Err(MutantsBaselineError::InvalidAcceptedByRule {
        path: Box::from(path),
        accepted_by_rule: Box::from(accepted_by_rule),
        reason: Box::from(reason),
    })
}

fn check_accepted_rule_shape(accepted_by_rule: &str) -> Result<(), &'static str> {
    let Some(rest) = accepted_by_rule.strip_prefix(ACCEPTED_BY_RULE_FAMILY) else {
        return Err("must start with `mutant-accept`");
    };
    let Some(rest) = rest.strip_prefix('/') else {
        return Err("missing `/` after `mutant-accept`");
    };
    let mut iter = rest.split('/');
    let owner: &str = iter.next().map_or("", |value| value);
    let reason_segment: &str = iter.next().map_or("", |value| value);
    let expiry: &str = iter.next().map_or("", |value| value);
    let overflow: bool = iter.next().is_some();
    if overflow || owner.is_empty() {
        return Err("must have exactly three non-empty `/`-separated segments");
    }
    if reason_segment.is_empty() {
        return Err("must have exactly three non-empty `/`-separated segments");
    }
    if expiry.is_empty() {
        return Err("must have exactly three non-empty `/`-separated segments");
    }
    if !is_valid_expiry(expiry) {
        return Err("expiry must be `never` or a positive integer");
    }
    Ok(())
}

fn is_valid_expiry(expiry: &str) -> bool {
    if expiry == "never" {
        return true;
    }
    parse_positive_u64_strict(expiry)
}

fn parse_positive_u64_strict(s: &str) -> bool {
    let parsed: Option<u64> = s.bytes().try_fold(0u64, |acc, byte| -> Option<u64> {
        let digit = u64::from(byte.checked_sub(b'0')?);
        let valid = digit <= 9;
        let next = acc.checked_mul(10)?.checked_add(digit)?;
        valid.then_some(next)
    });
    matches!(parsed, Some(value) if value > 0)
}

#[must_use]
fn entry_covers(entry: &MutantBaselineEntry, mutation_id: &MutantId, now_unix: u64) -> bool {
    let id_match = entry.mutation_id == *mutation_id;
    let in_date = entry.expires_on_unix.is_none_or(|exp| now_unix <= exp);
    id_match && in_date
}
