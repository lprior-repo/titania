//! v1.5 round-trip tests for `Lane::Kani` / `Lane::Mutants` and `GateScope::Full`.

use std::str::FromStr;

use titania_core::{GateScope, Lane};

#[test]
fn lane_kani_round_trip() {
    assert_eq!(Lane::Kani.to_string(), "Kani");
    assert_eq!(Lane::Kani.name(), "Kani");
    assert_eq!(Lane::Kani.file_stem(), "kani");
    assert_eq!(Lane::from_str("Kani"), Ok(Lane::Kani));
}

#[test]
fn lane_mutants_round_trip() {
    assert_eq!(Lane::Mutants.to_string(), "Mutants");
    assert_eq!(Lane::Mutants.name(), "Mutants");
    assert_eq!(Lane::Mutants.file_stem(), "mutants");
    assert_eq!(Lane::from_str("Mutants"), Ok(Lane::Mutants));
}

#[test]
fn lane_serde_round_trip_for_new_variants() {
    let kani = serde_json::to_string(&Lane::Kani).unwrap();
    let mutants = serde_json::to_string(&Lane::Mutants).unwrap();
    assert_eq!(kani, "\"Kani\"");
    assert_eq!(mutants, "\"Mutants\"");
    assert_eq!(serde_json::from_str::<Lane>(&kani).unwrap(), Lane::Kani);
    assert_eq!(serde_json::from_str::<Lane>(&mutants).unwrap(), Lane::Mutants);
}

#[test]
#[allow(let_underscore_drop)]
fn gate_scope_full_round_trip() {
    assert_eq!(GateScope::from_str("full"), Ok(GateScope::Full));
}

#[test]
fn gate_scope_full_includes_kani_and_mutants() {
    let lanes = GateScope::Full.lanes();
    assert!(lanes.contains(&Lane::Kani), "GateScope::Full must include Lane::Kani");
    assert!(lanes.contains(&Lane::Mutants), "GateScope::Full must include Lane::Mutants");
    for rel in GateScope::Release.lanes() {
        assert!(lanes.contains(rel), "GateScope::Full must include {rel:?} from Release");
    }
}

#[test]
fn gate_scope_release_does_not_include_kani_or_mutants() {
    let release = GateScope::Release.lanes();
    assert!(!release.contains(&Lane::Kani), "Release must not include Lane::Kani");
    assert!(!release.contains(&Lane::Mutants), "Release must not include Lane::Mutants");
}
