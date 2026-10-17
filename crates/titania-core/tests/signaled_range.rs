use serde_json::json;
use titania_core::{FailureError, LaneFailure, ProcessTermination};

#[test]
fn signaled_accepts_every_positive_value() {
    for signal in 1..=64 {
        assert_eq!(
            ProcessTermination::signaled(signal)
                .unwrap_or_else(|e| panic!("signal {signal} should be valid: {e}")),
            ProcessTermination::Signaled { signal },
            "valid signal {signal} did not round-trip to the same variant"
        );
    }
}

#[test]
fn signaled_accepts_positive_representatives() {
    assert_eq!(
        ProcessTermination::signaled(1).expect("SIGHUP must be valid"),
        ProcessTermination::Signaled { signal: 1 },
    );
    assert_eq!(
        ProcessTermination::signaled(31).expect("SIGSYS must be valid"),
        ProcessTermination::Signaled { signal: 31 },
    );
    assert_eq!(
        ProcessTermination::signaled(64).expect("SIGRTMAX default is valid"),
        ProcessTermination::Signaled { signal: 64 },
    );
    assert_eq!(
        ProcessTermination::signaled(i32::MAX).expect("i32::MAX is positive and thus valid"),
        ProcessTermination::Signaled { signal: i32::MAX },
    );
}

#[test]
fn signaled_rejects_zero_and_negatives() {
    assert!(ProcessTermination::signaled(0).is_err(), "signal 0 must be rejected");
    assert!(ProcessTermination::signaled(-1).is_err(), "signal -1 must be rejected");
    assert!(ProcessTermination::signaled(-9).is_err(), "signal -9 must be rejected");
    assert!(ProcessTermination::signaled(i32::MIN).is_err(), "signal i32::MIN must be rejected");
}

#[test]
fn signaled_error_carries_offending_value() {
    for &offending in &[0, -1, -9, i32::MIN] {
        let err =
            ProcessTermination::signaled(offending).expect_err("non-positive value must fail");
        assert!(
            matches!(err, FailureError::InvalidSignal(got) if got == offending),
            "expected InvalidSignal({offending}), got {err:?}"
        );
    }
}

#[test]
fn deserialize_rejects_signaled_zero() {
    let payload = json!({"Signaled": {"signal": 0}});
    let result = serde_json::from_value::<ProcessTermination>(payload);
    assert!(
        result.is_err(),
        "serde must not silently construct ProcessTermination::Signaled {{ signal: 0 }}; got: {result:?}"
    );
}

#[test]
fn deserialize_rejects_signaled_negative() {
    let payload = json!({"Signaled": {"signal": -1}});
    let result = serde_json::from_value::<ProcessTermination>(payload);
    assert!(
        result.is_err(),
        "serde must not silently construct ProcessTermination::Signaled {{ signal: -1 }}; got: {result:?}"
    );
}

#[test]
fn lane_failure_tool_deserialize_rejects_signaled_zero() {
    let payload = json!({
        "ToolFailure": {
            "tool": "cargo-clippy",
            "termination": {"Signaled": {"signal": 0}},
        }
    });
    let result = serde_json::from_value::<LaneFailure>(payload);
    assert!(
        result.is_err(),
        "LaneFailure::Tool with Signaled(0) must fail to deserialize; got: {result:?}"
    );
}
