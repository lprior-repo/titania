//! Production-bound behavior checks for the receipt domain (tn-c6n).
//!
//! Every check calls a production constructor directly; no mirrors, no
//! reimplementations. Exact error variants and messages throughout.

use std::error::Error;

use titania_core::{
    Digest, GateScope, LaneDigest, LaneName, QualityReceipt, RECEIPT_ENVELOPE_SCHEMA_VERSION,
    ReceiptDigests, ReceiptEnvelope, ReceiptError, ReceiptLaneExit, ReceiptPeriod,
    RecordedTargetRoot, TargetProject,
};

type TestResult = Result<(), Box<dyn Error>>;

fn other(message: &str) -> Box<dyn Error> {
    std::io::Error::other(message).into()
}

fn rejected<T>(
    result: Result<T, ReceiptError>,
    message: &str,
) -> Result<ReceiptError, Box<dyn Error>> {
    result.err().ok_or_else(|| other(message))
}

fn envelope_json() -> Result<serde_json::Value, Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    std::fs::write(temp.path().join("Cargo.toml"), "[package]\nname = \"demo\"\n")?;
    let utf8 =
        camino::Utf8Path::from_path(temp.path()).ok_or_else(|| other("tempdir not UTF-8"))?;
    let target = TargetProject::try_from_path_string(utf8)?;
    let lane = LaneDigest::new(LaneName::new("fmt")?, ReceiptLaneExit::Clean, 3, 3, 0)?;
    let digests = ReceiptDigests::new(
        Digest::from_bytes(b"source"),
        Digest::from_bytes(b"lock"),
        Digest::from_bytes(b"policy"),
        Digest::from_bytes(b"toolchain"),
    );
    let envelope = ReceiptEnvelope::new(&target, ReceiptPeriod::new(10, 12)?, vec![lane], digests)?;
    Ok(serde_json::to_value(&envelope)?)
}

#[test]
fn lane_name_rejects_empty_with_empty_lane_name() -> TestResult {
    let err = rejected(LaneName::new(""), "empty lane name was accepted")?;
    assert!(matches!(err, ReceiptError::EmptyLaneName));
    assert_eq!(err.to_string(), "lane name must not be empty");
    Ok(())
}

#[test]
fn lane_name_rejects_nul_with_invalid_lane_name() -> TestResult {
    let err = rejected(LaneName::new("a\0b"), "NUL lane name was accepted")?;
    assert!(matches!(err, ReceiptError::InvalidLaneName));
    assert_eq!(err.to_string(), "lane name must not contain NUL bytes");
    let hostile = serde_json::json!({
        "lane": "x\0y", "exit": "clean", "scanned": 1_u32, "passed": 1_u32, "finding_count": 0_u32,
    });
    let err = serde_json::from_value::<LaneDigest>(hostile)
        .err()
        .ok_or_else(|| other("NUL lane name JSON was accepted"))?;
    assert!(err.to_string().contains("NUL"), "unexpected error: {err}");
    Ok(())
}

#[test]
fn lane_name_preserves_valid_through_accessors_and_json() -> TestResult {
    let lane = LaneName::new("fmt").map_err(|err| other(&err.to_string()))?;
    assert_eq!(lane.as_str(), "fmt");
    assert_eq!(lane.to_string(), "fmt");
    let back: LaneName = serde_json::from_str(&serde_json::to_string(&lane)?)?;
    assert_eq!(back, lane);
    Ok(())
}

#[test]
fn lane_digest_rejects_passed_above_scanned_constructor() -> TestResult {
    let err = rejected(
        LaneDigest::new(LaneName::new("clippy")?, ReceiptLaneExit::Violations, 1, 2, 1),
        "passed-above-scanned digest was accepted",
    )?;
    assert!(matches!(err, ReceiptError::PassedExceedsScanned { passed: 2, scanned: 1 }));
    assert_eq!(err.to_string(), "lane passed count 2 exceeds scanned count 1");
    Ok(())
}

#[test]
fn lane_digest_accepts_equal_and_zero_boundaries() -> TestResult {
    let full = LaneDigest::new(LaneName::new("fmt")?, ReceiptLaneExit::Clean, 3, 3, 0)
        .map_err(|err| other(&err.to_string()))?;
    assert_eq!((full.scanned(), full.passed()), (3, 3));
    let zero = LaneDigest::new(LaneName::new("fmt")?, ReceiptLaneExit::Failure, 0, 0, 0)
        .map_err(|err| other(&err.to_string()))?;
    assert_eq!((zero.scanned(), zero.passed()), (0, 0));
    Ok(())
}

#[test]
fn lane_digest_json_rejects_passed_above_scanned() -> TestResult {
    let hostile = serde_json::json!({
        "lane": "fmt", "exit": "clean", "scanned": 1_u32, "passed": 2_u32, "finding_count": 0_u32,
    });
    let err = serde_json::from_value::<LaneDigest>(hostile)
        .err()
        .ok_or_else(|| other("passed-above-scanned JSON was accepted"))?;
    assert!(err.to_string().contains("lane passed count 2 exceeds scanned count 1"));
    Ok(())
}

#[test]
fn receipt_error_match_is_exhaustive_with_stable_messages() -> TestResult {
    let scope: GateScope = "edit".parse::<GateScope>().map_err(|err| other(&err.to_string()))?;
    let digests = ReceiptDigests::new(
        Digest::from_bytes(b"source"),
        Digest::from_bytes(b"lock"),
        Digest::from_bytes(b"policy"),
        Digest::from_bytes(b"toolchain"),
    );
    let cases: [(ReceiptError, &str); 9] = [
        (ReceiptError::UnsupportedSchemaVersion(1), "unsupported receipt schema version 1"),
        (rejected(LaneName::new(""), "empty accepted")?, "lane name must not be empty"),
        (rejected(LaneName::new("a\0b"), "NUL accepted")?, "lane name must not contain NUL bytes"),
        (
            rejected(
                LaneDigest::new(LaneName::new("c")?, ReceiptLaneExit::Clean, 1, 2, 0),
                "counts accepted",
            )?,
            "lane passed count 2 exceeds scanned count 1",
        ),
        (
            rejected(ReceiptPeriod::new(12, 10), "reversed stamps accepted")?,
            "receipt finished_at 10 is before started_at 12",
        ),
        (
            rejected(
                QualityReceipt::new(scope, digests, Vec::new().into_boxed_slice()),
                "empty lane list accepted",
            )?,
            "quality receipt must include at least one lane receipt",
        ),
        (
            rejected(RecordedTargetRoot::new(""), "empty root accepted")?,
            "receipt target_root must not be empty",
        ),
        (
            rejected(RecordedTargetRoot::new("relative/path"), "relative root accepted")?,
            "receipt target_root must be absolute, got \"relative/path\"",
        ),
        (
            rejected(RecordedTargetRoot::new("/ab\0c"), "NUL root accepted")?,
            "receipt target_root must not contain NUL bytes",
        ),
    ];
    for (err, expected) in cases {
        let rendered = match &err {
            ReceiptError::UnsupportedSchemaVersion(version) => {
                format!("unsupported receipt schema version {version}")
            }
            ReceiptError::EmptyLaneName => "lane name must not be empty".to_owned(),
            ReceiptError::InvalidLaneName => "lane name must not contain NUL bytes".to_owned(),
            ReceiptError::PassedExceedsScanned { passed, scanned } => {
                format!("lane passed count {passed} exceeds scanned count {scanned}")
            }
            ReceiptError::FinishedBeforeStarted { started_at, finished_at } => {
                format!("receipt finished_at {finished_at} is before started_at {started_at}")
            }
            ReceiptError::EmptyLaneReceiptList => {
                "quality receipt must include at least one lane receipt".to_owned()
            }
            ReceiptError::TargetRootEmpty => "receipt target_root must not be empty".to_owned(),
            ReceiptError::TargetRootNonAbsolute(path) => {
                format!("receipt target_root must be absolute, got {path:?}")
            }
            ReceiptError::TargetRootContainsNul => {
                "receipt target_root must not contain NUL bytes".to_owned()
            }
        };
        assert_eq!(rendered, expected);
        assert_eq!(err.to_string(), expected);
    }
    Ok(())
}

#[test]
fn receipt_period_accepts_equal_stamps() -> TestResult {
    let period = ReceiptPeriod::new(12, 12).map_err(|err| other(&err.to_string()))?;
    assert_eq!((period.started_at(), period.finished_at()), (12, 12));
    Ok(())
}

#[test]
fn receipt_period_rejects_finish_before_start() -> TestResult {
    let err = rejected(ReceiptPeriod::new(12, 10), "reversed stamps were accepted")?;
    assert!(matches!(err, ReceiptError::FinishedBeforeStarted { started_at: 12, finished_at: 10 }));
    Ok(())
}

#[test]
fn recorded_root_rejects_empty_relative_nul() -> TestResult {
    assert!(matches!(
        rejected(RecordedTargetRoot::new(""), "empty root was accepted")?,
        ReceiptError::TargetRootEmpty
    ));
    assert!(matches!(
        rejected(RecordedTargetRoot::new("relative/path"), "relative root was accepted")?,
        ReceiptError::TargetRootNonAbsolute(_)
    ));
    assert!(matches!(
        rejected(RecordedTargetRoot::new("/ab\0c"), "NUL root was accepted")?,
        ReceiptError::TargetRootContainsNul
    ));
    Ok(())
}

#[test]
fn recorded_root_manifest_path_joins_cargo_toml() -> TestResult {
    let root = RecordedTargetRoot::new("/proj").map_err(|err| other(&err.to_string()))?;
    assert_eq!(root.as_str(), "/proj");
    assert_eq!(root.manifest_path().as_str(), "/proj/Cargo.toml");
    Ok(())
}

#[test]
fn receipt_exit_round_trips_all_variants() -> TestResult {
    for exit in [
        ReceiptLaneExit::Clean,
        ReceiptLaneExit::Violations,
        ReceiptLaneExit::Usage,
        ReceiptLaneExit::Failure,
    ] {
        let digest = LaneDigest::new(LaneName::new("fmt")?, exit, 2, 2, 1)
            .map_err(|err| other(&err.to_string()))?;
        let json = serde_json::to_string(&digest)?;
        let back: LaneDigest = serde_json::from_str(&json)?;
        assert_eq!(back.exit(), exit);
    }
    assert!(serde_json::to_string(&ReceiptLaneExit::Violations)?.contains("violations"));
    Ok(())
}

#[test]
fn schema_gate_accepts_v2_envelope() -> TestResult {
    assert_eq!(RECEIPT_ENVELOPE_SCHEMA_VERSION, 2);
    let value = envelope_json()?;
    let envelope: ReceiptEnvelope = serde_json::from_value(value)?;
    assert_eq!(envelope.schema_version(), 2);
    Ok(())
}

#[test]
fn envelope_rejects_non_v2_schema_versions() -> TestResult {
    for version in [0_u32, 1, 3, u32::MAX] {
        let mut value = envelope_json()?;
        value["schema_version"] = serde_json::Value::from(version);
        let err = serde_json::from_value::<ReceiptEnvelope>(value)
            .err()
            .ok_or_else(|| other("stale schema version was accepted"))?;
        assert!(
            err.to_string().contains(&format!("unsupported receipt schema version {version}")),
            "unexpected error for v{version}: {err}"
        );
    }
    Ok(())
}
