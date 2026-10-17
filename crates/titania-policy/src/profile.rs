use std::collections::BTreeMap;

use serde::Deserialize;
use titania_core::WorkspacePath;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum PolicyValue {
    String(String),
    Integer(i64),
    Boolean(bool),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PolicyProfile {
    #[serde(default)]
    pub lints: BTreeMap<String, String>,
    #[serde(default)]
    pub thresholds: BTreeMap<String, PolicyValue>,
    pub architecture: PolicyArchitecture,
    #[serde(default)]
    pub supply_chain: BTreeMap<String, PolicyValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PolicyArchitecture {
    pub core_dirs: Vec<WorkspacePath>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileError {
    ParseError { message: Box<str> },
    MissingField { field: &'static str },
    InvalidField { field: &'static str, message: Box<str> },
}

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

fn validate_policy_maps(document: &PolicyDocument) -> Result<(), ProfileError> {
    validate_named_map_keys(&document.lints, "lints")?;
    validate_lint_levels(&document.lints)?;
    validate_named_map_keys(&document.thresholds, "thresholds")?;
    validate_named_map_keys(&document.supply_chain, "supply_chain")
}

fn validate_architecture(
    architecture: Option<PolicyArchitectureWire>,
) -> Result<PolicyArchitecture, ProfileError> {
    let architecture = architecture.ok_or(ProfileError::MissingField { field: "architecture" })?;
    let core_dirs = validate_core_dirs(&architecture.core_dirs)?;
    let infra_crates = validate_infra_crates(architecture.infra_crates)?;
    Ok(PolicyArchitecture { core_dirs, infra_crates })
}

fn validate_core_dirs(entries: &[String]) -> Result<Vec<WorkspacePath>, ProfileError> {
    check_entries_present(entries, "architecture.core_dirs")?;
    entries.iter().map(String::as_str).map(validate_core_dir_entry).collect()
}

fn validate_infra_crates(entries: Vec<String>) -> Result<Vec<String>, ProfileError> {
    check_entries_present(&entries, "architecture.infra_crates")?;
    entries.into_iter().map(validate_infra_crate_entry).collect()
}

fn check_entries_present<T>(entries: &[T], field: &'static str) -> Result<(), ProfileError> {
    (!entries.is_empty()).then_some(()).ok_or(ProfileError::MissingField { field })
}

fn validate_core_dir_entry(entry: &str) -> Result<WorkspacePath, ProfileError> {
    reject_surrounding_whitespace(entry, "architecture.core_dirs")?;
    WorkspacePath::new(entry).map_err(|error| ProfileError::InvalidField {
        field: "architecture.core_dirs",
        message: error.to_string().into_boxed_str(),
    })
}

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

fn validate_named_map_keys<T>(
    map: &BTreeMap<String, T>,
    field: &'static str,
) -> Result<(), ProfileError> {
    map.keys().try_for_each(|key| validate_named_key(key, field))
}

fn validate_named_key(key: &str, field: &'static str) -> Result<(), ProfileError> {
    reject_surrounding_whitespace(key, field)?;
    reject_empty(key, field)
}

fn validate_lint_levels(lints: &BTreeMap<String, String>) -> Result<(), ProfileError> {
    lints.values().try_for_each(|level| validate_lint_level(level))
}

fn validate_lint_level(level: &str) -> Result<(), ProfileError> {
    if matches!(level, "allow" | "warn" | "deny" | "forbid") {
        return Ok(());
    }
    Err(ProfileError::InvalidField {
        field: "lints",
        message: "lint level must be allow, warn, deny, or forbid".into(),
    })
}

fn reject_surrounding_whitespace(value: &str, field: &'static str) -> Result<(), ProfileError> {
    if value.trim() == value {
        return Ok(());
    }
    Err(ProfileError::InvalidField {
        field,
        message: "value must not have surrounding whitespace".into(),
    })
}

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
