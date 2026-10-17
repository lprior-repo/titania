use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{error::KaniInventoryError, proof_id::KaniHarnessId};

pub const KANI_INVENTORY_MAX_HARNESSES: usize = 1_000_000;

const KANI_HARNESS_PREFIX: &str = "kani::";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KaniHarnessListing {
    pub qualified_name: String,
    pub source_file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_id: Option<KaniHarnessId>,
    pub is_contract: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct KaniInventory {
    #[serde(default, rename = "kani-version", skip_serializing_if = "Option::is_none")]
    pub kani_version: Option<String>,
    #[serde(default, rename = "file-version", skip_serializing_if = "Option::is_none")]
    pub file_version: Option<String>,
    #[serde(default, rename = "standard-harnesses", skip_serializing_if = "Vec::is_empty")]
    pub standard_harnesses: Vec<KaniHarnessListing>,
    #[serde(default, rename = "contract-harnesses", skip_serializing_if = "Vec::is_empty")]
    pub contract_harnesses: Vec<KaniHarnessListing>,
}

impl KaniInventory {
    pub fn parse_str(contents: &str, path: &str) -> Result<Self, KaniInventoryError> {
        let wire: KaniInventoryWire = parse_wire(contents, path)?;
        let mut inventory = Self {
            kani_version: wire.kani_version,
            file_version: wire.file_version,
            ..Self::default()
        };
        flatten_harness_map(&wire.standard_harnesses, false, &mut inventory.standard_harnesses);
        flatten_harness_map(&wire.contract_harnesses, true, &mut inventory.contract_harnesses);
        let total =
            inventory.standard_harnesses.len().saturating_add(inventory.contract_harnesses.len());
        reject_too_many_harnesses(total, Box::from(path))?;
        Ok(inventory)
    }

    #[must_use]
    pub fn total(&self) -> usize {
        self.standard_harnesses.len().saturating_add(self.contract_harnesses.len())
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.standard_harnesses.is_empty() && self.contract_harnesses.is_empty()
    }
}

#[derive(Deserialize)]
struct KaniInventoryWire {
    #[serde(default, rename = "kani-version")]
    kani_version: Option<String>,
    #[serde(default, rename = "file-version")]
    file_version: Option<String>,
    #[serde(default, rename = "standard-harnesses")]
    standard_harnesses: BTreeMap<String, Vec<String>>,
    #[serde(default, rename = "contract-harnesses")]
    contract_harnesses: BTreeMap<String, Vec<String>>,
}

fn parse_wire(contents: &str, path_label: &str) -> Result<KaniInventoryWire, KaniInventoryError> {
    serde_json::from_str(contents).map_err(|error| KaniInventoryError::JsonParse {
        path: Box::from(path_label),
        reason: error.to_string().into_boxed_str(),
    })
}

fn reject_too_many_harnesses(total: usize, path: Box<str>) -> Result<(), KaniInventoryError> {
    if total > KANI_INVENTORY_MAX_HARNESSES {
        return Err(KaniInventoryError::TooManyHarnesses {
            path,
            found: total,
            max: KANI_INVENTORY_MAX_HARNESSES,
        });
    }
    Ok(())
}

fn flatten_harness_map(
    map: &BTreeMap<String, Vec<String>>,
    is_contract: bool,
    out: &mut Vec<KaniHarnessListing>,
) {
    out.extend(map.iter().flat_map(|(source_file, harnesses)| {
        harnesses
            .iter()
            .map(|qualified_name| listing_from_wire(qualified_name, source_file, is_contract))
    }));
}

fn listing_from_wire(
    qualified_name: &str,
    source_file: &str,
    is_contract: bool,
) -> KaniHarnessListing {
    KaniHarnessListing {
        qualified_name: qualified_name.to_owned(),
        source_file: source_file.to_owned(),
        canonical_id: canonical_harness_id(qualified_name),
        is_contract,
    }
}

#[must_use]
pub fn canonical_harness_id(qualified_name: &str) -> Option<KaniHarnessId> {
    let stripped: &str =
        qualified_name.strip_prefix(KANI_HARNESS_PREFIX).map_or(qualified_name, |rest| rest);
    if stripped.is_empty() {
        return None;
    }
    let canonical: String =
        stripped.chars().map(ascii_upper_or_pass).map(collapse_to_id_byte).collect();
    KaniHarnessId::new(&canonical).ok()
}

const fn ascii_upper_or_pass(character: char) -> char {
    if character.is_ascii_lowercase() { character.to_ascii_uppercase() } else { character }
}

const fn collapse_to_id_byte(character: char) -> char {
    if character.is_ascii_uppercase() || character.is_ascii_digit() { character } else { '_' }
}
