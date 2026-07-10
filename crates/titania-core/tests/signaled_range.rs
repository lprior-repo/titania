//! Focused contract test for `ProcessTermination::signaled`.
//!
//! Pins the validator behaviour for the constructor:
//!
//! * any positive `i32` (including real-time signals >= 32 on glibc)
//!   must be accepted and round-trip through the `Signaled` variant,
//! * zero and any negative `i32` must be rejected with
//!   `FailureError::InvalidSignal` carrying the offending value,
//! * the typed error path `serde_json` routes through `signaled()` for
//!   `Signaled` deserialization, so a wire payload with `signal: 0`
//!   must FAIL deserialization (no bypass into the domain layer).
//!
//! These tests cover boundary behaviour only; they do not cover the
//! full receipt/replay pipeline owned by other tests.

use serde_json::json;
use titania_core::{FailureError, LaneFailure, ProcessTermination};

/// All positive `i32` values, including real-time signals, must build.
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

/// The Unix signal endpoints from the spec (SIGHUP=1, SIGSYS=31) plus a
/// real-time glibc signal must all round-trip identically.
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
    // glibc real-time signal example
    assert_eq!(
        ProcessTermination::signaled(64).expect("SIGRTMAX default is valid"),
        ProcessTermination::Signaled { signal: 64 },
    );
    assert_eq!(
        ProcessTermination::signaled(i32::MAX).expect("i32::MAX is positive and thus valid"),
        ProcessTermination::Signaled { signal: i32::MAX },
    );
}

/// Zero and every negative value must be rejected. Signal `0` is reserved
/// (`waitpid(2)` "no signal" sentinel); negative values are kernel-internal.
#[test]
fn signaled_rejects_zero_and_negatives() {
    assert!(ProcessTermination::signaled(0).is_err(), "signal 0 must be rejected");
    assert!(ProcessTermination::signaled(-1).is_err(), "signal -1 must be rejected");
    assert!(ProcessTermination::signaled(-9).is_err(), "signal -9 must be rejected");
    assert!(ProcessTermination::signaled(i32::MIN).is_err(), "signal i32::MIN must be rejected");
}

/// On rejection, the error variant MUST carry the offending signal value
/// so downstream callers can report it without losing information.
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

/// `#[derive(Deserialize)]` on `ProcessTermination` routes `Signaled`
/// payloads through `signaled()`. A wire `{"Signaled":{"signal":0}}` must
/// therefore FAIL to deserialize, closing the prior P2 bypass.
#[test]
fn deserialize_rejects_signaled_zero() {
    let payload = json!({"Signaled": {"signal": 0}});
    let result = serde_json::from_value::<ProcessTermination>(payload);
    assert!(
        result.is_err(),
        "serde must not silently construct ProcessTermination::Signaled {{ signal: 0 }}; got: {result:?}"
    );
}

/// Negative-signal wire payloads must also fail deserialization.
#[test]
fn deserialize_rejects_signaled_negative() {
    let payload = json!({"Signaled": {"signal": -1}});
    let result = serde_json::from_value::<ProcessTermination>(payload);
    assert!(
        result.is_err(),
        "serde must not silently construct ProcessTermination::Signaled {{ signal: -1 }}; got: {result:?}"
    );
}

/// The wire-side `LaneFailure::Tool` ingest path carries a
/// `ProcessTermination` and must surface an invalid-signal payload as a
/// serde error (not silently accept it).
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
