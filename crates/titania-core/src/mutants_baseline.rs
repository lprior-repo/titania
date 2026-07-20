//! v1.5 mutants-baseline typed JSON document.
//!
//! Baseline lives at `.titania/profiles/strict-ai/mutants.baseline.json` and
//! records the surviving mutants the v1.5 contract accepts. An empty
//! `entries` array is the goal: any mutation that survives tests must
//! either be killed by adding a regression test, or be added to the
//! baseline via `scripts/dev/mutants-bootstrap.sh` with an explicit
//! `titania-bypass-mutant-<id>` policy exception.

use serde::{Deserialize, Serialize};

use crate::{error::MutantsBaselineError, proof_id::MutantId};

/// Schema version understood by this crate.
pub const MUTANTS_BASELINE_SCHEMA_VERSION: u32 = 1;

/// Required literal prefix for `accepted_by_rule`. The whole value must
/// match `mutant-accept/<owner>/<reason>/<expiry>` so a hostile baseline
/// cannot sneak in alternative acceptance strings.
pub const ACCEPTED_BY_RULE_FAMILY: &str = "mutant-accept";

/// One accepted survivor inside the baseline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutantBaselineEntry {
    /// Stable typed mutation id this entry accepts. Deserialised via the
    /// [`MutantId`] parser so malformed or wildcard ids cannot land in a
    /// baseline.
    pub mutation_id: MutantId,
    /// `accepted-by-rule` literal of the form
    /// `mutant-accept/<owner>/<reason>/<expiry>`. The shape is enforced by
    /// [`MutantsBaseline::parse_str`].
    pub accepted_by_rule: String,
    /// Human-readable reason for the bypass. Must be non-empty and contain
    /// at least one non-whitespace byte.
    pub reason: String,
    /// Optional expiry as unix-seconds; `None` ⇒ never expires.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_on_unix: Option<u64>,
}

/// Typed mutants baseline document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutantsBaseline {
    /// Schema version; must equal [`MUTANTS_BASELINE_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Optional ISO-8601 computed-at timestamp (kept as raw string for
    /// serde-stability; no `chrono` dependency is introduced here).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub computed_at: Option<String>,
    /// All accepted entries.
    pub entries: Vec<MutantBaselineEntry>,
}

impl Default for MutantsBaseline {
    fn default() -> Self {
        Self::empty()
    }
}

impl MutantsBaseline {
    /// Build an empty baseline with the current schema version.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            schema_version: MUTANTS_BASELINE_SCHEMA_VERSION,
            computed_at: None,
            entries: Vec::new(),
        }
    }

    /// Construct a baseline from raw `bypasses` rows (operator-facing
    /// entry point; used by the bootstrap script and tests).
    #[must_use]
    pub const fn from_bypasses(entries: Vec<MutantBaselineEntry>) -> Self {
        Self { schema_version: MUTANTS_BASELINE_SCHEMA_VERSION, computed_at: None, entries }
    }

    /// Parse a baseline from a UTF-8 string slice without touching the
    /// filesystem. `path` is a caller-provided label (typically the file
    /// path) used in error diagnostics so the lane can surface structured
    /// [`MutantsBaselineError`] reasons.
    ///
    /// # Errors
    /// - [`MutantsBaselineError::JsonParse`] when `contents` is not valid JSON.
    /// - [`MutantsBaselineError::UnsupportedSchemaVersion`] when the
    ///   `schema_version` does not match
    ///   [`MUTANTS_BASELINE_SCHEMA_VERSION`].
    /// - [`MutantsBaselineError::InvalidAcceptedByRule`] when any entry's
    ///   `accepted_by_rule` does not match the contract family
    ///   `mutant-accept/<owner>/<reason>/<expiry>`, has fewer or more
    ///   than three non-empty components, or carries invalid `expiry`.
    /// - [`MutantsBaselineError::InvalidReason`] when any entry's
    ///   `reason` is empty or whitespace-only.
    ///
    /// Mutation-id validation runs inside serde's
    /// [`MutantId`] deserialise and surfaces a
    /// [`MutantsBaselineError::JsonParse`] with a descriptive `reason`
    /// when the literal does not match the canonical
    /// `<pkg>::<rel-path>:<line>:<col>:<operator>` shape.
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

    /// True when `mutation_id` is covered by any non-expired baseline entry.
    ///
    /// `now_unix` is the reference timestamp (typically
    /// `SystemTime::now().duration_since(UNIX_EPOCH).as_secs()`); pass
    /// `u64::MAX` for "treat expiry as disabled".
    #[must_use]
    pub fn contains(&self, mutation_id: &MutantId, now_unix: u64) -> bool {
        self.entries.iter().any(|entry| entry_covers(entry, mutation_id, now_unix))
    }

    /// Set-difference: `survivors - baseline` using [`Self::contains`].
    ///
    /// Empty result ⇒ every survivor is in the baseline; non-empty ⇒ the
    /// lane must emit one `MUTANT_SURVIVED` finding per returned id.
    #[must_use]
    pub fn diff<'a>(&self, survivors: &'a [MutantId], now_unix: u64) -> Vec<&'a MutantId> {
        survivors.iter().filter(|id| !self.contains(id, now_unix)).collect()
    }

    /// Borrow the entries slice.
    #[must_use]
    pub fn entries(&self) -> &[MutantBaselineEntry] {
        &self.entries
    }

    /// Borrow the schema version.
    #[must_use]
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    /// Internal validator runs after `serde_json::from_str` so any entry
    /// typed mismatch (wildcard ids, missing separators, etc.) and any
    /// `accepted_by_rule` shape drift is converted into a typed
    /// [`MutantsBaselineError`] rather than silently producing an invalid
    /// baseline.
    ///
    /// # Errors
    /// - [`MutantsBaselineError::UnsupportedSchemaVersion`] when the version
    ///   is mismatched.
    /// - [`MutantsBaselineError::InvalidAcceptedByRule`] when any entry has
    ///   an invalid `accepted_by_rule`.
    /// - [`MutantsBaselineError::InvalidReason`] when any entry's
    ///   `reason` is empty or whitespace-only.
    ///
    /// Mutation-id validation is performed by the [`MutantId`] deserialiser
    /// itself, which surfaces inside [`MutantsBaselineError::JsonParse`].
    fn validate(&self, path: &str) -> Result<(), MutantsBaselineError> {
        check_schema_version(self.schema_version, path)?;
        validate_entries(&self.entries, path)
    }
}

/// # Errors
/// - [`MutantsBaselineError::UnsupportedSchemaVersion`] when the version
///   does not match [`MUTANTS_BASELINE_SCHEMA_VERSION`].
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

/// Validate every entry's `accepted_by_rule` and `reason` fields.
///
/// # Errors
/// - [`MutantsBaselineError::InvalidAcceptedByRule`] when any entry has an
///   invalid `accepted_by_rule`.
/// - [`MutantsBaselineError::InvalidReason`] when any entry's `reason`
///   field is empty or whitespace-only.
fn validate_entries(
    entries: &[MutantBaselineEntry],
    path: &str,
) -> Result<(), MutantsBaselineError> {
    entries.iter().try_for_each(|entry| {
        validate_accepted_by_rule(&entry.accepted_by_rule, path)?;
        validate_reason(&entry.reason, path)
    })
}

/// Reject whitespace-only `reason` fields. The `reason` is the audit
/// trail for the bypass and must carry visible content.
///
/// # Errors
/// - [`MutantsBaselineError::InvalidReason`] when `reason` is empty or
///   every byte is ASCII whitespace.
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

/// # Errors
/// - [`MutantsBaselineError::InvalidAcceptedByRule`] when the literal does
///   not match `mutant-accept/<owner>/<reason>/<expiry>` or any component
///   is empty.
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

/// Validate the `mutant-accept/<owner>/<reason>/<expiry>` shape.
///
/// The literal must split on `/` into exactly three non-empty parts after
/// the `mutant-accept` prefix. An extra `/` segment is rejected (no
/// wildcard accommodation) and the `expiry` segment must be `never` or a
/// strictly-positive decimal integer that fits in `u64` without
/// overflow.
///
/// # Errors
/// - Returns a static `&'static str` error message when the prefix is
///   wrong, a separator is missing, any segment is empty, fewer or more
///   than three segments appear, or `expiry` is neither `never` nor a
///   strictly-positive integer that fits in `u64`.
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

/// Validate the `expiry` segment of an `accepted_by_rule` literal.
///
/// Accepts the literal `never` or a strictly-positive decimal integer
/// that fits in `u64` without overflow. The empty string, a leading `0`
/// (e.g. `"0"`, `"00"`), and out-of-range digits are rejected.
fn is_valid_expiry(expiry: &str) -> bool {
    if expiry == "never" {
        return true;
    }
    parse_positive_u64_strict(expiry)
}

/// Validate the `expiry` segment of an `accepted_by_rule` literal via
/// checked arithmetic.
///
/// Accepts the literal `never` or a strictly-positive decimal integer
/// that fits in `u64` without overflow. The empty string, leading `0`
/// (e.g. `"0"`, `"00"`), and out-of-range digits are rejected.
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
