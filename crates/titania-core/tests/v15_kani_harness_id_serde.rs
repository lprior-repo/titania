//! v1.5 serde round-trip tests for `KaniHarnessId`.

use titania_core::KaniHarnessId;

#[test]
fn serde_roundtrip_preserves_inner_string() {
    let id = KaniHarnessId::new("FooBar").unwrap();
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, "\"FooBar\"");
    let parsed: KaniHarnessId = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, id);
}

#[test]
fn serde_roundtrip_preserves_uppercase() {
    let id = KaniHarnessId::new("FOO_BAR").unwrap();
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, "\"FOO_BAR\"");
    let parsed: KaniHarnessId = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, id);
}

#[test]
fn serde_rejects_invalid_strings() {
    let err = serde_json::from_str::<KaniHarnessId>("\"1Foo\"").unwrap_err();
    assert!(err.to_string().to_lowercase().contains("kani harness id"));
}

#[test]
fn serde_rejects_empty_string() {
    let err = serde_json::from_str::<KaniHarnessId>("\"\"").unwrap_err();
    assert!(err.to_string().to_lowercase().contains("kani harness id"));
}

#[test]
fn serde_rejects_leading_digit() {
    let err = serde_json::from_str::<KaniHarnessId>("\"1Foo\"").unwrap_err();
    assert!(err.to_string().to_lowercase().contains("kani harness id"));
}
