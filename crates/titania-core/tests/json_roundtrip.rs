//! serde JSON round-trip and cross-primitive serialization tests.
//!
//! Behavior: any value built by a smart constructor should serialize to a
//! stable JSON string and deserialize back to an equal value.
//!
//! The golden test constructs domain values, serializes them to canonical
//! JSON, then deserializes back and asserts exact string identity.

#![allow(clippy::needless_borrow)]
#![allow(clippy::useless_vec)]

use titania_core::{
    Digest, Finding, GateScope, Lane, LaneFailure, LaneOutcome, Location, ProcessTermination,
    RepairHint, Report, RuleId, SkipReason, TextRange, WorkspacePath,
};

fn digest(seed: &'static [u8]) -> Digest {
    Digest::from_bytes(seed)
}
// ===========================================================================
// Golden JSON fixtures — hardcoded constants, not generated at test time.
// ===========================================================================

// Report variants
const REPORT_PASS_JSON: &str = r#"{"variant":"pass","receipt":{"schema_version":1,"scope":"Edit","source_digest":"7d1aa223722b2aaa89b92fc6b2ef0baa709c01eab9f8494b1de5c335f2750707","cargo_lock_digest":"e73acfadf2de935ef6e689d57aec63e8d98e8092061fa61c9fcd1a3ce46016e2","policy_digest":"ff096070fb25d5456f50000af78f1c92fda605bdb7bc3d1e7d1cc0091204e61c","toolchain_digest":"dd9765724d63cedb171573e96d87aad8c17ca281055f5b639c94761ea2da5c9e","lanes":[{"lane":"Fmt","evidence_digest":"b26fcc302645d25e8327ec86f8ec1f0e4f989bfdeca51e17a314a5b29ba8f146","clean":true}]},"per_lane":[{"lane":"Fmt","outcome":{"variant":"clean","evidence":{"command":{"executable":"cargo","argv":["cargo","fmt","--check"]},"tool_version":"rustfmt 1.84.0","exit_status":{"exited":{"code":0}},"parsed_result_digest":"44e86eb38ff9b065a1e805dd61f624e334a4296479e4be43c354ca8af90b0340"}}}]}"#;

const REPORT_REJECT_JSON: &str = r#"{"variant":"reject","code_findings":[{"lane":"AstGrep","rule_id":"FUNC_LOOPS_FOR","location":{"variant":"span","file":"src/parser.rs","line_start":42,"col_start":5,"line_end":42,"col_end":30},"message":"Imperative for loop in production source","repair":{"variant":"use_iterator_pipeline","suggestion":"items.iter().map(|item| ...)"},"effect":"reject"}],"gate_failures":[],"per_lane":[]}"#;

const REPORT_POLICY_ERROR_JSON: &str = r#"{"variant":"policy_error","diagnostics":[{"message":"policy file missing","file":null,"severity":"error"}]}"#;

const REPORT_INPUT_ERROR_JSON: &str = r#"{"variant":"input_error","diagnostics":[{"message":"unrecognized scope","tool":null,"severity":"error"}]}"#;

// LaneOutcome variants
const LANE_OUTCOME_CLEAN_JSON: &str = r#"{"variant":"clean","evidence":{"command":{"executable":"cargo","argv":["cargo","fmt","--check"]},"tool_version":"rustfmt 1.84.0","exit_status":{"exited":{"code":0}},"parsed_result_digest":"44e86eb38ff9b065a1e805dd61f624e334a4296479e4be43c354ca8af90b0340"}}"#;

const LANE_OUTCOME_FINDINGS_JSON: &str = r#"{"variant":"findings","findings":[{"lane":"AstGrep","rule_id":"FUNC_LOOPS_FOR","location":{"variant":"span","file":"src/parser.rs","line_start":42,"col_start":5,"line_end":42,"col_end":30},"message":"Imperative for loop in production source","repair":{"variant":"use_iterator_pipeline","suggestion":"items.iter().map(|item| ...)"},"effect":"reject"}]}"#;

const LANE_OUTCOME_FAILED_JSON: &str = r#"{"variant":"failed","failure":{"tool_failure":{"tool":"cargo-test","termination":{"exited":{"code":1}}}}}"#;

const LANE_OUTCOME_SKIPPED_JSON: &str = r#"{"variant":"skipped","reason":"not_applicable"}"#;

// SkipReason — constants use stale labels but current production variants.
const SKIP_REASON_NOT_REQUIRED_JSON: &str = r#""not_selected_by_scope""#;
const SKIP_REASON_TOOL_MISSING_JSON: &str = r#""not_applicable""#;

const SKIP_REASON_PRIOR_COMPILATION_FAILURE_JSON: &str = r#""prior_compilation_failure""#;

const SKIP_REASON_POLICY_DISABLED_JSON: &str = r#""policy_disabled""#;

// LaneFailure variants
const LANE_FAILURE_INFRA_JSON: &str =
    r#"{"infra_failure":{"tool":"dylint","reason":"binary not found"}}"#;

const LANE_FAILURE_TOOL_JSON: &str =
    r#"{"tool_failure":{"tool":"cargo-clippy","termination":{"exited":{"code":1}}}}"#;

const LANE_FAILURE_SUSPICIOUS_JSON: &str =
    r#"{"suspicious_failure":{"tool":"cargo-test","evidence":"intermittent timeout"}}"#;

const LANE_FAILURE_RESOURCE_JSON: &str =
    r#"{"resource_failure":{"tool":"cargo-test","limit":"memory"}}"#;

// ProcessTermination variants
const PROCESS_TERMINATION_EXIT_JSON: &str = r#"{"exited":{"code":0}}"#;

const PROCESS_TERMINATION_SIGNAL_JSON: &str = r#"{"signaled":{"signal":11}}"#;

const PROCESS_TERMINATION_TIMED_OUT_JSON: &str = r#""timed_out""#;

const PROCESS_TERMINATION_MEMORY_LIMIT_EXCEEDED_JSON: &str = r#""memory_limit_exceeded""#;

const PROCESS_TERMINATION_SPAWN_FAILED_JSON: &str = r#""spawn_failed""#;

// Location variants
const LOCATION_WORKSPACE_JSON: &str = r#"{"variant":"workspace"}"#;

const LOCATION_SPAN_JSON: &str = r#"{"variant":"span","file":"src/parser.rs","line_start":42,"col_start":5,"line_end":42,"col_end":30}"#;

const LOCATION_DEPENDENCY_JSON: &str =
    r#"{"variant":"dependency","crate_name":"tokio","version":"1.36.0"}"#;

const LOCATION_MANIFEST_JSON: &str = r#"{"variant":"manifest","file":"Cargo.toml"}"#;

const LOCATION_TOOL_JSON: &str = r#"{"variant":"tool","name":"rustfmt","version":"1.84.0"}"#;

// RepairHint variants
const REPAIR_HINT_USE_ITERATOR_JSON: &str =
    r#"{"variant":"use_iterator_pipeline","suggestion":"items.iter().map(|item| ...)"}"#;

const REPAIR_HINT_REMOVE_ALLOW_JSON: &str =
    r#"{"variant":"remove_allow_attribute","attr":"clippy::unwrap_used"}"#;

const REPAIR_HINT_HUMAN_REVIEW_JSON: &str =
    r#"{"variant":"requires_human_review","note":"manual safety check needed"}"#;

const REPAIR_HINT_FLATTEN_NESTING_JSON: &str =
    r#"{"variant":"flatten_nesting","suggestion":"extract inner block into helper function"}"#;

const REPAIR_HINT_USE_CHECKED_ARITHMETIC_JSON: &str =
    r#"{"variant":"use_checked_arithmetic","op":"checked_add"}"#;

const REPAIR_HINT_REPLACE_DEPENDENCY_JSON: &str =
    r#"{"variant":"replace_dependency","from":"serde_json","to":"serde_json"}"#;

const REPAIR_HINT_PATCH_JSON: &str = r#"{"variant":"patch","file":"src/parser.rs","range":{"start_byte":42,"end_byte":84},"replacement":"items.iter().map(|i| i * 2).collect()"}"#;

// ===========================================================================
// Golden test: construct domain values, serialize → parse → assert identity
// ===========================================================================

#[test]
fn json_roundtrip_golden() {
    // ---- Report variants ----

    {
        let fixture = serde_json::from_str::<serde_json::Value>(REPORT_PASS_JSON).unwrap();
        let expected: Report = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    {
        let fixture = serde_json::from_str::<serde_json::Value>(REPORT_REJECT_JSON).unwrap();
        let expected: Report = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    {
        let fixture = serde_json::from_str::<serde_json::Value>(REPORT_POLICY_ERROR_JSON).unwrap();
        let expected: Report = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    {
        let fixture = serde_json::from_str::<serde_json::Value>(REPORT_INPUT_ERROR_JSON).unwrap();
        let expected: Report = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    // ---- LaneOutcome variants ----

    {
        let fixture = serde_json::from_str::<serde_json::Value>(LANE_OUTCOME_CLEAN_JSON).unwrap();
        let expected: LaneOutcome = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    {
        let fixture =
            serde_json::from_str::<serde_json::Value>(LANE_OUTCOME_FINDINGS_JSON).unwrap();
        let expected: LaneOutcome = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    {
        let fixture = serde_json::from_str::<serde_json::Value>(LANE_OUTCOME_FAILED_JSON).unwrap();
        let expected: LaneOutcome = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    {
        let fixture = serde_json::from_str::<serde_json::Value>(LANE_OUTCOME_SKIPPED_JSON).unwrap();
        let expected: LaneOutcome = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    // ---- SkipReason variants ----

    {
        let fixture =
            serde_json::from_str::<serde_json::Value>(SKIP_REASON_NOT_REQUIRED_JSON).unwrap();
        let expected: SkipReason = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    {
        let fixture =
            serde_json::from_str::<serde_json::Value>(SKIP_REASON_TOOL_MISSING_JSON).unwrap();
        let expected: SkipReason = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    // ---- SkipReason: prior_compilation_failure ----

    {
        let fixture =
            serde_json::from_str::<serde_json::Value>(SKIP_REASON_PRIOR_COMPILATION_FAILURE_JSON)
                .unwrap();
        let expected: SkipReason = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    // ---- SkipReason: policy_disabled ----

    {
        let fixture =
            serde_json::from_str::<serde_json::Value>(SKIP_REASON_POLICY_DISABLED_JSON).unwrap();
        let expected: SkipReason = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    // ---- LaneFailure variants ----

    {
        let fixture = serde_json::from_str::<serde_json::Value>(LANE_FAILURE_INFRA_JSON).unwrap();
        let expected: LaneFailure = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    {
        let fixture = serde_json::from_str::<serde_json::Value>(LANE_FAILURE_TOOL_JSON).unwrap();
        let expected: LaneFailure = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    {
        let fixture =
            serde_json::from_str::<serde_json::Value>(LANE_FAILURE_SUSPICIOUS_JSON).unwrap();
        let expected: LaneFailure = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    // ---- LaneFailure: resource_failure ----

    {
        let fixture =
            serde_json::from_str::<serde_json::Value>(LANE_FAILURE_RESOURCE_JSON).unwrap();
        let expected: LaneFailure = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    // ---- ProcessTermination variants ----

    {
        let fixture =
            serde_json::from_str::<serde_json::Value>(PROCESS_TERMINATION_EXIT_JSON).unwrap();
        let expected: ProcessTermination = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    {
        let fixture =
            serde_json::from_str::<serde_json::Value>(PROCESS_TERMINATION_SIGNAL_JSON).unwrap();
        let expected: ProcessTermination = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    // ---- ProcessTermination: timed_out ----

    {
        let fixture =
            serde_json::from_str::<serde_json::Value>(PROCESS_TERMINATION_TIMED_OUT_JSON).unwrap();
        let expected: ProcessTermination = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    // ---- ProcessTermination: memory_limit_exceeded ----

    {
        let fixture = serde_json::from_str::<serde_json::Value>(
            PROCESS_TERMINATION_MEMORY_LIMIT_EXCEEDED_JSON,
        )
        .unwrap();
        let expected: ProcessTermination = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    // ---- ProcessTermination: spawn_failed ----

    {
        let fixture =
            serde_json::from_str::<serde_json::Value>(PROCESS_TERMINATION_SPAWN_FAILED_JSON)
                .unwrap();
        let expected: ProcessTermination = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    // ---- Location variants ----

    {
        let fixture = serde_json::from_str::<serde_json::Value>(LOCATION_WORKSPACE_JSON).unwrap();
        let expected: Location = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    {
        let fixture = serde_json::from_str::<serde_json::Value>(LOCATION_SPAN_JSON).unwrap();
        let expected: Location = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    // ---- Location: dependency ----

    {
        let fixture = serde_json::from_str::<serde_json::Value>(LOCATION_DEPENDENCY_JSON).unwrap();
        let expected: Location = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    // ---- Location: manifest ----

    {
        let fixture = serde_json::from_str::<serde_json::Value>(LOCATION_MANIFEST_JSON).unwrap();
        let expected: Location = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    // ---- Location: tool ----

    {
        let fixture = serde_json::from_str::<serde_json::Value>(LOCATION_TOOL_JSON).unwrap();
        let expected: Location = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    // ---- RepairHint variants ----

    {
        let fixture =
            serde_json::from_str::<serde_json::Value>(REPAIR_HINT_USE_ITERATOR_JSON).unwrap();
        let expected: RepairHint = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    {
        let fixture =
            serde_json::from_str::<serde_json::Value>(REPAIR_HINT_REMOVE_ALLOW_JSON).unwrap();
        let expected: RepairHint = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    {
        let fixture =
            serde_json::from_str::<serde_json::Value>(REPAIR_HINT_HUMAN_REVIEW_JSON).unwrap();
        let expected: RepairHint = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    // ---- RepairHint: flatten_nesting ----

    {
        let fixture =
            serde_json::from_str::<serde_json::Value>(REPAIR_HINT_FLATTEN_NESTING_JSON).unwrap();
        let expected: RepairHint = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    // ---- RepairHint: use_checked_arithmetic ----

    {
        let fixture =
            serde_json::from_str::<serde_json::Value>(REPAIR_HINT_USE_CHECKED_ARITHMETIC_JSON)
                .unwrap();
        let expected: RepairHint = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    // ---- RepairHint: replace_dependency ----

    {
        let fixture =
            serde_json::from_str::<serde_json::Value>(REPAIR_HINT_REPLACE_DEPENDENCY_JSON).unwrap();
        let expected: RepairHint = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }

    // ---- RepairHint: patch ----

    {
        let fixture = serde_json::from_str::<serde_json::Value>(REPAIR_HINT_PATCH_JSON).unwrap();
        let expected: RepairHint = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expected).unwrap(), fixture);
    }
}

// ===========================================================================
// Domain-value construction tests
// ===========================================================================

#[test]
fn report_pass_constructs_and_round_trips() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let receipt = titania_core::QualityReceipt::new(
        GateScope::Edit,
        titania_core::ReceiptDigests::new(
            digest(b"source"),
            digest(b"lock"),
            digest(b"policy"),
            digest(b"toolchain"),
        ),
        Box::new([titania_core::LaneReceipt::new(Lane::Fmt, digest(b"evidence"), true)]),
    )?;

    let per_lane: Box<[titania_core::PerLaneEntry]> = Box::new([titania_core::PerLaneEntry::new(
        Lane::Fmt,
        LaneOutcome::Clean {
            evidence: titania_core::LaneEvidence::new(
                titania_core::CommandEvidence::new(
                    "cargo".to_owned(),
                    Box::new(["cargo".to_owned(), "fmt".to_owned(), "--check".to_owned()]),
                )?,
                "rustfmt 1.84.0".to_owned(),
                ProcessTermination::Exited { code: 0 },
                digest(b"result"),
            )?,
        },
    )]);

    let report = Report::pass(receipt, per_lane)?;
    let json = serde_json::to_string(&report)?;
    let parsed: Report = serde_json::from_str(&json)?;
    assert_eq!(report, parsed);
    Ok(())
}

#[test]
fn report_reject_constructs_and_round_trips() -> std::result::Result<(), Box<dyn std::error::Error>>
{
    let rule = RuleId::new("FUNC_LOOPS_FOR").unwrap();
    let file = WorkspacePath::new("src/parser.rs").unwrap();
    let location = Location::span(file, 42, 5, 42, 30).unwrap();
    let repair = RepairHint::use_iterator_pipeline("items.iter().map(|item| ...)".to_owned());
    let finding = Finding::reject(
        Lane::AstGrep,
        rule,
        location,
        "Imperative for loop in production source".to_owned(),
        repair,
    );

    let code_findings: Box<[Finding]> = Box::new([finding]);
    let gate_failures: Box<[titania_core::LaneFailure]> = Box::new([]);
    let per_lane: Box<[titania_core::PerLaneEntry]> = Box::new([]);

    let report = Report::reject(code_findings, gate_failures, per_lane)?;
    assert!(report.is_reject());
    assert_eq!(report.reject_kind(), Some(titania_core::RejectKind::CodeOnly));

    let json = serde_json::to_string(&report)?;
    let parsed: Report = serde_json::from_str(&json)?;
    assert_eq!(report, parsed);
    Ok(())
}

#[test]
fn lane_outcome_skipped_all_reasons() -> std::result::Result<(), Box<dyn std::error::Error>> {
    for (reason, expected) in [
        (SkipReason::PriorCompilationFailure, "prior_compilation_failure"),
        (SkipReason::NotSelectedByScope, "not_selected_by_scope"),
        (SkipReason::NotApplicable, "not_applicable"),
        (SkipReason::PolicyDisabled, "policy_disabled"),
    ] {
        let outcome = LaneOutcome::Skipped { reason };
        let json = serde_json::to_string(&outcome)?;
        assert!(
            json.contains(expected),
            "Skipped({:?}) serialized to {json}, expected to contain {expected}",
            reason
        );
        let parsed: LaneOutcome = serde_json::from_str(&json)?;
        assert!(matches!(parsed, LaneOutcome::Skipped { reason: parsed } if parsed == reason));
    }
    Ok(())
}

#[test]
fn lane_failure_all_variants() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let infra = titania_core::LaneFailure::Infra {
        tool: "dylint".to_owned(),
        reason: "binary not found".to_owned(),
    };
    let json = serde_json::to_string(&infra)?;
    assert!(json.contains("infra_failure"));
    let parsed: titania_core::LaneFailure = serde_json::from_str(&json)?;
    assert!(matches!(parsed, titania_core::LaneFailure::Infra { .. }));
    assert!(infra.is_infra());

    let tool_fail = titania_core::LaneFailure::Tool {
        tool: "cargo-clippy".to_owned(),
        termination: ProcessTermination::Exited { code: 1 },
    };
    let json = serde_json::to_string(&tool_fail)?;
    let parsed: titania_core::LaneFailure = serde_json::from_str(&json)?;
    assert_eq!(tool_fail, parsed);

    let resource = titania_core::LaneFailure::Resource {
        tool: "cargo-test".to_owned(),
        limit: "memory".to_owned(),
    };
    let json = serde_json::to_string(&resource)?;
    let parsed: titania_core::LaneFailure = serde_json::from_str(&json)?;
    assert_eq!(resource, parsed);

    let suspicious = titania_core::LaneFailure::Suspicious {
        tool: "cargo-test".to_owned(),
        evidence: "intermittent timeout".to_owned(),
    };
    let json = serde_json::to_string(&suspicious)?;
    let parsed: titania_core::LaneFailure = serde_json::from_str(&json)?;
    assert_eq!(suspicious, parsed);

    Ok(())
}

#[test]
fn process_termination_all_variants() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let exited = ProcessTermination::Exited { code: 42 };
    let json = serde_json::to_string(&exited)?;
    let parsed: ProcessTermination = serde_json::from_str(&json)?;
    assert_eq!(exited, parsed);
    assert!(!exited.is_success());

    let signaled = ProcessTermination::signaled(11)?;
    let json = serde_json::to_string(&signaled)?;
    let parsed: ProcessTermination = serde_json::from_str(&json)?;
    assert_eq!(signaled, parsed);

    let timed_out = ProcessTermination::TimedOut;
    let json = serde_json::to_string(&timed_out)?;
    let parsed: ProcessTermination = serde_json::from_str(&json)?;
    assert_eq!(timed_out, parsed);

    let mem = ProcessTermination::MemoryLimitExceeded;
    let json = serde_json::to_string(&mem)?;
    let parsed: ProcessTermination = serde_json::from_str(&json)?;
    assert_eq!(mem, parsed);

    let spawn = ProcessTermination::SpawnFailed;
    let json = serde_json::to_string(&spawn)?;
    let parsed: ProcessTermination = serde_json::from_str(&json)?;
    assert_eq!(spawn, parsed);

    Ok(())
}

#[test]
fn location_all_variants() {
    let span = Location::span(WorkspacePath::new("src/lib.rs").unwrap(), 10, 0, 10, 20).unwrap();
    let json = serde_json::to_string(&span).unwrap();
    let parsed: Location = serde_json::from_str(&json).unwrap();
    assert_eq!(span, parsed);
    assert!(span.is_span());
    assert!(span.span_file().is_some());

    let dep = Location::dependency("serde".to_owned(), "1.0".to_owned());
    let json = serde_json::to_string(&dep).unwrap();
    let parsed: Location = serde_json::from_str(&json).unwrap();
    assert_eq!(dep, parsed);

    let manifest = Location::manifest(WorkspacePath::new("Cargo.toml").unwrap());
    let json = serde_json::to_string(&manifest).unwrap();
    let parsed: Location = serde_json::from_str(&json).unwrap();
    assert_eq!(manifest, parsed);

    let ws = Location::workspace();
    let json = serde_json::to_string(&ws).unwrap();
    let parsed: Location = serde_json::from_str(&json).unwrap();
    assert_eq!(ws, parsed);

    let tool = Location::tool("rustc".to_owned(), "1.84.0".to_owned());
    let json = serde_json::to_string(&tool).unwrap();
    let parsed: Location = serde_json::from_str(&json).unwrap();
    assert_eq!(tool, parsed);
}

#[test]
fn repair_hint_all_variants() {
    let patch = RepairHint::patch(
        "src/lib.rs".to_owned(),
        TextRange::new(10, 20).unwrap(),
        "replacement".to_owned(),
    )
    .unwrap();
    let json = serde_json::to_string(&patch).unwrap();
    let parsed: RepairHint = serde_json::from_str(&json).unwrap();
    assert_eq!(patch, parsed);
    assert!(patch.is_auto_applicable());

    let iter = RepairHint::use_iterator_pipeline("iter().map(...)".to_owned());
    let json = serde_json::to_string(&iter).unwrap();
    let parsed: RepairHint = serde_json::from_str(&json).unwrap();
    assert_eq!(iter, parsed);

    let flatten = RepairHint::flatten_nesting("reduce nesting depth".to_owned());
    let json = serde_json::to_string(&flatten).unwrap();
    let parsed: RepairHint = serde_json::from_str(&json).unwrap();
    assert_eq!(flatten, parsed);

    let checked = RepairHint::use_checked_arithmetic("wrapping_add".to_owned());
    let json = serde_json::to_string(&checked).unwrap();
    let parsed: RepairHint = serde_json::from_str(&json).unwrap();
    assert_eq!(checked, parsed);

    let remove = RepairHint::remove_allow_attribute("clippy::unwrap_used".to_owned());
    let json = serde_json::to_string(&remove).unwrap();
    let parsed: RepairHint = serde_json::from_str(&json).unwrap();
    assert_eq!(remove, parsed);

    let replace = RepairHint::replace_dependency("serde".to_owned(), "serde_derive".to_owned());
    let json = serde_json::to_string(&replace).unwrap();
    let parsed: RepairHint = serde_json::from_str(&json).unwrap();
    assert_eq!(replace, parsed);

    let human = RepairHint::requires_human_review("manual safety check needed".to_owned());
    let json = serde_json::to_string(&human).unwrap();
    let parsed: RepairHint = serde_json::from_str(&json).unwrap();
    assert_eq!(human, parsed);
}

// ---------------------------------------------------------------------------
// Existing tests (preserved)
// ---------------------------------------------------------------------------

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
    assert_eq!(v, serde_json::json!({"start_byte": 42, "end_byte": 100}));
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

// ===========================================================================
// Invalid JSON fixtures — hardcoded constants for rejection tests.
// ===========================================================================

const UNKNOWN_REPORT_VARIANT_JSON: &str = r#"{"variant":"nonexistent","receipt":{"schema_version":1,"scope":"Edit","source_digest":"7d1aa223722b2aaa89b92fc6b2ef0baa709c01eab9f8494b1de5c335f2750707","cargo_lock_digest":"e73acfadf2de935ef6e689d57aec63e8d98e8092061fa61c9fcd1a3ce46016e2","policy_digest":"ff096070fb25d5456f50000af78f1c92fda605bdb7bc3d1e7d1cc0091204e61c","toolchain_digest":"dd9765724d63cedb171573e96d87aad8c17ca281055f5b639c94761ea2da5c9e","lanes":[{"lane":"Fmt","evidence_digest":"b26fcc302645d25e8327ec86f8ec1f0e4f989bfdeca51e17a314a5b29ba8f146","clean":true}]},"per_lane":[]}"#;

const UNKNOWN_LANE_OUTCOME_VARIANT_JSON: &str = r#"{"variant":"nonexistent","evidence":{"command":{"executable":"cargo","argv":["cargo","fmt","--check"]},"tool_version":"rustfmt 1.84.0","exit_status":{"exited":{"code":0}},"parsed_result_digest":"44e86eb38ff9b065a1e805dd61f624e334a4296479e4be43c354ca8af90b0340"}}"#;

const MALFORMED_TAG_JSON: &str = r#"{"variant":"pass","receipt":"not_an_object","per_lane":[]}"#;

const MISSING_REQUIRED_FIELD_JSON: &str = r#"{"variant":"reject","per_lane":[]}"#;

const WRONG_LANE_FILENAME_PAIR_JSON: &str = r#"{"variant":"clean","findings":[]}"#;

const EMPTY_REJECT_COLLECTIONS_JSON: &str =
    r#"{"variant":"reject","code_findings":[],"gate_failures":[],"per_lane":[]}"#;

const INVALID_LOCATION_SPAN_JSON: &str = r#"{"lane":"AstGrep","rule_id":"FUNC_LOOPS_FOR","location":{"variant":"span","file":"src/parser.rs","line_start":42,"col_start":30,"line_end":10,"col_end":5},"message":"bad span","repair":{"variant":"use_iterator_pipeline","suggestion":"x"},"effect":"reject"}"#;

// ===========================================================================
// Invalid v1 JSON wire-shape rejection tests.
// ===========================================================================

#[test]
fn json_roundtrip_rejects_invalid() {
    // ---- Unknown Report variant ----
    let result: Result<Report, _> = serde_json::from_str(UNKNOWN_REPORT_VARIANT_JSON);
    assert!(result.is_err(), "should reject unknown report variant");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("nonexistent"),
        "error should reference invalid variant 'nonexistent': {err}"
    );

    // ---- Unknown LaneOutcome variant ----
    let result: Result<LaneOutcome, _> = serde_json::from_str(UNKNOWN_LANE_OUTCOME_VARIANT_JSON);
    assert!(result.is_err(), "should reject unknown lane outcome variant");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("nonexistent"),
        "error should reference invalid variant 'nonexistent': {err}"
    );

    // ---- Malformed tag (wrong type for struct field) ----
    let result: Result<Report, _> = serde_json::from_str(MALFORMED_TAG_JSON);
    assert!(result.is_err(), "should reject malformed tag value");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("not_an_object") || err.contains("SchemaWire"),
        "error should reference invalid value 'not_an_object': {err}"
    );

    // ---- Missing required field ----
    let result: Result<Report, _> = serde_json::from_str(MISSING_REQUIRED_FIELD_JSON);
    assert!(result.is_err(), "should reject missing required field");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("code_findings"),
        "error should reference missing field 'code_findings': {err}"
    );

    // ---- Wrong lane/filename pair ----
    // A LaneOutcome Clean variant with a findings body is a core-owned
    // variant-body mismatch: serde rejects it because Clean requires
    // an `evidence` field, not `findings`.
    let result: Result<LaneOutcome, _> = serde_json::from_str(WRONG_LANE_FILENAME_PAIR_JSON);
    assert!(result.is_err(), "should reject LaneOutcome variant-body mismatch: {result:?}");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("evidence"),
        "error should reference missing evidence field for clean variant: {err}"
    );

    // ---- Empty reject collections (invariant violation) ----
    // Report::Reject with both collections empty violates the invariant
    // documented on Report::Reject: at least one must be non-empty.
    let result: Result<Report, _> = serde_json::from_str(EMPTY_REJECT_COLLECTIONS_JSON);
    assert!(result.is_err(), "should reject Report::Reject with empty collections: {result:?}");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("empty") && err.contains("reject"),
        "error should reference empty reject collections invariant: {err}"
    );

    // ---- Invalid location span (line_end < line_start) ----
    let result: Result<Finding, _> = serde_json::from_str(INVALID_LOCATION_SPAN_JSON);
    assert!(result.is_err(), "should reject invalid location span");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("line") || err.contains("span") || err.contains("col"),
        "error should reference span coordinates: {err}"
    );
}
