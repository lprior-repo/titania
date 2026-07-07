//! Round-trip tests for the canonical on-disk lane artifact format.
//!
//! These prove every [`LaneOutcome`] variant serializes through the shared
//! [`LaneArtifact`] / [`ArtifactOutcome`] projection and parses back into an
//! equal [`LaneOutcome`]. The `dylint_infra_failure_*` cases are the direct
//! regression for the aggregator parse bug: a lane that fails because its tool
//! binary is missing must remain aggregator-parseable end to end.
//!
//! Beads: tn-6qv

use serde_json::Value;
use titania_core::{
    ArtifactOutcome, CommandEvidence, Digest, Finding, Lane, LaneArtifact, LaneEvidence,
    LaneFailure, LaneOutcome, Location, ProcessTermination, RepairHint, RuleId, SkipReason,
    WorkspacePath,
};

fn stub_evidence() -> LaneEvidence {
    let command = CommandEvidence::new(
        "cargo".into(),
        vec!["cargo".into(), "fmt".into(), "--check".into()].into_boxed_slice(),
    )
    .expect("valid argv");
    LaneEvidence::new(
        command,
        "rustfmt 1.84.0".into(),
        ProcessTermination::Exited { code: 0 },
        Digest::from_bytes(b"stub-evidence-digest"),
    )
    .expect("clean exit")
}

fn stub_finding() -> Finding {
    Finding::reject(
        Lane::Fmt,
        RuleId::new("FUNC_PRINT_STDOUT").expect("valid rule id"),
        Location::span(WorkspacePath::new("src/main.rs").expect("valid path"), 42, 5, 42, 30)
            .expect("valid span"),
        String::from("Found `println!` in production source"),
        RepairHint::requires_human_review(String::from("Replace with tracing")),
    )
}

fn round_trip(outcome: &LaneOutcome, lane: Lane) -> LaneOutcome {
    let artifact = LaneArtifact::new(lane, ArtifactOutcome::from(outcome));
    let json = serde_json::to_string(&artifact).expect("serialize artifact");
    let parsed: LaneArtifact = serde_json::from_str(&json).expect("deserialize artifact");
    parsed.into_outcome().into_lane_outcome().expect("reconstruct outcome")
}

#[test]
fn clean_outcome_round_trips() {
    let original = LaneOutcome::Clean { evidence: stub_evidence() };
    assert_eq!(round_trip(&original, Lane::Fmt), original);
}

#[test]
fn findings_outcome_round_trips() {
    let original = LaneOutcome::Findings { findings: Box::new([stub_finding()]) };
    assert_eq!(round_trip(&original, Lane::Fmt), original);
}

#[test]
fn skipped_outcome_round_trips() {
    let original = LaneOutcome::Skipped { reason: SkipReason::NotSelectedByScope };
    assert_eq!(round_trip(&original, Lane::PolicyScan), original);
}

#[test]
fn dylint_infra_failure_outcome_round_trips() {
    // Regression for the aggregator parse error: a dylint lane that fails
    // because cargo-dylint is missing must serialize to the canonical v1
    // artifact shape and parse back into LaneOutcome::Failed { failure: ... }.
    let original = LaneOutcome::Failed {
        failure: LaneFailure::Infra {
            tool: String::from("cargo-dylint"),
            reason: String::from("subcommand unavailable"),
        },
    };
    assert_eq!(round_trip(&original, Lane::Dylint), original);
}

#[test]
fn dylint_infra_failure_on_disk_shape_is_aggregator_parseable() {
    // The on-disk shape must use the v1 external lane tag and the inner
    // LaneFailure tag. This is the exact shape `.titania/out/edit/dylint.json`
    // must hold.
    let outcome = LaneOutcome::Failed {
        failure: LaneFailure::Infra {
            tool: String::from("cargo-dylint"),
            reason: String::from("subcommand unavailable"),
        },
    };
    let artifact = LaneArtifact::new(Lane::Dylint, ArtifactOutcome::from(&outcome));
    let json: Value = serde_json::to_value(&artifact).expect("serialize to value");

    assert_eq!(json["lane"], "Dylint");
    assert_eq!(json["outcome"]["Failed"]["InfraFailure"]["tool"], "cargo-dylint");
    assert_eq!(json["outcome"]["Failed"]["InfraFailure"]["reason"], "subcommand unavailable");

    assert_ne!(json["outcome"]["Failed"]["InfraFailure"], Value::Null);
}
