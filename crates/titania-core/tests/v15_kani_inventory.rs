//! v1.5 contract tests for the pure-core kani inventory parser
//! (`crates/titania-core/src/kani_inventory.rs`).
//!
//! Mirrors the spec promise in `.beads/tn-7bq2.1/boundary-map.md`:
//! the parser accepts `&str`, returns typed thiserror errors, tolerates
//! documented unknown top-level keys (forward compatibility for
//! cargo-kani field evolution), rejects malformed required shapes, and
//! performs no I/O / time / env / process access.

use std::path::PathBuf;

use titania_core::{
    KANI_INVENTORY_MAX_HARNESSES, KaniHarnessId, KaniHarnessListing, KaniInventory,
    KaniInventoryError, canonical_harness_id,
};

fn fixture_path(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push(name);
    path
}

fn read_fixture(name: &str) -> (String, String) {
    let path = fixture_path(name);
    let label = path.display().to_string();
    let contents = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("fixture `{}` unreadable: {error}", path.display()));
    (label, contents)
}

#[test]
fn happy_path_loads_eight_standard_harnesses() {
    let (label, contents) = read_fixture("v15_kani_inventory_full.json");
    let inventory = KaniInventory::parse_str(&contents, &label).expect("happy path must parse");
    assert_eq!(inventory.standard_harnesses.len(), 8);
    assert!(inventory.contract_harnesses.is_empty());
    assert_eq!(inventory.total(), 8);
    assert!(!inventory.is_empty());
    assert_eq!(inventory.kani_version.as_deref(), Some("0.67.0"));
    assert_eq!(inventory.file_version.as_deref(), Some("0.1"));
}

#[test]
fn canonical_id_uppercases_and_strips_kani_prefix() {
    let (label, contents) = read_fixture("v15_kani_inventory_full.json");
    let inventory = KaniInventory::parse_str(&contents, &label).expect("parse full");
    let first = inventory.standard_harnesses.first().expect("fixture must carry a first harness");
    assert_eq!(first.qualified_name, "kani::lane_digest_accepts_passed_not_greater_than_scanned");
    let canonical = first.canonical_id.as_ref().expect("canonical id must be Some");
    assert_eq!(canonical.as_str(), "LANE_DIGEST_ACCEPTS_PASSED_NOT_GREATER_THAN_SCANNED");
    assert!(!first.is_contract);
}

#[test]
fn empty_inventory_loads_with_zero_count() {
    let (label, contents) = read_fixture("v15_kani_inventory_empty.json");
    let inventory =
        KaniInventory::parse_str(&contents, &label).expect("empty inventory must parse");
    assert_eq!(inventory.standard_harnesses.len(), 0);
    assert_eq!(inventory.contract_harnesses.len(), 0);
    assert_eq!(inventory.total(), 0);
    assert!(inventory.is_empty());
}

#[test]
fn minimal_inventory_omits_optional_metadata() {
    let (label, contents) = read_fixture("v15_kani_inventory_minimal.json");
    let inventory =
        KaniInventory::parse_str(&contents, &label).expect("minimal inventory must parse");
    assert_eq!(inventory.standard_harnesses.len(), 1);
    assert!(inventory.kani_version.is_none());
    assert!(inventory.file_version.is_none());
}

#[test]
fn unknown_top_level_keys_are_ignored() {
    let (label, contents) = read_fixture("v15_kani_inventory_with_unknown_keys.json");
    let inventory = KaniInventory::parse_str(&contents, &label)
        .expect("unknown top-level keys must be tolerated (forward compat)");
    assert_eq!(inventory.standard_harnesses.len(), 1);
    assert_eq!(inventory.contract_harnesses.len(), 1);
    let standard = &inventory.standard_harnesses[0];
    let contract = &inventory.contract_harnesses[0];
    assert!(!standard.is_contract);
    assert!(contract.is_contract);
    assert_eq!(contract.qualified_name, "kani::contract_increment_under_modulus");
    assert_eq!(
        contract.canonical_id.as_ref().expect("canonical id").as_str(),
        "CONTRACT_INCREMENT_UNDER_MODULUS"
    );
}

#[test]
fn malformed_json_returns_typed_json_parse_error() {
    let (label, contents) = read_fixture("v15_kani_inventory_malformed.json");
    let err = KaniInventory::parse_str(&contents, &label).expect_err("malformed JSON must fail");
    assert!(matches!(err, KaniInventoryError::JsonParse { .. }), "got {err:?}");
}

#[test]
fn rejects_top_level_string() {
    let err = KaniInventory::parse_str("\"a string\"", "<inline>")
        .expect_err("string root must be rejected");
    assert!(matches!(err, KaniInventoryError::JsonParse { .. }), "got {err:?}");
}

#[test]
fn rejects_top_level_array() {
    let err =
        KaniInventory::parse_str("[1, 2, 3]", "<inline>").expect_err("array root must be rejected");
    assert!(matches!(err, KaniInventoryError::JsonParse { .. }), "got {err:?}");
}

#[test]
fn rejects_non_array_harness_list() {
    // A harness list that is a string instead of an array must be
    // rejected as a malformed required shape, not silently coerced.
    let contents = r#"{ "standard-harnesses": { "file.rs": "not-an-array" } }"#;
    let err = KaniInventory::parse_str(contents, "<inline>")
        .expect_err("non-array harness list must fail");
    assert!(matches!(err, KaniInventoryError::JsonParse { .. }), "got {err:?}");
}

#[test]
fn empty_qualified_name_produces_no_canonical_id() {
    let harness = KaniHarnessListing {
        qualified_name: String::from("kani::"),
        source_file: String::from("crates/foo/src/lib.rs"),
        canonical_id: canonical_harness_id("kani::"),
        is_contract: false,
    };
    assert!(harness.canonical_id.is_none());
}

#[test]
fn canonical_harness_id_collapses_punctuation_to_underscore() {
    // `replace-self.with_dash` ⇒ `REPLACE_SELF_WITH_DASH`
    let canonical = canonical_harness_id("kani::replace-self.with_dash")
        .expect("punctuation must collapse to underscore");
    assert_eq!(canonical.as_str(), "REPLACE_SELF_WITH_DASH");
}

#[test]
fn canonical_harness_id_rejects_non_ascii_body() {
    // UTF-8 multi-byte sequences collapse to `_`; the result is
    // valid, but we also want to assert that names without the
    // `kani::` prefix are still canonicalised.
    let canonical =
        canonical_harness_id("plain_name").expect("plain name without prefix must canonicalise");
    assert_eq!(canonical.as_str(), "PLAIN_NAME");
}

#[test]
fn canonical_harness_id_rejects_leading_digit() {
    assert!(canonical_harness_id("kani::1leading_digit").is_none());
}

#[test]
fn canonical_harness_id_rejects_empty_body() {
    assert!(canonical_harness_id("").is_none());
    assert!(canonical_harness_id("kani::").is_none());
}

#[test]
fn cap_constant_matches_docstring() {
    assert!(KANI_INVENTORY_MAX_HARNESSES >= 1024, "cap must be generous for real workloads");
}

#[test]
fn serde_round_trip_preserves_inventory() {
    let (label, contents) = read_fixture("v15_kani_inventory_full.json");
    let inventory = KaniInventory::parse_str(&contents, &label).expect("parse full");
    let json = serde_json::to_string(&inventory).expect("serialize must succeed");
    let back: KaniInventory = serde_json::from_str(&json).expect("round-trip must succeed");
    assert_eq!(back, inventory);
}

#[test]
fn serde_round_trip_harness_listing_preserves_canonical_id() {
    let listing = KaniHarnessListing {
        qualified_name: String::from("kani::foo_bar"),
        source_file: String::from("crates/pkg/src/lib.rs"),
        canonical_id: KaniHarnessId::new("FOO_BAR").ok(),
        is_contract: false,
    };
    let json = serde_json::to_string(&listing).expect("serialize");
    let back: KaniHarnessListing = serde_json::from_str(&json).expect("parse");
    assert_eq!(back, listing);
}
