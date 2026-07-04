//! Public API tests for v1 `QualityReceipt` JSON contract.
//!
//! These tests assert the v1 schema one produced by bead tn-e0r.1:
//!   1. `schema_version` is 1 and the serialized JSON has exactly the
//!      v1 fields (scope, source_digest, cargo_lock_digest, policy_digest,
//!      toolchain_digest, lanes) with no legacy v2 fields.
//!   2. A schema-version-2 envelope does not deserialise as v1.

use std::error::Error;

use titania_core::{Digest, GateScope, Lane, LaneReceipt, QualityReceipt, ReceiptDigests};

type TestResult = Result<(), Box<dyn Error>>;

fn digest(seed: &'static [u8]) -> Digest {
    Digest::from_bytes(seed)
}

/// Assert that a v1 `QualityReceipt` serialises to JSON whose top-level
/// keys are **exactly** the v1 field names and that `schema_version` is 1.
#[test]
fn quality_receipt_v1_schema_version_is_one_and_fields_exact() -> TestResult {
    let receipt = QualityReceipt::new(
        GateScope::Edit,
        ReceiptDigests::new(
            digest(b"source"),
            digest(b"lock"),
            digest(b"policy"),
            digest(b"toolchain"),
        ),
        Box::new([LaneReceipt::new(Lane::Fmt, digest(b"evidence"), true)]),
    )?;

    let json = serde_json::to_string(&receipt)?;
    let obj = serde_json::from_str::<serde_json::Value>(&json)?;
    let obj = obj.as_object().expect("root must be a JSON object");

    // 1. schema_version must be 1
    assert_eq!(
        obj.get("schema_version").and_then(|v| v.as_u64()),
        Some(1),
        "schema_version must be 1 (v1)"
    );

    // 2. Exactly the v1 field set — no more, no less.
    let expected_keys = [
        "schema_version",
        "scope",
        "source_digest",
        "cargo_lock_digest",
        "policy_digest",
        "toolchain_digest",
        "lanes",
    ];

    let actual_keys: Vec<&str> = obj.keys().map(String::as_str).collect();
    let mut expected_sorted = expected_keys;
    expected_sorted.sort();
    let mut actual_sorted = actual_keys.clone();
    actual_sorted.sort();

    assert_eq!(
        actual_sorted, expected_sorted,
        "v1 JSON fields must be exactly {:?}; got {:?}",
        expected_keys, actual_keys
    );

    // 3. Reject legacy v2 fields that MUST NOT appear.
    for legacy_key in ["target_root", "started_at", "finished_at", "lane_results", "lock_digest"] {
        assert!(
            !obj.contains_key(legacy_key),
            "v1 JSON must not contain legacy v2 field '{legacy_key}'; found in {json}"
        );
    }

    // 4. Lane inner shape: each lane entry has exactly lane, evidence_digest, clean.
    let lanes = obj.get("lanes").and_then(|v| v.as_array()).expect("lanes must be an array");
    assert_eq!(lanes.len(), 1, "expected exactly one lane entry");
    let lane_obj = lanes[0].as_object().expect("lane entry must be an object");
    let lane_keys: Vec<&str> = lane_obj.keys().map(String::as_str).collect();
    let mut lane_keys_sorted = lane_keys;
    lane_keys_sorted.sort();
    assert_eq!(
        lane_keys_sorted,
        ["clean", "evidence_digest", "lane"],
        "LaneReceipt must have exactly [clean, evidence_digest, lane]"
    );

    // 5. scope must serialise as the v1 enum wire name "Edit"
    assert_eq!(
        obj.get("scope").and_then(|v| v.as_str()),
        Some("Edit"),
        "GateScope::Edit must serialise as \"Edit\""
    );

    Ok(())
}

/// A JSON payload with schema_version 2 must fail to deserialise into
/// the v1 `QualityReceipt` because v1 expects `schema_version` as u16 = 1
/// and does not recognise the v2 field layout.
#[test]
fn quality_receipt_v1_rejects_schema_version_two() -> TestResult {
    // Valid v1-shaped JSON, changing ONLY schema_version from 1 to 2.
    // All v1 fields are present with valid values; no legacy fields.
    let v2_json = r#"
    {
        "schema_version": 2,
        "scope": "Edit",
        "source_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "cargo_lock_digest": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "policy_digest": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        "toolchain_digest": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
        "lanes": [
            {
                "lane": "Fmt",
                "evidence_digest": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
                "clean": true
            }
        ]
    }"#;

    let err = serde_json::from_str::<QualityReceipt>(v2_json)
        .err()
        .expect("v2 JSON must fail to deserialise as v1 QualityReceipt");

    let msg = err.to_string();
    assert!(
        msg.contains("unsupported schema version") && msg.contains("got 2"),
        "deserialisation failure must indicate schema version mismatch; got: {msg}"
    );

    Ok(())
}
