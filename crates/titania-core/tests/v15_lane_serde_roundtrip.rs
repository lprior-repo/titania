//! v1.5 contract tests for `Lane` serde round-trip across all 12 variants.

use std::str::FromStr;

use titania_core::Lane;

const ALL_LANES: &[Lane] = &[
    Lane::Fmt,
    Lane::Compile,
    Lane::Clippy,
    Lane::AstGrep,
    Lane::Dylint,
    Lane::PanicScan,
    Lane::PolicyScan,
    Lane::Test,
    Lane::Deny,
    Lane::Build,
    Lane::Kani,
    Lane::Mutants,
];

#[test]
fn serde_round_trip_all_variants() {
    for &lane in ALL_LANES {
        let json = serde_json::to_string(&lane).unwrap();
        let back: Lane = serde_json::from_str(&json).unwrap();
        assert_eq!(back, lane, "round-trip failed for {lane:?}");
    }
}

#[test]
fn name_from_str_round_trip_all_variants() {
    for &lane in ALL_LANES {
        let name = lane.name();
        let parsed = Lane::from_str(name).unwrap();
        assert_eq!(parsed, lane, "name round-trip failed for {name:?}");
    }
}

#[test]
fn serde_json_serializes_in_pascal_case() {
    assert_eq!(serde_json::to_string(&Lane::Kani).unwrap(), "\"Kani\"");
    assert_eq!(serde_json::to_string(&Lane::Mutants).unwrap(), "\"Mutants\"");
    assert_eq!(serde_json::to_string(&Lane::PolicyScan).unwrap(), "\"PolicyScan\"");
}

#[test]
fn serde_rejects_unknown_variant() {
    let err = serde_json::from_str::<Lane>("\"UnknownLane\"").unwrap_err();
    assert!(err.to_string().to_lowercase().contains("unknown"));
}
