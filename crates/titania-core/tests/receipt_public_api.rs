use std::error::Error;

use titania_core::{Digest, GateScope, Lane, LaneReceipt, QualityReceipt, ReceiptDigests};

type TestResult = Result<(), Box<dyn Error>>;

fn digest(seed: &'static [u8]) -> Digest {
    Digest::from_bytes(seed)
}

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

    assert_eq!(
        obj.get("schema_version").and_then(|v| v.as_u64()),
        Some(1),
        "schema_version must be 1 (v1)"
    );

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

    for legacy_key in ["target_root", "started_at", "finished_at", "lane_results", "lock_digest"] {
        assert!(
            !obj.contains_key(legacy_key),
            "v1 JSON must not contain legacy v2 field '{legacy_key}'; found in {json}"
        );
    }

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

    assert_eq!(
        obj.get("scope").and_then(|v| v.as_str()),
        Some("Edit"),
        "GateScope::Edit must serialise as \"Edit\""
    );

    Ok(())
}

#[test]
fn quality_receipt_v1_rejects_schema_version_two() -> TestResult {
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
