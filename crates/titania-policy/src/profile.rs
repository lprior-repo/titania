//! Parser and validator for `.titania/profiles/strict-ai/policy.toml`.

use std::collections::BTreeMap;

use serde::Deserialize;
use titania_core::WorkspacePath;

/// TOML scalar value accepted in policy override maps.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum PolicyValue {
    /// String scalar.
    String(String),
    /// Integer scalar.
    Integer(i64),
    /// Boolean scalar.
    Boolean(bool),
}

/// Parsed strict-ai policy override file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PolicyProfile {
    /// Clippy/rustc lint-level overrides keyed by lint name.
    #[serde(default)]
    pub lints: BTreeMap<String, String>,
    /// Numeric/string threshold overrides keyed by threshold name.
    #[serde(default)]
    pub thresholds: BTreeMap<String, PolicyValue>,
    /// Architecture import-boundary settings.
    pub architecture: PolicyArchitecture,
    /// Supply-chain policy overrides keyed by setting name.
    #[serde(default)]
    pub supply_chain: BTreeMap<String, PolicyValue>,
}

/// Architecture import-boundary settings from `policy.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PolicyArchitecture {
    /// Directories that count as core/domain source.
    pub core_dirs: Vec<WorkspacePath>,
    /// Infrastructure crates forbidden from core/domain source.
    pub infra_crates: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyDocument {
    #[serde(default)]
    lints: BTreeMap<String, String>,
    #[serde(default)]
    thresholds: BTreeMap<String, PolicyValue>,
    architecture: Option<PolicyArchitectureWire>,
    #[serde(default)]
    supply_chain: BTreeMap<String, PolicyValue>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyArchitectureWire {
    core_dirs: Vec<String>,
    infra_crates: Vec<String>,
}

/// Errors returned while parsing or validating `policy.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileError {
    /// TOML parsing failed.
    ParseError {
        /// Human-readable parser message.
        message: Box<str>,
    },
    /// A required field or section is absent or empty.
    MissingField {
        /// Dot-path of the missing field.
        field: &'static str,
    },
    /// A present field failed semantic validation.
    InvalidField {
        /// Dot-path of the invalid field.
        field: &'static str,
        /// Human-readable validation message.
        message: Box<str>,
    },
}

/// Parse and validate `.titania/profiles/strict-ai/policy.toml` content.
///
/// # Errors
/// Returns [`ProfileError::ParseError`] for malformed TOML,
/// [`ProfileError::MissingField`] for missing/empty required architecture
/// settings, and [`ProfileError::InvalidField`] for malformed paths, crate
/// names, or map keys.
pub fn parse_profile(content: &str) -> Result<PolicyProfile, ProfileError> {
    let document = toml_edit::de::from_str::<PolicyDocument>(content).map_err(|error| {
        ProfileError::ParseError { message: error.to_string().into_boxed_str() }
    })?;
    validate_policy_maps(&document)?;
    let architecture = validate_architecture(document.architecture)?;
    Ok(PolicyProfile {
        lints: document.lints,
        thresholds: document.thresholds,
        architecture,
        supply_chain: document.supply_chain,
    })
}

/// Validate override map keys and lint levels.
///
/// # Errors
/// Returns [`ProfileError::InvalidField`] when a map key is blank or a lint
/// level is not one of the supported rustc/clippy levels.
fn validate_policy_maps(document: &PolicyDocument) -> Result<(), ProfileError> {
    validate_named_map_keys(&document.lints, "lints")?;
    validate_lint_levels(&document.lints)?;
    validate_named_map_keys(&document.thresholds, "thresholds")?;
    validate_named_map_keys(&document.supply_chain, "supply_chain")
}

/// Validate required architecture fields.
///
/// # Errors
/// Returns [`ProfileError::MissingField`] when architecture, `core_dirs`, or
/// `infra_crates` is missing or empty. Returns [`ProfileError::InvalidField`]
/// when any path or crate entry is semantically invalid.
fn validate_architecture(
    architecture: Option<PolicyArchitectureWire>,
) -> Result<PolicyArchitecture, ProfileError> {
    let architecture = architecture.ok_or(ProfileError::MissingField { field: "architecture" })?;
    let core_dirs = validate_core_dirs(&architecture.core_dirs)?;
    let infra_crates = validate_infra_crates(architecture.infra_crates)?;
    Ok(PolicyArchitecture { core_dirs, infra_crates })
}

/// Validate core source directory entries.
///
/// # Errors
/// Returns [`ProfileError::MissingField`] when no entries are supplied, and
/// [`ProfileError::InvalidField`] when an entry is blank, whitespace-padded,
/// non-relative, or otherwise rejected by [`WorkspacePath`].
fn validate_core_dirs(entries: &[String]) -> Result<Vec<WorkspacePath>, ProfileError> {
    check_entries_present(entries, "architecture.core_dirs")?;
    entries.iter().map(String::as_str).map(validate_core_dir_entry).collect()
}

/// Validate infrastructure crate entries.
///
/// # Errors
/// Returns [`ProfileError::MissingField`] when no entries are supplied, and
/// [`ProfileError::InvalidField`] when an entry is blank, whitespace-padded, or
/// not a conservative Cargo package identifier.
fn validate_infra_crates(entries: Vec<String>) -> Result<Vec<String>, ProfileError> {
    check_entries_present(&entries, "architecture.infra_crates")?;
    entries.into_iter().map(validate_infra_crate_entry).collect()
}

/// Check a required list is present and non-empty.
///
/// # Errors
/// Returns [`ProfileError::MissingField`] when the list is empty.
fn check_entries_present<T>(entries: &[T], field: &'static str) -> Result<(), ProfileError> {
    (!entries.is_empty()).then_some(()).ok_or(ProfileError::MissingField { field })
}

/// Validate one core directory entry.
///
/// # Errors
/// Returns [`ProfileError::InvalidField`] when the entry is blank,
/// whitespace-padded, or invalid as a workspace-relative path.
fn validate_core_dir_entry(entry: &str) -> Result<WorkspacePath, ProfileError> {
    reject_surrounding_whitespace(entry, "architecture.core_dirs")?;
    WorkspacePath::new(entry).map_err(|error| ProfileError::InvalidField {
        field: "architecture.core_dirs",
        message: error.to_string().into_boxed_str(),
    })
}

/// Validate one infrastructure crate entry.
///
/// # Errors
/// Returns [`ProfileError::InvalidField`] when the entry is blank,
/// whitespace-padded, or contains characters outside lowercase ASCII letters,
/// digits, `_`, and `-`.
fn validate_infra_crate_entry(entry: String) -> Result<String, ProfileError> {
    reject_surrounding_whitespace(&entry, "architecture.infra_crates")?;
    reject_empty(&entry, "architecture.infra_crates")?;
    if !entry.chars().all(is_cargo_package_char) {
        return Err(ProfileError::InvalidField {
            field: "architecture.infra_crates",
            message: "infrastructure crate entry must use lowercase ASCII, digits, '_' or '-'"
                .into(),
        });
    }
    Ok(entry)
}

/// Validate map keys are non-empty and not whitespace-padded.
///
/// # Errors
/// Returns [`ProfileError::InvalidField`] when any key is blank or has leading
/// or trailing whitespace.
fn validate_named_map_keys<T>(
    map: &BTreeMap<String, T>,
    field: &'static str,
) -> Result<(), ProfileError> {
    map.keys().try_for_each(|key| validate_named_key(key, field))
}

/// Validate one map key.
///
/// # Errors
/// Returns [`ProfileError::InvalidField`] when the key is blank or
/// whitespace-padded.
fn validate_named_key(key: &str, field: &'static str) -> Result<(), ProfileError> {
    reject_surrounding_whitespace(key, field)?;
    reject_empty(key, field)
}

/// Validate lint override levels.
///
/// # Errors
/// Returns [`ProfileError::InvalidField`] when any lint level is not supported.
fn validate_lint_levels(lints: &BTreeMap<String, String>) -> Result<(), ProfileError> {
    lints.values().try_for_each(|level| validate_lint_level(level))
}

/// Validate one lint level.
///
/// # Errors
/// Returns [`ProfileError::InvalidField`] when the level is not allow, warn,
/// deny, or forbid.
fn validate_lint_level(level: &str) -> Result<(), ProfileError> {
    if matches!(level, "allow" | "warn" | "deny" | "forbid") {
        return Ok(());
    }
    Err(ProfileError::InvalidField {
        field: "lints",
        message: "lint level must be allow, warn, deny, or forbid".into(),
    })
}

/// Reject surrounding whitespace.
///
/// # Errors
/// Returns [`ProfileError::InvalidField`] when trimming would change the value.
fn reject_surrounding_whitespace(value: &str, field: &'static str) -> Result<(), ProfileError> {
    if value.trim() == value {
        return Ok(());
    }
    Err(ProfileError::InvalidField {
        field,
        message: "value must not have surrounding whitespace".into(),
    })
}

/// Reject blank values.
///
/// # Errors
/// Returns [`ProfileError::InvalidField`] when the value is empty after trim.
fn reject_empty(value: &str, field: &'static str) -> Result<(), ProfileError> {
    if value.trim().is_empty() {
        return Err(ProfileError::InvalidField {
            field,
            message: "value must not be blank".into(),
        });
    }
    Ok(())
}

#[must_use]
const fn is_cargo_package_char(ch: char) -> bool {
    matches!(ch, 'a'..='z' | '0'..='9' | '_' | '-')
}
