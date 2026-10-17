//! Executable property checks for the receipt domain (tn-c6n).
//!
//! Each property asserts accept/reject equivalence between a production
//! constructor and its documented predicate over a full input class.

use std::collections::BTreeSet;

use proptest::prelude::*;
use titania_core::{
    Digest, LaneDigest, LaneName, RECEIPT_ENVELOPE_SCHEMA_VERSION, ReceiptDigests, ReceiptEnvelope,
    ReceiptError, ReceiptLaneExit, ReceiptPeriod, RecordedTargetRoot, TargetProject,
};

fn lane(name: &str) -> LaneName {
    LaneName::new(name).unwrap()
}

fn digests() -> ReceiptDigests {
    ReceiptDigests::new(
        Digest::from_bytes(b"source"),
        Digest::from_bytes(b"lock"),
        Digest::from_bytes(b"policy"),
        Digest::from_bytes(b"toolchain"),
    )
}

fn envelope_value_with_schema(version: u32) -> serde_json::Value {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("Cargo.toml"), "[package]\nname = \"demo\"\n").unwrap();
    let utf8 = camino::Utf8Path::from_path(temp.path()).unwrap();
    let target = TargetProject::try_from_path_string(utf8).unwrap();
    let digest = LaneDigest::new(lane("fmt"), ReceiptLaneExit::Clean, 3, 3, 0).unwrap();
    let envelope =
        ReceiptEnvelope::new(&target, ReceiptPeriod::new(10, 12).unwrap(), vec![digest], digests())
            .unwrap();
    let mut value = serde_json::to_value(&envelope).unwrap();
    value["schema_version"] = serde_json::Value::from(version);
    value
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    fn prop_lane_name_decides_validity(bytes in prop::collection::vec(any::<u8>(), 0..48)) {
        let candidate = String::from_utf8_lossy(&bytes).into_owned();
        let expected = !candidate.is_empty() && !candidate.contains('\0');
        prop_assert_eq!(LaneName::new(candidate.clone()).is_ok(), expected);
        if expected {
            let lane = LaneName::new(candidate.clone()).unwrap();
            prop_assert_eq!(lane.as_str(), candidate);
        } else if candidate.is_empty() {
            prop_assert!(matches!(
                LaneName::new(candidate).unwrap_err(),
                ReceiptError::EmptyLaneName
            ));
        } else {
            prop_assert!(matches!(
                LaneName::new(candidate).unwrap_err(),
                ReceiptError::InvalidLaneName
            ));
        }
    }

    #[test]
    fn prop_lane_digest_guards_counts(passed in any::<u32>(), scanned in any::<u32>()) {
        let result = LaneDigest::new(lane("fmt"), ReceiptLaneExit::Clean, scanned, passed, 0);
        prop_assert_eq!(result.is_ok(), passed <= scanned);
        match result {
            Ok(digest) => {
                prop_assert_eq!((digest.scanned(), digest.passed()), (scanned, passed));
            }
            Err(ReceiptError::PassedExceedsScanned { passed: p, scanned: s }) => {
                prop_assert_eq!((p, s), (passed, scanned));
            }
            Err(other) => panic!("wrong rejection for ({passed}, {scanned}): {other:?}"),
        }
    }

    #[test]
    fn prop_receipt_period_orders_stamps(started in any::<u64>(), finished in any::<u64>()) {
        let result = ReceiptPeriod::new(started, finished);
        prop_assert_eq!(result.is_ok(), finished >= started);
        match result {
            Ok(period) => {
                prop_assert_eq!((period.started_at(), period.finished_at()), (started, finished));
            }
            Err(ReceiptError::FinishedBeforeStarted { started_at, finished_at }) => {
                prop_assert_eq!((started_at, finished_at), (started, finished));
            }
            Err(other) => panic!("wrong rejection for ({started}, {finished}): {other:?}"),
        }
    }

    #[test]
    fn prop_recorded_root_decides_validity(text in any::<String>()) {
        let expected =
            !text.is_empty() && text.starts_with('/') && !text.contains('\0');
        let result = RecordedTargetRoot::new(text.clone());
        prop_assert_eq!(result.is_ok(), expected);
        if expected {
            let root = result.unwrap();
            prop_assert_eq!(root.as_str(), text);
        }
    }

    #[test]
    fn prop_schema_gate_rejects_other_versions(version in any::<u32>()) {
        prop_assert_eq!(RECEIPT_ENVELOPE_SCHEMA_VERSION, 2);
        let value = envelope_value_with_schema(version);
        let result = serde_json::from_value::<ReceiptEnvelope>(value);
        prop_assert_eq!(result.is_ok(), version == 2);
        if version != 2 {
            let message = result.unwrap_err().to_string();
            prop_assert!(
                message.contains(&format!("unsupported receipt schema version {version}")),
                "unexpected message: {message}"
            );
        }
    }
}

#[test]
fn error_messages_stable_across_all_variants() {
    let messages = [
        ReceiptError::UnsupportedSchemaVersion(1).to_string(),
        ReceiptError::EmptyLaneName.to_string(),
        ReceiptError::InvalidLaneName.to_string(),
        ReceiptError::PassedExceedsScanned { passed: 2, scanned: 1 }.to_string(),
        ReceiptError::FinishedBeforeStarted { started_at: 12, finished_at: 10 }.to_string(),
        ReceiptError::EmptyLaneReceiptList.to_string(),
        ReceiptError::TargetRootEmpty.to_string(),
        ReceiptError::TargetRootNonAbsolute("r".to_owned()).to_string(),
        ReceiptError::TargetRootContainsNul.to_string(),
    ];
    for message in &messages {
        assert!(!message.is_empty(), "empty error message");
    }
    let distinct: BTreeSet<&str> = messages.iter().map(String::as_str).collect();
    assert_eq!(distinct.len(), messages.len(), "ambiguous error messages");
}
