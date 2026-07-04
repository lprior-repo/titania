//! Canonical strict-ai policy digest calculation.

use std::collections::BTreeMap;

use serde::Deserialize;
use titania_core::Digest;

use crate::PolicyDefaults;

/// Blake3 digest of binary policy defaults and v1 policy input files.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PolicyDigest {
    digest: Digest,
}

impl PolicyDigest {
    /// Compute the canonical v1 policy digest.
    ///
    /// Each component is serialized with explicit length prefixes before the
    /// final Blake3 hash. TOML inputs are parsed into a sorted representation so
    /// comments and source key order do not affect the digest.
    #[must_use]
    pub fn compute(
        defaults: &PolicyDefaults,
        policy_toml: Option<&str>,
        exceptions_toml: Option<&str>,
        deny_toml: Option<&str>,
        clippy_toml: Option<&str>,
    ) -> Self {
        let mut payload = String::new();
        push_segment(&mut payload, "binary_defaults", &canonical_defaults(defaults));
        push_segment(&mut payload, "policy_toml", &canonical_toml(policy_toml));
        push_segment(&mut payload, "exceptions_toml", &canonical_toml(exceptions_toml));
        push_segment(&mut payload, "deny_toml", &canonical_toml(deny_toml));
        push_segment(&mut payload, "clippy_toml", &canonical_toml(clippy_toml));
        Self { digest: Digest::from_bytes(payload.as_bytes()) }
    }

    /// Borrow the 64-character lowercase hex digest.
    #[must_use]
    pub fn as_hex(&self) -> &str {
        self.digest.as_hex()
    }

    /// Borrow the validated core digest.
    #[must_use]
    pub const fn digest(&self) -> &Digest {
        &self.digest
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum CanonicalTomlValue {
    Table(BTreeMap<String, Self>),
    Array(Vec<Self>),
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Datetime(toml_edit::Datetime),
}

fn canonical_defaults(defaults: &PolicyDefaults) -> String {
    let mut out = String::new();
    push_segment(&mut out, "schema_version", &defaults.schema_version.to_string());
    push_segment(&mut out, "profile_name", &defaults.profile_name);
    out.push_str(&canonical_string_list("sources", &defaults.sources));
    out.push_str(&canonical_string_list(
        "architecture.core_dirs",
        &defaults.architecture.core_dirs,
    ));
    out.push_str(&canonical_string_list(
        "architecture.infra_crates",
        &defaults.architecture.infra_crates,
    ));
    push_segment(&mut out, "embedded", bool_text(defaults.embedded));
    out
}

fn canonical_toml(input: Option<&str>) -> String {
    let Some(text) = input else {
        return String::new();
    };
    match toml_edit::de::from_str::<BTreeMap<String, CanonicalTomlValue>>(text) {
        Ok(table) => canonical_table(&table),
        Err(error) => canonical_invalid_toml(text, &error),
    }
}

fn canonical_invalid_toml(text: &str, error: &toml_edit::de::Error) -> String {
    let mut out = String::new();
    push_segment(&mut out, "invalid_toml_error", &error.to_string());
    push_segment(&mut out, "invalid_toml_raw", text);
    out
}

fn canonical_table(table: &BTreeMap<String, CanonicalTomlValue>) -> String {
    let entries = table
        .iter()
        .map(|(key, value)| {
            let mut entry = String::new();
            push_segment(&mut entry, "key", key);
            entry.push_str(&canonical_value(value));
            entry
        })
        .collect::<String>();
    let mut out = String::new();
    out.push_str("table");
    push_len(&mut out, table.len());
    out.push_str(&entries);
    out
}

fn canonical_value(value: &CanonicalTomlValue) -> String {
    match value {
        CanonicalTomlValue::Table(table) => canonical_table(table),
        CanonicalTomlValue::Array(values) => canonical_array(values),
        CanonicalTomlValue::String(value) => segment("string", value),
        CanonicalTomlValue::Integer(value) => segment("integer", &value.to_string()),
        CanonicalTomlValue::Float(value) => segment("float", &value.to_string()),
        CanonicalTomlValue::Boolean(value) => segment("boolean", bool_text(*value)),
        CanonicalTomlValue::Datetime(value) => segment("datetime", &value.to_string()),
    }
}

fn canonical_array(values: &[CanonicalTomlValue]) -> String {
    let entries = values.iter().map(canonical_value).collect::<String>();
    let mut out = String::new();
    out.push_str("array");
    push_len(&mut out, values.len());
    out.push_str(&entries);
    out
}

fn canonical_string_list(label: &str, values: &[String]) -> String {
    let entries = values.iter().map(|value| segment("item", value)).collect::<String>();
    let mut out = String::new();
    out.push_str(label);
    push_len(&mut out, values.len());
    out.push_str(&entries);
    out
}

fn segment(label: &str, text: &str) -> String {
    let mut out = String::new();
    push_segment(&mut out, label, text);
    out
}

fn push_segment(out: &mut String, label: &str, text: &str) {
    out.push_str(label);
    push_len(out, text.len());
    out.push_str(text);
}

fn push_len(out: &mut String, len: usize) {
    out.push(':');
    out.push_str(&len.to_string());
    out.push(':');
}

const fn bool_text(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}
