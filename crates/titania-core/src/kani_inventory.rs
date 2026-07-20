//! Pure-core parser for cargo-kani `kani-list.json` harness inventory.
//!
//! cargo-kani 0.67.0 writes the per-crate inventory to
//! `<crate_dir>/kani-list.json` after `cargo kani list --format json`.
//! The wire shape is documented by cargo-kani itself and looks like:
//!
//! ```json
//! {
//!   "kani-version": "0.67.0",
//!   "file-version": "0.1",
//!   "standard-harnesses": {
//!     "crates/<pkg>/src/kani.rs": [
//!       "kani::lane_digest_rejects_passed_greater_than_scanned",
//!       ...
//!     ]
//!   },
//!   "contract-harnesses": {},
//!   "contracts": [],
//!   "totals": { "standard-harnesses": 8, "contract-harnesses": 0, "functions-under-contract": 0 }
//! }
//! ```
//!
//! The parser only declares the fields v1.5 lane consumes; unknown
//! top-level keys (e.g. a future `checksum`, `build-flags`, etc.) are
//! ignored by serde's permissive default. The contract-harnesses and
//! standard-harnesses sections are flattened into a single ordered
//! listing so the lane can iterate without nested map walks.
//!
//! Each harness name is mapped to a canonical [`KaniHarnessId`] via
//! the same canonicalisation the lane applies: strip the
//! `kani::` prefix, uppercase ASCII letters, collapse
//! non-alphanumeric bytes to `_`. When the canonical form is
//! rejected by [`KaniHarnessId::new`] the listing still surfaces the
//! raw qualified name so the lane can fall back to the static
//! `PROOF_KANI_PASS` / `PROOF_KANI_FAIL` rule id family.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{error::KaniInventoryError, proof_id::KaniHarnessId};

/// Static upper bound on total harnesses per inventory file.
///
/// Bounded validation rejects pathologically large inputs before they
/// can exhaust the parser's allocation budget. The cap is generous
/// (one million) so a well-formed cargo-kani inventory never trips it;
/// in practice a single crate contributes fewer than a few hundred
/// harnesses.
pub const KANI_INVENTORY_MAX_HARNESSES: usize = 1_000_000;

/// `kani::` prefix cargo-kani stamps on every discovered harness name.
const KANI_HARNESS_PREFIX: &str = "kani::";

/// One harness entry surfaced by the inventory parser.
///
/// `qualified_name` is the raw string cargo-kani printed (e.g.
/// `"kani::lane_digest_rejects_passed_greater_than_scanned"`);
/// `canonical_id` is the result of running the same canonicalisation
/// the v1.5 Kani lane uses to build a `PROOF_KANI_<NAME>` rule id.
/// `None` means the canonical form was rejected by
/// [`KaniHarnessId::new`] (e.g. a non-ASCII byte or an empty body
/// after the `kani::` strip); the lane then falls back to a static
/// `PROOF_KANI_PASS` / `PROOF_KANI_FAIL` rule id literal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KaniHarnessListing {
    /// Raw cargo-kani qualified name, preserved verbatim.
    pub qualified_name: String,
    /// Source file path; the JSON map key under
    /// `standard-harnesses` / `contract-harnesses`.
    pub source_file: String,
    /// Canonical uppercase form parsed via [`KaniHarnessId::new`].
    ///
    /// `None` when the canonical form is rejected; the lane then uses
    /// the static `PROOF_KANI_*` fallback rule id family.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_id: Option<KaniHarnessId>,
    /// `true` when the entry was collected from the
    /// `contract-harnesses` section; `false` for
    /// `standard-harnesses`.
    pub is_contract: bool,
}

/// Typed kani-list.json contents.
///
/// Only the fields the v1.5 lane consumes are declared. Unknown
/// top-level keys (e.g. a future `checksum`) are accepted silently by
/// serde's permissive default. The flatten step is deterministic:
/// `BTreeMap` iterates the wire map in lexicographic key order so
/// `standard_harnesses` and `contract_harnesses` are stable across
/// runs.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct KaniInventory {
    /// `kani-version` metadata reported by cargo-kani (e.g.
    /// `"0.67.0"`). Tolerated as missing for forward compatibility
    /// with future cargo-kani revisions.
    #[serde(default, rename = "kani-version", skip_serializing_if = "Option::is_none")]
    pub kani_version: Option<String>,
    /// `file-version` metadata reported by cargo-kani (e.g.
    /// `"0.1"`). Tolerated as missing for forward compatibility.
    #[serde(default, rename = "file-version", skip_serializing_if = "Option::is_none")]
    pub file_version: Option<String>,
    /// All `standard-harnesses` entries, flattened into a typed list.
    /// Empty when the section is absent or empty.
    #[serde(default, rename = "standard-harnesses", skip_serializing_if = "Vec::is_empty")]
    pub standard_harnesses: Vec<KaniHarnessListing>,
    /// All `contract-harnesses` entries, flattened into a typed list.
    /// Empty when the section is absent or empty.
    #[serde(default, rename = "contract-harnesses", skip_serializing_if = "Vec::is_empty")]
    pub contract_harnesses: Vec<KaniHarnessListing>,
}

impl KaniInventory {
    /// Parse a kani inventory from a UTF-8 string slice.
    ///
    /// `path` is a caller-provided label (typically the on-disk path)
    /// used in [`KaniInventoryError`] diagnostics so the lane can
    /// surface structured reasons.
    ///
    /// # Errors
    /// - [`KaniInventoryError::JsonParse`] when `contents` is not
    ///   valid JSON or carries a non-array harness list.
    /// - [`KaniInventoryError::TooManyHarnesses`] when the flattened
    ///   harness count exceeds
    ///   [`KANI_INVENTORY_MAX_HARNESSES`].
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

    /// Total harness count (standard + contract).
    #[must_use]
    pub fn total(&self) -> usize {
        self.standard_harnesses.len().saturating_add(self.contract_harnesses.len())
    }

    /// True when no harnesses were discovered in either section.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.standard_harnesses.is_empty() && self.contract_harnesses.is_empty()
    }
}

/// Wire-only deserialise target.
///
/// Only the fields v1.5 consumes are declared so unknown keys are
/// ignored by serde's permissive default. The map type is `BTreeMap`
/// so flatten order is deterministic.
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

/// Run [`serde_json::from_str`] over the wire-only mirror and convert
/// any deserialise failure into a typed
/// [`KaniInventoryError::JsonParse`].
///
/// # Errors
/// - [`KaniInventoryError::JsonParse`] when serde rejects the input
///   as malformed JSON or the wire shape does not match the declared
///   fields.
fn parse_wire(contents: &str, path_label: &str) -> Result<KaniInventoryWire, KaniInventoryError> {
    serde_json::from_str(contents).map_err(|error| KaniInventoryError::JsonParse {
        path: Box::from(path_label),
        reason: error.to_string().into_boxed_str(),
    })
}

/// Enforce the per-file static harness cap. The cap is generous (one
/// million); a well-formed cargo-kani inventory never trips it.
///
/// # Errors
/// - [`KaniInventoryError::TooManyHarnesses`] when `total` exceeds
///   [`KANI_INVENTORY_MAX_HARNESSES`].
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

/// Flatten one `{file: [harness, ...]}` map into a typed listing.
///
/// `BTreeMap` iterates keys in lexicographic order so the resulting
/// listing is deterministic across runs (the lane depends on a stable
/// harness ordering for receipt diffs). The flat iterator chain keeps
/// the body under the strict [`clippy::excessive_nesting`]
/// threshold of two.
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

/// Build one [`KaniHarnessListing`] from the wire-form components.
///
/// Free function (rather than a struct-literal inside the for-loop
/// body) so the surrounding loop keeps the strict
/// [`clippy::excessive_nesting`] threshold of two.
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

/// Map a cargo-kani qualified name to its canonical
/// [`KaniHarnessId`].
///
/// Strips a `kani::` prefix when present, uppercases ASCII letters,
/// collapses every other byte to `_`, and finally validates the
/// canonical form via [`KaniHarnessId::new`]. Returns `None` for any
/// input whose canonical form is rejected by the newtype validator
/// (empty body, leading non-letter, non-ASCII body byte, or length
/// overflow).
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

/// Map an ASCII lowercase letter to its uppercase form; pass every
/// other byte through unchanged. The pass-through is intentional —
/// `collapse_to_id_byte` performs the allow-list filter.
const fn ascii_upper_or_pass(character: char) -> char {
    if character.is_ascii_lowercase() { character.to_ascii_uppercase() } else { character }
}

/// Collapse every byte that is not an uppercase ASCII letter, ASCII
/// digit, or underscore to `_`. Aligns with the
/// `^[A-Z][A-Z0-9_]*$` shape [`KaniHarnessId::new`] enforces.
const fn collapse_to_id_byte(character: char) -> char {
    if character.is_ascii_uppercase() || character.is_ascii_digit() { character } else { '_' }
}
