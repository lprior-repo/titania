//! Wire-format invariant tests for [`LaneOutcome`] and its evidence types.
//!
//! These tests pin the deserialize-time invariants that the hand-written
//! `Deserialize` implementations now enforce on top of the smart constructors:
//!
//! - [`CommandEvidence`] cannot bypass `argv[0] == executable` via JSON.
//! - [`LaneEvidence`] cannot promote a non-success termination into clean
//!   evidence via JSON.
//! - [`LaneOutcome::Findings`] cannot deserialize an empty findings list into
//!   a vacuous pass.
//!
//! Round-trip tests prove the validated wire shapes still serialise back to
//! the same domain value, so the public wire format is unchanged.

#![allow(clippy::needless_borrow)]
#![allow(clippy::useless_vec)]

use titania_core::{
    CommandEvidence, Digest, Finding, Lane, LaneEvidence, LaneFailure, LaneOutcome, Location,
    OutcomeError, ProcessTermination, RepairHint, RuleId, SkipReason, WorkspacePath,
};

fn dig(seed: &'static [u8]) -> Digest {
    Digest::from_bytes(seed)
}

fn stub_finding() -> Finding {
    Finding::reject(
        Lane::AstGrep,
        RuleId::new("FUNC_LOOPS_FOR").expect("rule id"),
        Location::span(WorkspacePath::new("src/parser.rs").expect("path"), 42, 5, 42, 30)
            .expect("span"),
        String::from("Imperative for loop in production source"),
        RepairHint::use_iterator_pipeline(String::from("items.iter().map(|item| ...)")),
    )
}

fn stub_clean_evidence() -> LaneEvidence {
    let command = CommandEvidence::new(
        String::from("cargo"),
        vec![String::from("cargo"), String::from("fmt"), String::from("--check")]
            .into_boxed_slice(),
    )
    .expect("valid command evidence");
    LaneEvidence::new(
        command,
        String::from("rustfmt 1.84.0"),
        ProcessTermination::Exited { code: 0 },
        dig(b"clean-evidence-digest"),
    )
    .expect("clean evidence")
}

// ===========================================================================
// CommandEvidence wire invariants
// ===========================================================================

#[test]
fn command_evidence_round_trip_preserves_value() {
    let original = CommandEvidence::new(
        String::from("cargo"),
        vec![String::from("cargo"), String::from("fmt")].into_boxed_slice(),
    )
    .expect("valid command evidence");
    let json = serde_json::to_string(&original).expect("serialize");
    let back: CommandEvidence = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, original);
}

#[test]
fn command_evidence_deserialize_rejects_argv_zero_mismatch() {
    let json = r#"{"executable":"cargo","argv":["rustc","-V"]}"#;
    let result: Result<CommandEvidence, _> = serde_json::from_str(json);
    let err = result.expect_err("argv[0] != executable must be rejected on deserialize");
    assert!(err.to_string().contains("argv[0]"), "error should reference argv[0] mismatch: {err}");
}

#[test]
fn command_evidence_deserialize_rejects_empty_argv() {
    let json = r#"{"executable":"cargo","argv":[]}"#;
    let result: Result<CommandEvidence, _> = serde_json::from_str(json);
    let err = result.expect_err("empty argv must be rejected on deserialize");
    assert!(err.to_string().contains("argv"), "error should reference argv: {err}");
}

#[test]
fn command_evidence_deserialize_preserves_mode_field_when_present() {
    // Construct an embedded evidence so the mode round-trips.
    let original = CommandEvidence::embedded(
        String::from("cargo"),
        vec![String::from("cargo"), String::from("ast-grep"), String::from("scan")]
            .into_boxed_slice(),
    )
    .expect("valid embedded evidence");
    let value = serde_json::to_value(&original).expect("to_value");
    assert_eq!(value["mode"], "Embedded");
    let back: CommandEvidence = serde_json::from_value(value).expect("from_value");
    assert_eq!(back, original);
}

#[test]
fn command_evidence_deserialize_omits_mode_when_none() {
    let original = CommandEvidence::new(
        String::from("cargo"),
        vec![String::from("cargo"), String::from("fmt")].into_boxed_slice(),
    )
    .expect("valid command evidence");
    let value = serde_json::to_value(&original).expect("to_value");
    assert!(value.get("mode").is_none(), "ChildProcess / default must skip serializing mode field");
    let back: CommandEvidence = serde_json::from_value(value).expect("from_value");
    assert_eq!(back, original);
}

// ===========================================================================
// LaneEvidence wire invariants
// ===========================================================================

#[test]
fn lane_evidence_round_trip_preserves_value() {
    let original = stub_clean_evidence();
    let json = serde_json::to_string(&original).expect("serialize");
    let back: LaneEvidence = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, original);
}

#[test]
fn lane_evidence_deserialize_rejects_nonzero_exit() {
    let json = r#"{
        "command": {"executable":"cargo","argv":["cargo","fmt","--check"]},
        "tool_version":"rustfmt 1.84.0",
        "exit_status":{"Exited":{"code":1}},
        "parsed_result_digest":"44e86eb38ff9b065a1e805dd61f624e334a4296479e4be43c354ca8af90b0340"
    }"#;
    let result: Result<LaneEvidence, _> = serde_json::from_str(json);
    let err = result.expect_err("non-success exit must be rejected on deserialize");
    assert!(err.to_string().contains("exit status"), "error should reference exit status: {err}");
}

#[test]
fn lane_evidence_deserialize_rejects_signaled_exit() {
    let json = r#"{
        "command": {"executable":"cargo","argv":["cargo","test"]},
        "tool_version":"cargo 1.84.0",
        "exit_status":{"Signaled":{"signal":11}},
        "parsed_result_digest":"44e86eb38ff9b065a1e805dd61f624e334a4296479e4be43c354ca8af90b0340"
    }"#;
    let result: Result<LaneEvidence, _> = serde_json::from_str(json);
    let err = result.expect_err("signal termination must be rejected on deserialize");
    assert!(err.to_string().contains("exit status"), "error should reference exit status: {err}");
}

#[test]
fn lane_evidence_deserialize_rejects_argv_zero_mismatch_inside_clean() {
    // Crafted JSON where argv[0] does not match executable. The
    // LaneEvidence deserializer must reject it via the same
    // OutcomeError::Argv0Mismatch error path the constructor uses.
    let json = r#"{
        "command": {"executable":"cargo","argv":["rustc","-V"]},
        "tool_version":"rustc 1.84.0",
        "exit_status":{"Exited":{"code":0}},
        "parsed_result_digest":"44e86eb38ff9b065a1e805dd61f624e334a4296479e4be43c354ca8af90b0340"
    }"#;
    let result: Result<LaneEvidence, _> = serde_json::from_str(json);
    let err = result.expect_err("argv mismatch inside Clean must be rejected on deserialize");
    assert!(err.to_string().contains("argv[0]"), "error should reference argv[0] mismatch: {err}");
}

#[test]
fn lane_evidence_deserialize_rejects_empty_argv_inside_clean() {
    let json = r#"{
        "command": {"executable":"cargo","argv":[]},
        "tool_version":"cargo 1.84.0",
        "exit_status":{"Exited":{"code":0}},
        "parsed_result_digest":"44e86eb38ff9b065a1e805dd61f624e334a4296479e4be43c354ca8af90b0340"
    }"#;
    let result: Result<LaneEvidence, _> = serde_json::from_str(json);
    let err = result.expect_err("empty argv inside Clean must be rejected on deserialize");
    assert!(err.to_string().contains("argv"), "error should reference argv: {err}");
}

// ===========================================================================
// LaneOutcome::Findings wire invariants
// ===========================================================================

#[test]
fn lane_outcome_findings_round_trip_preserves_value() {
    let original = LaneOutcome::Findings { findings: Box::new([stub_finding()]) };
    let json = serde_json::to_string(&original).expect("serialize");
    let back: LaneOutcome = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, original);
}

#[test]
fn lane_outcome_findings_deserialize_rejects_empty_list() {
    let json = r#"{"Findings":[]}"#;
    let result: Result<LaneOutcome, _> = serde_json::from_str(json);
    let err = result.expect_err("empty findings list must be rejected on deserialize");
    assert!(
        err.to_string().contains("at least one finding"),
        "error should reference the empty-findings invariant: {err}"
    );
}

#[test]
fn lane_outcome_findings_empty_constructor_remains_allowed() {
    // The constructor path is unchanged: in-process construction of an
    // empty Findings variant remains the caller's responsibility. Only
    // wire deserialization rejects the vacuous-pass shape.
    let outcome = LaneOutcome::Findings { findings: Box::new([]) };
    assert!(matches!(outcome, LaneOutcome::Findings { .. }));
    assert_eq!(outcome.is_findings(), true);
}

// ===========================================================================
// LaneOutcome wire invariants across all variants
// ===========================================================================

#[test]
fn lane_outcome_clean_round_trip_preserves_value() {
    let original = LaneOutcome::Clean { evidence: stub_clean_evidence() };
    let json = serde_json::to_string(&original).expect("serialize");
    let back: LaneOutcome = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, original);
}

#[test]
fn lane_outcome_clean_deserialize_rejects_nonzero_exit() {
    // Clean variant with non-success exit must be rejected at deserialize.
    let json = r#"{
        "Clean": {
            "evidence": {
                "command": {"executable":"cargo","argv":["cargo","fmt","--check"]},
                "tool_version":"rustfmt 1.84.0",
                "exit_status":{"Exited":{"code":2}},
                "parsed_result_digest":"44e86eb38ff9b065a1e805dd61f624e334a4296479e4be43c354ca8af90b0340"
            }
        }
    }"#;
    let result: Result<LaneOutcome, _> = serde_json::from_str(json);
    let err = result.expect_err("Clean with non-zero exit must be rejected on deserialize");
    assert!(err.to_string().contains("exit status"), "error should reference exit status: {err}");
}

#[test]
fn lane_outcome_failed_round_trip_preserves_value() {
    let original = LaneOutcome::Failed {
        failure: LaneFailure::Infra {
            tool: String::from("cargo-dylint"),
            reason: String::from("subcommand unavailable"),
        },
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let back: LaneOutcome = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, original);
}

#[test]
fn lane_outcome_skipped_round_trip_preserves_value() {
    let original = LaneOutcome::Skipped { reason: SkipReason::NotApplicable };
    let json = serde_json::to_string(&original).expect("serialize");
    let back: LaneOutcome = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, original);
}

// ===========================================================================
// OutcomeError coverage — prove the typed error variants surface to callers.
// ===========================================================================

#[test]
fn outcome_error_empty_argv_message() {
    let err = OutcomeError::EmptyArgv;
    assert_eq!(err.to_string(), "argv must not be empty");
}

#[test]
fn outcome_error_argv0_mismatch_message() {
    let err = OutcomeError::Argv0Mismatch {
        expected: String::from("cargo"),
        found: String::from("rustc"),
    };
    let rendered = err.to_string();
    assert!(rendered.contains("argv[0]"), "{rendered}");
    assert!(rendered.contains("cargo"), "{rendered}");
    assert!(rendered.contains("rustc"), "{rendered}");
}

#[test]
fn outcome_error_non_zero_exit_message() {
    let err = OutcomeError::NonZeroExit;
    assert_eq!(err.to_string(), "exit status must be Exited(0) for Clean lanes");
}

#[test]
fn outcome_error_empty_findings_message() {
    let err = OutcomeError::EmptyFindings;
    assert_eq!(err.to_string(), "lane outcome Findings payload must contain at least one finding");
}
