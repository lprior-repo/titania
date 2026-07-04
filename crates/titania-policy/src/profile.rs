//! Parser and validator for `.titania/profiles/strict-ai/policy.toml`.

use std::collections::BTreeMap;

use serde::Deserialize;

/// Parsed strict-ai policy override file.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PolicyProfile {
    /// Clippy/rustc lint-level overrides keyed by lint name.
    #[serde(default)]
    pub lints: BTreeMap<String, String>,
    /// Numeric/string threshold overrides keyed by threshold name.
    #[serde(default)]
    pub thresholds: BTreeMap<String, toml::Value>,
    /// Architecture import-boundary settings.
    pub architecture: PolicyArchitecture,
    /// Supply-chain policy overrides keyed by setting name.
    #[serde(default)]
    pub supply_chain: BTreeMap<String, toml::Value>,
}

/// Architecture import-boundary settings from `policy.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PolicyArchitecture {
    /// Directories that count as core/domain source.
    pub core_dirs: Vec<String>,
    /// Infrastructure crates forbidden from core/domain source.
    pub infra_crates: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PolicyDocument {
    #[serde(default)]
    lints: BTreeMap<String, String>,
    #[serde(default)]
    thresholds: BTreeMap<String, toml::Value>,
    architecture: Option<PolicyArchitecture>,
    #[serde(default)]
    supply_chain: BTreeMap<String, toml::Value>,
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
}

/// Parse and validate `.titania/profiles/strict-ai/policy.toml` content.
///
/// # Errors
/// Returns [`ProfileError::ParseError`] for malformed TOML and
/// [`ProfileError::MissingField`] for missing/empty required architecture
/// settings.
pub fn parse_profile(content: &str) -> Result<PolicyProfile, ProfileError> {
    let document = toml::from_str::<PolicyDocument>(content).map_err(|error| {
        ProfileError::ParseError { message: error.to_string().into_boxed_str() }
    })?;
    let architecture = validate_architecture(document.architecture)?;
    Ok(PolicyProfile {
        lints: document.lints,
        thresholds: document.thresholds,
        architecture,
        supply_chain: document.supply_chain,
    })
}

/// Validate required architecture fields.
///
/// # Errors
/// Returns [`ProfileError::MissingField`] when architecture, `core_dirs`, or
/// `infra_crates` is missing or empty.
fn validate_architecture(
    architecture: Option<PolicyArchitecture>,
) -> Result<PolicyArchitecture, ProfileError> {
    let architecture = architecture.ok_or(ProfileError::MissingField { field: "architecture" })?;
    if architecture.core_dirs.is_empty() {
        return Err(ProfileError::MissingField { field: "architecture.core_dirs" });
    }
    if architecture.infra_crates.is_empty() {
        return Err(ProfileError::MissingField { field: "architecture.infra_crates" });
    }
    Ok(architecture)
}
