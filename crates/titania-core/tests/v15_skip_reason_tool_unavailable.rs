//! v1.5 contract tests for `SkipReason::ToolUnavailable(ToolKind)` serde round-trip.

use titania_core::{SkipReason, ToolKind};

#[test]
fn skip_reason_serde_round_trip_tool_unavailable_cargo_kani() {
    let reason = SkipReason::ToolUnavailable(ToolKind::CargoKani);
    let json = serde_json::to_string(&reason).unwrap();
    let back: SkipReason = serde_json::from_str(&json).unwrap();
    assert_eq!(back, reason);
}

#[test]
fn skip_reason_serde_round_trip_tool_unavailable_cargo_mutants() {
    let reason = SkipReason::ToolUnavailable(ToolKind::CargoMutants);
    let json = serde_json::to_string(&reason).unwrap();
    let back: SkipReason = serde_json::from_str(&json).unwrap();
    assert_eq!(back, reason);
}

#[test]
fn skip_reason_tool_unavailable_uses_object_payload() {
    let json = serde_json::to_string(&SkipReason::ToolUnavailable(ToolKind::CargoKani)).unwrap();
    assert_eq!(json, r#"{"ToolUnavailable":"cargo-kani"}"#);
    let json = serde_json::to_string(&SkipReason::ToolUnavailable(ToolKind::CargoMutants)).unwrap();
    assert_eq!(json, r#"{"ToolUnavailable":"cargo-mutants"}"#);
}

#[test]
fn skip_reason_tool_unavailable_deserializes_from_object_payload() {
    let parsed: SkipReason = serde_json::from_str(r#"{"ToolUnavailable":"cargo-kani"}"#).unwrap();
    assert_eq!(parsed, SkipReason::ToolUnavailable(ToolKind::CargoKani));
    let parsed: SkipReason =
        serde_json::from_str(r#"{"ToolUnavailable":"cargo-mutants"}"#).unwrap();
    assert_eq!(parsed, SkipReason::ToolUnavailable(ToolKind::CargoMutants));
}

#[test]
fn skip_reason_not_applicable_round_trip_is_preserved() {
    let reason = SkipReason::NotApplicable;
    let json = serde_json::to_string(&reason).unwrap();
    let back: SkipReason = serde_json::from_str(&json).unwrap();
    assert_eq!(back, reason);
}

#[test]
fn tool_kind_serde_uses_kebab_case() {
    assert_eq!(serde_json::to_string(&ToolKind::CargoKani).unwrap(), "\"cargo-kani\"");
    assert_eq!(serde_json::to_string(&ToolKind::CargoMutants).unwrap(), "\"cargo-mutants\"");
}

#[test]
fn tool_kind_as_str_matches_serde_form() {
    assert_eq!(ToolKind::CargoKani.as_str(), "cargo-kani");
    assert_eq!(ToolKind::CargoMutants.as_str(), "cargo-mutants");
}

#[test]
fn tool_kind_round_trip_via_serde() {
    for kind in [ToolKind::CargoKani, ToolKind::CargoMutants] {
        let json = serde_json::to_string(&kind).unwrap();
        let back: ToolKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, kind);
    }
}
