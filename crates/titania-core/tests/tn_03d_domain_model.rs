//! Comprehensive tests for the v1 domain model (19 types).
//!
//! Covers: serde round-trips, constructor validation, invariants,
//! Acceptance criteria from bead tn-03d §4.

#![allow(clippy::as_conversions)]
#![allow(clippy::useless_vec)]
#![allow(clippy::arithmetic_side_effects)]

use titania_core::{
    CommandEvidence, DiagnosticSeverity, Digest, Finding, FindingEffect, GateScope,
    InputDiagnostic, Lane, LaneEvidence, LaneFailure, LaneOutcome, LaneReceipt, Location,
    PolicyDiagnostic, ProcessTermination, QualityReceipt, ReceiptDigests, ReceiptError, RejectKind,
    RepairHint, Report, RuleId, SkipReason, TextRange, WorkspacePath,
};

// ===========================================================================
// Lane — 10 variants
// ===========================================================================

#[test]
fn lane_all_10_variants_constructible() {
    let names = [
        "Fmt",
        "Compile",
        "Clippy",
        "AstGrep",
        "Dylint",
        "PanicScan",
        "PolicyScan",
        "Test",
        "Deny",
        "Build",
    ];
    for name in names {
        let lane = name.parse::<Lane>();
        assert!(lane.is_ok(), "Lane::{} should parse", name);
    }
}

#[test]
fn lane_from_str_exact_pascal_case() {
    assert_eq!("Fmt".parse::<Lane>(), Ok(Lane::Fmt));
    assert_eq!("Compile".parse::<Lane>(), Ok(Lane::Compile));
    assert_eq!("Clippy".parse::<Lane>(), Ok(Lane::Clippy));
    assert_eq!("AstGrep".parse::<Lane>(), Ok(Lane::AstGrep));
    assert_eq!("Dylint".parse::<Lane>(), Ok(Lane::Dylint));
    assert_eq!("PanicScan".parse::<Lane>(), Ok(Lane::PanicScan));
    assert_eq!("PolicyScan".parse::<Lane>(), Ok(Lane::PolicyScan));
    assert_eq!("Test".parse::<Lane>(), Ok(Lane::Test));
    assert_eq!("Deny".parse::<Lane>(), Ok(Lane::Deny));
    assert_eq!("Build".parse::<Lane>(), Ok(Lane::Build));
    // Case-sensitive: lowercase should fail
    assert!("compile".parse::<Lane>().is_err());
    assert!("fmt".parse::<Lane>().is_err());
    // Mixed case should fail
    assert!("FMT".parse::<Lane>().is_err());
}

#[test]
fn lane_from_str_unknown_rejected() {
    assert!("UNKNOWN".parse::<Lane>().is_err());
    assert!("".parse::<Lane>().is_err());
}

#[test]
fn lane_serde_round_trip() {
    let lanes = [
        Lane::Fmt,
        Lane::Compile,
        Lane::Clippy,
        Lane::AstGrep,
        Lane::Dylint,
        Lane::PanicScan,
        Lane::PolicyScan,
        Lane::Test,
        Lane::Deny,
        Lane::Build,
    ];
    for lane in &lanes {
        let json = serde_json::to_string(lane).unwrap();
        let back: Lane = serde_json::from_str(&json).unwrap();
        assert_eq!(*lane, back);
    }
}

// ===========================================================================
// GateScope — 3 variants + #[non_exhaustive]
// ===========================================================================

#[test]
fn gate_scope_from_str() {
    assert_eq!("edit".parse::<GateScope>(), Ok(GateScope::Edit));
    assert_eq!("prepush".parse::<GateScope>(), Ok(GateScope::Prepush));
    assert_eq!("release".parse::<GateScope>(), Ok(GateScope::Release));
    assert!("unknown".parse::<GateScope>().is_err());
}

#[test]
fn gate_scope_edit_lanes_ordered() {
    let edit = GateScope::Edit;
    let lanes = edit.lanes();
    assert_eq!(lanes.len(), 7);
    assert_eq!(
        lanes,
        [
            Lane::Fmt,
            Lane::Compile,
            Lane::Clippy,
            Lane::AstGrep,
            Lane::Dylint,
            Lane::PanicScan,
            Lane::PolicyScan,
        ]
    );
}

#[test]
fn gate_scope_prepush_lanes_ordered() {
    let prepush = GateScope::Prepush;
    let lanes = prepush.lanes();
    assert_eq!(lanes.len(), 9);
    assert_eq!(lanes[7], Lane::Test);
    assert_eq!(lanes[8], Lane::Deny);
}

#[test]
fn gate_scope_release_lanes_ordered() {
    let release = GateScope::Release;
    let lanes = release.lanes();
    assert_eq!(lanes.len(), 10);
    assert_eq!(lanes[9], Lane::Build);
}

#[test]
fn gate_scope_serde_round_trip() {
    let scopes = [GateScope::Edit, GateScope::Prepush, GateScope::Release];
    for scope in &scopes {
        let json = serde_json::to_string(scope).unwrap();
        let back: GateScope = serde_json::from_str(&json).unwrap();
        assert_eq!(*scope, back);
    }
}

// ===========================================================================
// Finding, FindingEffect, Location, RepairHint
// ===========================================================================

fn make_valid_finding() -> Finding {
    Finding::reject(
        Lane::Clippy,
        RuleId::new("CLIPPY_UNWRAP_USED").unwrap(),
        Location::span(WorkspacePath::new("src/lib.rs").unwrap(), 10, 5, 10, 20).unwrap(),
        "unwrap() used".to_string(),
        RepairHint::use_iterator_pipeline("use .into_iter()".to_string()),
    )
}

#[test]
fn finding_constructs_valid() {
    let f = make_valid_finding();
    assert_eq!(f.lane(), Lane::Clippy);
    assert_eq!(f.rule_id().as_str(), "CLIPPY_UNWRAP_USED");
    assert_eq!(f.message(), "unwrap() used");
}

#[test]
fn finding_serde_round_trip() {
    let f = make_valid_finding();
    let json = serde_json::to_string(&f).unwrap();
    let back: Finding = serde_json::from_str(&json).unwrap();
    assert_eq!(f, back);
}

#[test]
fn location_span_rejects_line_start_zero() {
    let result = Location::span(WorkspacePath::new("src/lib.rs").unwrap(), 0, 0, 1, 1);
    assert!(result.is_err());
    assert!(matches!(result, Err(titania_core::LocationError::LineStartBeforeOne)));
}

#[test]
fn location_span() {
    let span = Location::span(WorkspacePath::new("src/main.rs").unwrap(), 1, 0, 5, 10).unwrap();
    assert!(span.is_span());
    assert_eq!(span.span_file().map(WorkspacePath::as_str), Some("src/main.rs"));
    let dep = Location::dependency("serde".to_string(), "1.0".to_string());
    assert!(dep.span_file().is_none());
    let ws = Location::workspace();
    assert!(ws.span_file().is_none());
}

#[test]
fn location_serde_round_trip() {
    let locations: [Location; 5] = [
        Location::span(WorkspacePath::new("src/a.rs").unwrap(), 1, 0, 2, 5).unwrap(),
        Location::dependency("tokio".to_string(), "1.0".to_string()),
        Location::manifest(WorkspacePath::new("Cargo.toml").unwrap()),
        Location::workspace(),
        Location::tool("clippy".to_string(), "1.84".to_string()),
    ];
    for loc in &locations {
        let json = serde_json::to_string(loc).unwrap();
        let back: Location = serde_json::from_str(&json).unwrap();
        assert_eq!(*loc, back);
    }
}

#[test]
fn repair_hint_patch() {
    let range = TextRange::new(0, 10).unwrap();
    let patch = RepairHint::patch("file.rs".to_string(), range, "replacement".to_string()).unwrap();
    assert!(patch.is_auto_applicable());
}

#[test]
fn repair_hint_other_variants() {
    let ui = RepairHint::use_iterator_pipeline("use .iter()".to_string());
    assert!(!ui.is_auto_applicable());

    let human = RepairHint::requires_human_review("manual fix".to_string());
    assert!(!human.is_auto_applicable());
}

#[test]
fn repair_hint_patch_rejects_zero_width() {
    let range = TextRange::new(5, 5).unwrap();
    let result = RepairHint::patch("f.rs".to_string(), range, "x".to_string());
    assert!(result.is_err());
}

#[test]
fn repair_hint_serde_round_trip() {
    let range = TextRange::new(0, 10).unwrap();
    let hints = [
        RepairHint::patch("a.rs".to_string(), range, "r".to_string()).unwrap(),
        RepairHint::use_iterator_pipeline("s".to_string()),
        RepairHint::flatten_nesting("s".to_string()),
        RepairHint::use_checked_arithmetic("add".to_string()),
        RepairHint::remove_allow_attribute("allow(unused)".to_string()),
        RepairHint::replace_dependency("a".to_string(), "b".to_string()),
        RepairHint::requires_human_review("n".to_string()),
    ];
    for hint in &hints {
        let json = serde_json::to_string(hint).unwrap();
        let back: RepairHint = serde_json::from_str(&json).unwrap();
        assert_eq!(*hint, back);
    }
}

// ===========================================================================
// Report, RejectKind
// ===========================================================================

fn make_quality_receipt() -> Result<QualityReceipt, ReceiptError> {
    let digest = Digest::from_bytes(b"test");
    QualityReceipt::new(
        GateScope::Edit,
        ReceiptDigests::new(digest.clone(), digest.clone(), digest.clone(), digest.clone()),
        Box::new([LaneReceipt::new(Lane::Fmt, digest, true)]),
    )
}

#[test]
fn report_pass_direct_construction() {
    let receipt = make_quality_receipt().unwrap();
    let per_lane: Box<[titania_core::PerLaneEntry]> = Box::new([titania_core::PerLaneEntry::new(
        Lane::Fmt,
        LaneOutcome::Skipped { reason: titania_core::SkipReason::PriorCompilationFailure },
    )]);
    let report = Report::pass(receipt, per_lane).unwrap();
    assert!(report.is_pass());
    assert!(!report.is_reject());
}
#[test]
fn report_pass_constructor_accepts_lane_outcomes() {
    let receipt = make_quality_receipt().unwrap();
    let per_lane: Box<[titania_core::PerLaneEntry]> = Box::new([titania_core::PerLaneEntry::new(
        Lane::Fmt,
        LaneOutcome::Skipped { reason: SkipReason::NotApplicable },
    )]);
    let report = Report::pass(receipt, per_lane).unwrap();
    assert!(report.is_pass());
}

#[test]
fn report_pass_constructor_rejects_empty_lane_outcomes() {
    let receipt = make_quality_receipt().unwrap();
    let per_lane: Box<[titania_core::PerLaneEntry]> = Box::new([]);
    let result = Report::pass(receipt, per_lane);
    assert!(matches!(result, Err(titania_core::ReportError::EmptyPerLane)));
}

#[test]
fn report_reject_code_only() {
    let finding = make_valid_finding();
    let report = Report::reject(Box::new([finding]), Box::new([]), Box::new([])).unwrap();
    assert_eq!(report.reject_kind(), Some(RejectKind::CodeOnly));
    assert_eq!(report.code_findings().unwrap().len(), 1);
    assert!(report.gate_failures().unwrap().is_empty());
}

#[test]
fn report_reject_gate_only() {
    let failure =
        LaneFailure::Infra { tool: "cargo-fmt".to_string(), reason: "missing".to_string() };
    let report = Report::reject(Box::new([]), Box::new([failure]), Box::new([])).unwrap();
    assert_eq!(report.reject_kind(), Some(RejectKind::GateOnly));
}

#[test]
fn report_reject_mixed() {
    let finding = make_valid_finding();
    let failure =
        LaneFailure::Infra { tool: "cargo-fmt".to_string(), reason: "missing".to_string() };
    let report = Report::reject(Box::new([finding]), Box::new([failure]), Box::new([])).unwrap();
    assert_eq!(report.reject_kind(), Some(RejectKind::Mixed));
}

#[test]
fn report_reject_rejects_empty_collections() {
    let result = Report::reject(Box::new([]), Box::new([]), Box::new([]));
    assert!(matches!(result, Err(titania_core::ReportError::EmptyReject)));
}

#[test]
fn report_reject_kind_none_on_non_reject() {
    let receipt = make_quality_receipt().unwrap();
    let p = Report::pass(
        receipt,
        Box::new([titania_core::PerLaneEntry::new(
            Lane::Fmt,
            LaneOutcome::Skipped { reason: titania_core::SkipReason::PriorCompilationFailure },
        )]),
    )
    .unwrap();
    assert_eq!(p.reject_kind(), None);
    let diag = Box::new([PolicyDiagnostic {
        message: "bad policy".to_string(),
        file: None,
        severity: DiagnosticSeverity::Error,
    }]);
    let e = Report::policy_error(diag);
    assert_eq!(e.reject_kind(), None);
}

#[test]
fn report_serde_round_trip() {
    let finding = make_valid_finding();
    let report = Report::reject(Box::new([finding]), Box::new([]), Box::new([])).unwrap();
    let json = serde_json::to_string(&report).unwrap();
    let back: Report = serde_json::from_str(&json).unwrap();
    assert_eq!(report, back);
}

/// Informational-only findings SHOULD make Report::pass succeed.
/// This guards the pass-shaped findings invariant used by aggregate reports.
#[test]
fn report_pass_accepts_informational_only_findings() {
    let rule_id = RuleId::new("TEST_STYLE_NOTE").unwrap();
    let file = WorkspacePath::new("src/main.rs").unwrap();
    let loc = Location::span(file, 1, 0, 1, 10).unwrap();
    let repair = RepairHint::patch(
        "src/main.rs".to_string(),
        TextRange::new(0, 10).unwrap(),
        "// style fix".to_string(),
    )
    .unwrap();
    let finding = Finding::informational(Lane::Fmt, rule_id, loc, "style note".into(), repair);
    let receipt = make_quality_receipt().unwrap();
    let per_lane: Box<[titania_core::PerLaneEntry]> = Box::new([titania_core::PerLaneEntry::new(
        Lane::Fmt,
        LaneOutcome::Findings { findings: Box::new([finding]) },
    )]);
    let result = Report::pass(receipt, per_lane);
    assert!(
        result.is_ok(),
        "Report::pass with informational-only findings should succeed: {result:?}"
    );
}

#[test]
fn report_pass_rejects_findings_with_reject() {
    let rule_id = RuleId::new("TEST_LINT_VIOLATION").unwrap();
    let file = WorkspacePath::new("src/main.rs").unwrap();
    let loc = Location::span(file, 1, 0, 1, 10).unwrap();
    let repair = RepairHint::patch(
        "src/main.rs".to_string(),
        TextRange::new(0, 10).unwrap(),
        "// fix".to_string(),
    )
    .unwrap();
    let finding = Finding::reject(Lane::Fmt, rule_id, loc, "lint violation".into(), repair);
    let receipt = make_quality_receipt().unwrap();
    let per_lane: Box<[titania_core::PerLaneEntry]> = Box::new([titania_core::PerLaneEntry::new(
        Lane::Fmt,
        LaneOutcome::Findings { findings: Box::new([finding]) },
    )]);
    let result = Report::pass(receipt, per_lane);
    assert!(result.is_err(), "Report::pass must reject Findings with reject findings");
}

#[test]
fn report_pass_rejects_failed_outcome() {
    let failure =
        LaneFailure::Infra { tool: "cargo-test".to_string(), reason: "not found".to_string() };
    let receipt = make_quality_receipt().unwrap();
    let per_lane: Box<[titania_core::PerLaneEntry]> =
        Box::new([titania_core::PerLaneEntry::new(Lane::Fmt, LaneOutcome::Failed { failure })]);
    let result = Report::pass(receipt, per_lane);
    assert!(result.is_err());
}

// ===========================================================================
// LaneOutcome, SkipReason
// ===========================================================================

fn make_valid_evidence() -> LaneEvidence {
    let cmd = CommandEvidence::new(
        "cargo".to_string(),
        Box::new(["cargo".to_string(), "fmt".to_string()]),
    )
    .unwrap();
    LaneEvidence::new(
        cmd,
        "rustfmt 1.84.0".to_string(),
        ProcessTermination::Exited { code: 0 },
        Digest::from_bytes(b"evidence"),
    )
    .unwrap()
}

#[test]
fn lane_outcome_clean() {
    let evidence = make_valid_evidence();
    let outcome = LaneOutcome::Clean { evidence };
    assert!(matches!(outcome, LaneOutcome::Clean { .. }));
}

#[test]
fn lane_outcome_clean_rejects_nonzero_exit() {
    let cmd = CommandEvidence::new("cargo".to_string(), Box::new(["cargo".to_string()])).unwrap();
    let result = LaneEvidence::new(
        cmd,
        "cargo 1.0".to_string(),
        ProcessTermination::Exited { code: 1 },
        Digest::from_bytes(b"x"),
    );
    assert!(result.is_err());
}

#[test]
fn lane_outcome_findings() {
    let findings: Box<[Finding]> = Box::new([]);
    let outcome = LaneOutcome::Findings { findings };
    assert!(matches!(outcome, LaneOutcome::Findings { findings } if findings.is_empty()));
}

#[test]
fn lane_outcome_failed() {
    let failure =
        LaneFailure::Infra { tool: "dylint".to_string(), reason: "not found".to_string() };
    let outcome = LaneOutcome::Failed { failure };
    assert!(matches!(outcome, LaneOutcome::Failed { .. }));
}

#[test]
fn lane_outcome_skipped_all_reasons() {
    let outcome = LaneOutcome::Skipped { reason: SkipReason::PriorCompilationFailure };
    assert!(matches!(
        outcome,
        LaneOutcome::Skipped { reason: SkipReason::PriorCompilationFailure }
    ));

    let outcome = LaneOutcome::Skipped { reason: SkipReason::NotSelectedByScope };
    assert!(matches!(outcome, LaneOutcome::Skipped { reason: SkipReason::NotSelectedByScope }));

    let outcome = LaneOutcome::Skipped { reason: SkipReason::NotApplicable };
    assert!(matches!(outcome, LaneOutcome::Skipped { reason: SkipReason::NotApplicable }));

    let outcome = LaneOutcome::Skipped { reason: SkipReason::PolicyDisabled };
    assert!(matches!(outcome, LaneOutcome::Skipped { reason: SkipReason::PolicyDisabled }));
}

#[test]
fn lane_outcome_is_pass_clean() {
    let evidence = make_valid_evidence();
    let outcome = LaneOutcome::Clean { evidence };
    assert!(outcome.is_pass());
}

#[test]
fn lane_outcome_is_pass_skipped() {
    let outcome = LaneOutcome::Skipped { reason: SkipReason::NotSelectedByScope };
    assert!(outcome.is_pass());
}

#[test]
fn lane_outcome_is_not_pass_findings_with_reject() {
    let rule_id = RuleId::new("TEST_LINT_VIOLATION").unwrap();
    let file = WorkspacePath::new("src/main.rs").unwrap();
    let loc = Location::span(file, 1, 0, 1, 10).unwrap();
    let repair = RepairHint::patch(
        "src/main.rs".to_string(),
        TextRange::new(0, 10).unwrap(),
        "// fix".to_string(),
    )
    .unwrap();
    let finding = Finding::reject(Lane::Fmt, rule_id, loc, "lint violation".into(), repair);
    let outcome = LaneOutcome::Findings { findings: Box::new([finding]) };
    assert!(!outcome.is_pass());
}

#[test]
fn lane_outcome_is_not_pass_findings_mixed() {
    let rule_id_r = RuleId::new("TEST_LINT_VIOLATION").unwrap();
    let rule_id_i = RuleId::new("TEST_STYLE_NOTE").unwrap();
    let file = WorkspacePath::new("src/main.rs").unwrap();
    let loc = Location::span(file, 1, 0, 1, 10).unwrap();
    let repair = RepairHint::patch(
        "src/main.rs".to_string(),
        TextRange::new(0, 10).unwrap(),
        "// fix".to_string(),
    )
    .unwrap();
    let reject = Finding::reject(Lane::Fmt, rule_id_r, loc.clone(), "lint".into(), repair.clone());
    let info = Finding::informational(Lane::Fmt, rule_id_i, loc, "style note".into(), repair);
    let outcome = LaneOutcome::Findings { findings: Box::new([info, reject]) };
    assert!(!outcome.is_pass());
}

#[test]
fn lane_outcome_is_not_pass_failed() {
    let failure =
        LaneFailure::Infra { tool: "cargo-test".to_string(), reason: "not found".to_string() };
    let outcome = LaneOutcome::Failed { failure };
    assert!(!outcome.is_pass());
}

// ===========================================================================
// ProcessTermination, LaneFailure
// ===========================================================================

#[test]
fn process_termination_variants() {
    assert!(
        matches!(ProcessTermination::Exited { code: 0 }, ProcessTermination::Exited { code } if code == 0)
    );
    assert!(matches!(ProcessTermination::TimedOut, ProcessTermination::TimedOut));
    assert!(matches!(ProcessTermination::SpawnFailed, ProcessTermination::SpawnFailed));
    assert!(matches!(
        ProcessTermination::MemoryLimitExceeded,
        ProcessTermination::MemoryLimitExceeded
    ));
}

#[test]
fn process_termination_is_success() {
    assert!(ProcessTermination::Exited { code: 0 }.is_success());
    assert!(!ProcessTermination::Exited { code: 1 }.is_success());
    assert!(!ProcessTermination::TimedOut.is_success());
}

#[test]
fn process_termination_exit_code() {
    assert_eq!(ProcessTermination::Exited { code: 42 }.exit_code(), Some(42));
    assert!(ProcessTermination::TimedOut.exit_code().is_none());
    assert!(ProcessTermination::SpawnFailed.exit_code().is_none());
}

#[test]
fn process_termination_signaled_validates() {
    assert!(ProcessTermination::signaled(1).is_ok());
    assert!(ProcessTermination::signaled(31).is_ok());
    assert!(ProcessTermination::signaled(0).is_err());
    assert!(ProcessTermination::signaled(32).is_err());
}

#[test]
fn process_termination_serde_round_trip() {
    let terms = [
        ProcessTermination::Exited { code: 0 },
        ProcessTermination::Exited { code: 1 },
        ProcessTermination::Signaled { signal: 9 },
        ProcessTermination::TimedOut,
        ProcessTermination::MemoryLimitExceeded,
        ProcessTermination::SpawnFailed,
    ];
    for term in &terms {
        let json = serde_json::to_string(term).unwrap();
        let back: ProcessTermination = serde_json::from_str(&json).unwrap();
        assert_eq!(*term, back);
    }
}

#[test]
fn lane_failure_variants() {
    let infra = LaneFailure::Infra { tool: "cargo-fmt".to_string(), reason: "missing".to_string() };
    assert!(matches!(infra, LaneFailure::Infra { .. }));

    let tool = LaneFailure::Tool {
        tool: "clippy".to_string(),
        termination: ProcessTermination::Exited { code: 1 },
    };
    assert!(matches!(tool, LaneFailure::Tool { .. }));

    let resource =
        LaneFailure::Resource { tool: "dylint".to_string(), limit: "timeout".to_string() };
    assert!(matches!(resource, LaneFailure::Resource { .. }));

    let suspicious =
        LaneFailure::Suspicious { tool: "ast-grep".to_string(), evidence: "tampered".to_string() };
    assert!(matches!(suspicious, LaneFailure::Suspicious { .. }));
}

#[test]
fn lane_failure_serde_round_trip() {
    let failures = [
        LaneFailure::Infra { tool: "a".to_string(), reason: "r".to_string() },
        LaneFailure::Tool {
            tool: "b".to_string(),
            termination: ProcessTermination::Exited { code: 1 },
        },
        LaneFailure::Resource { tool: "c".to_string(), limit: "l".to_string() },
        LaneFailure::Suspicious { tool: "d".to_string(), evidence: "e".to_string() },
    ];
    for f in &failures {
        let json = serde_json::to_string(f).unwrap();
        let back: LaneFailure = serde_json::from_str(&json).unwrap();
        assert_eq!(*f, back);
    }
}

// ===========================================================================
// CommandEvidence, LaneEvidence
// ===========================================================================

#[test]
fn command_evidence_constructs() {
    let cmd = CommandEvidence::new(
        "cargo".to_string(),
        Box::new(["cargo".to_string(), "fmt".to_string()]),
    )
    .unwrap();
    assert_eq!(cmd.executable(), "cargo");
    assert_eq!(cmd.argv().len(), 2);
}

#[test]
fn command_evidence_rejects_empty_argv() {
    let result = CommandEvidence::new("cargo".to_string(), Box::new([]));
    assert!(result.is_err());
}

#[test]
fn command_evidence_rejects_argv_zero_mismatch() {
    let result = CommandEvidence::new("cargo".to_string(), Box::new(["rustc".to_string()]));
    assert!(result.is_err());
}

#[test]
fn lane_evidence_constructs() {
    let cmd = CommandEvidence::new("cargo".to_string(), Box::new(["cargo".to_string()])).unwrap();
    let evidence = LaneEvidence::new(
        cmd,
        "rustfmt 1.84".to_string(),
        ProcessTermination::Exited { code: 0 },
        Digest::from_bytes(b"evidence"),
    )
    .unwrap();
    assert_eq!(evidence.tool_version(), "rustfmt 1.84");
}

// ===========================================================================
// QualityReceipt, LaneReceipt
// ===========================================================================

#[test]
fn quality_receipt_constructs() {
    let digest = Digest::from_bytes(b"test");
    let receipt = QualityReceipt::new(
        GateScope::Edit,
        ReceiptDigests::new(digest.clone(), digest.clone(), digest.clone(), digest.clone()),
        Box::new([LaneReceipt::new(Lane::Fmt, digest, true)]),
    )
    .unwrap();
    assert_eq!(receipt.schema_version(), 1);
    assert_eq!(*receipt.scope(), GateScope::Edit);
}

#[test]
fn lane_receipt_constructs() {
    let digest = Digest::from_bytes(b"evidence");
    let lr = LaneReceipt::new(Lane::Fmt, digest, true);
    assert_eq!(*lr.lane(), Lane::Fmt);
    assert!(lr.clean());
}

#[test]
fn lane_receipt_serde_round_trip() {
    let digest = Digest::from_bytes(b"lr");
    let lr = LaneReceipt::new(Lane::Compile, digest, false);
    let json = serde_json::to_string(&lr).unwrap();
    let back: LaneReceipt = serde_json::from_str(&json).unwrap();
    assert_eq!(lr, back);
}

#[test]
fn quality_receipt_serde_round_trip() {
    let digest = Digest::from_bytes(b"qr");
    let lr = LaneReceipt::new(Lane::Fmt, digest.clone(), true);
    let receipt = QualityReceipt::new(
        GateScope::Edit,
        ReceiptDigests::new(digest.clone(), digest.clone(), digest.clone(), digest.clone()),
        Box::new([lr]),
    )
    .unwrap();
    let json = serde_json::to_string(&receipt).unwrap();
    let back: QualityReceipt = serde_json::from_str(&json).unwrap();
    assert_eq!(receipt, back);
}

// ===========================================================================
// PolicyDiagnostic, InputDiagnostic, DiagnosticSeverity
// ===========================================================================

#[test]
fn policy_diagnostic_constructs() {
    let d = PolicyDiagnostic {
        message: "bad policy".to_string(),
        file: Some(WorkspacePath::new(".titania/policy.toml").unwrap()),
        severity: DiagnosticSeverity::Error,
    };
    assert_eq!(d.message, "bad policy");
    assert_eq!(d.severity, DiagnosticSeverity::Error);
}

#[test]
fn input_diagnostic_constructs() {
    let d = InputDiagnostic {
        message: "missing workspace".to_string(),
        tool: Some("cargo".to_string()),
        severity: DiagnosticSeverity::Warning,
    };
    assert_eq!(d.message, "missing workspace");
    assert_eq!(d.severity, DiagnosticSeverity::Warning);
}

#[test]
fn diagnostic_severity_variants() {
    assert!(matches!(DiagnosticSeverity::Error, DiagnosticSeverity::Error));
    assert!(matches!(DiagnosticSeverity::Warning, DiagnosticSeverity::Warning));
}

// ===========================================================================
// FindingEffect
// ===========================================================================

#[test]
fn finding_effect_variants() {
    assert!(matches!(FindingEffect::Reject, FindingEffect::Reject));
    assert!(matches!(FindingEffect::Informational, FindingEffect::Informational));
}

#[test]
fn finding_effect_serde_round_trip() {
    let effects = [FindingEffect::Reject, FindingEffect::Informational];
    for e in &effects {
        let json = serde_json::to_string(e).unwrap();
        let back: FindingEffect = serde_json::from_str(&json).unwrap();
        assert_eq!(*e, back);
    }
}
