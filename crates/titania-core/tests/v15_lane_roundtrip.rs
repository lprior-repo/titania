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
fn gate_scope_full_is_not_parseable_in_v1() {
    assert!("full".parse::<GateScope>().is_err(), "v1 must not parse a full scope");
}

#[test]
fn gate_scope_v1_scopes_exclude_kani_and_mutants() {
    for scope in [GateScope::Edit, GateScope::Prepush, GateScope::Release] {
        assert!(!scope.lanes().contains(&Lane::Kani), "{scope:?} must not include Lane::Kani");
        assert!(
            !scope.lanes().contains(&Lane::Mutants),
            "{scope:?} must not include Lane::Mutants"
        );
    }
}

#[test]
fn gate_scope_release_does_not_include_kani_or_mutants() {
    let release = GateScope::Release.lanes();
    assert!(!release.contains(&Lane::Kani), "Release must not include Lane::Kani");
    assert!(!release.contains(&Lane::Mutants), "Release must not include Lane::Mutants");
}
