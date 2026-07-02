#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_macros,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    clippy::missing_panics_doc,
    clippy::panic_in_result_fn,
    clippy::cognitive_complexity,
    clippy::doc_markdown,
    clippy::excessive_nesting,
    clippy::many_single_char_names,
    clippy::integer_division,
    clippy::integer_division_remainder_used,
    clippy::missing_errors_doc,
    clippy::type_complexity,
    clippy::map_identity,
    reason = "Tests are exempt from the strict production deny list per project doctrine."
)]
//! serde JSON round-trip and cross-primitive serialization tests.
//!
//! Behavior: any value built by a smart constructor should serialize to a
//! stable JSON string and deserialize back to an equal value.
//!
//! Test files are exempt from the strict production code lint policy.

#![allow(clippy::needless_borrow, reason = "Project doctrine: tests are exempt from production lint policy.")]
#![allow(clippy::useless_vec, reason = "Project doctrine: tests are exempt from production lint policy.")]

use titania_core::{Digest, RuleId, TextRange, WorkspacePath};

#[test]
fn digest_json_is_string_form() {
    let d = Digest::from_bytes(b"alpha");
    let v: serde_json::Value = serde_json::to_value(&d).unwrap();
    assert!(v.is_string(), "expected JSON string, got {v}");
    assert_eq!(v.as_str().unwrap().len(), 64);
}

#[test]
fn digest_round_trip_preserves_value() {
    let d = Digest::from_bytes(b"");
    let json = serde_json::to_string(&d).unwrap();
    let back: Digest = serde_json::from_str(&json).unwrap();
    assert_eq!(d, back);
}

#[test]
fn digest_deserialize_rejects_garbage() {
    let bad_strings: Vec<String> = vec![
        String::new(),
        "ab".to_string(),
        "deadbeef".to_string(),
        "Z".repeat(64),
        "g".repeat(64),
    ];
    for raw in &bad_strings {
        let json_input = serde_json::Value::String(raw.clone()).to_string();
        let result: Result<Digest, _> = serde_json::from_str(&json_input);
        assert!(result.is_err(), "should reject {json_input}");
    }
}

#[test]
fn rule_id_json_round_trip_preserves_value() {
    let id = RuleId::new("CLIPPY_UNWRAP_USED").unwrap();
    let v: serde_json::Value = serde_json::to_value(&id).unwrap();
    assert_eq!(v, serde_json::Value::String("CLIPPY_UNWRAP_USED".into()));
    let back: RuleId = serde_json::from_value(v).unwrap();
    assert_eq!(id, back);
}

#[test]
fn rule_id_deserialize_rejects_garbage() {
    let bad_strings: Vec<String> = vec![
        String::new(),
        "lowercase_input".to_string(),
        "no-leading-prefix-at-end".to_string(),
        "has-dash".to_string(),
    ];
    for raw in &bad_strings {
        let json_input = serde_json::Value::String(raw.clone()).to_string();
        let result: Result<RuleId, _> = serde_json::from_str(&json_input);
        assert!(result.is_err(), "should reject {json_input}");
    }
}

#[test]
fn workspace_path_json_round_trip_preserves_value() {
    let p = WorkspacePath::new("crates/titania-core/src/lib.rs").unwrap();
    let v: serde_json::Value = serde_json::to_value(&p).unwrap();
    assert_eq!(v, serde_json::Value::String("crates/titania-core/src/lib.rs".into()));
    let back: WorkspacePath = serde_json::from_value(v).unwrap();
    assert_eq!(p, back);
}

#[test]
fn workspace_path_deserialize_rejects_garbage() {
    let bad_strings: Vec<String> = vec![
        String::new(),
        "/abs/path".to_string(),
        "../etc/passwd".to_string(),
        "back\\slash".to_string(),
    ];
    for raw in &bad_strings {
        let json_input = serde_json::Value::String(raw.clone()).to_string();
        let result: Result<WorkspacePath, _> = serde_json::from_str(&json_input);
        assert!(result.is_err(), "should reject {json_input}");
    }
}

#[test]
fn text_range_json_round_trip_preserves_value() {
    let r = TextRange::new(42, 100).unwrap();
    let v: serde_json::Value = serde_json::to_value(r).unwrap();
    assert_eq!(v, serde_json::json!({"start_byte": 42_u32, "end_byte": 100_u32}));
    let back: TextRange = serde_json::from_value(v).unwrap();
    assert_eq!(r, back);
}

#[test]
fn text_range_deserialize_rejects_inverted() {
    let bad = r#"{"start_byte": 100, "end_byte": 42}"#;
    let result: Result<TextRange, _> = serde_json::from_str(bad);
    assert!(result.is_err());
}

#[test]
fn structured_value_serializes_deterministically() {
    let a = WorkspacePath::new("src/lib.rs").unwrap();
    let b = WorkspacePath::new("src/lib.rs").unwrap();
    assert_eq!(serde_json::to_string(&a).unwrap(), serde_json::to_string(&b).unwrap());

    let a = Digest::from_bytes(b"deterministic");
    let b = Digest::from_bytes(b"deterministic");
    assert_eq!(serde_json::to_string(&a).unwrap(), serde_json::to_string(&b).unwrap());
}
