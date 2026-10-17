//! Behaviour tests for the versioned required-versus-optional obligation contract.
//!
//! The production entry point
//! `titania_aggregate::assemble_report_with_obligations` is the aggregation
//! boundary where a mandatory lane must prove execution.
//!
//! Contracts defended:
//! 1. Required lane `Skipped` (`NotApplicable`) + otherwise clean → Reject.
//! 2. Required lane `Skipped` (`ToolUnavailable`) + otherwise clean → Reject.
//! 3. Legacy v1 `assemble_report` still passes with a skip (compat pin).
//! 4. `ObligationSet::v1_legacy` through the new entry point still passes.
//! 5. Optional-lane skip through the new entry point still passes.
//! 6. All-required + all-clean + valid receipt → Pass.
//! 7. Required lane `Failed` with `TimedOut` termination → Reject.
//! 8. Missing required lane → `Err`, never a pass receipt.
//! 9. `requiring` rejects out-of-scope and duplicate lanes with typed errors.
//! 10. Required skip + rejecting finding → Reject `Mixed` carrying both.
//! 11. Informational-only findings on a required lane still pass.

use titania_aggregate::{
    ObligationError, ObligationSet, ReportAssemblyError, assemble_report,
    assemble_report_with_obligations,
};
use titania_core::{
    Digest, Finding, GateScope, InputDiagnostic, Lane, LaneEvidence, LaneFailure, LaneOutcome,
    LaneReceipt, PerLaneEntry, PolicyDiagnostic, ProcessTermination, QualityReceiptV1,
    ReceiptDigests, RepairHint, RuleId, SkipReason, TextRange, ToolKind, WorkspacePath,
};

/// Minimal helper: build a `LaneEvidence` for a clean lane.
fn clean_evidence() -> LaneEvidence {
    let cmd = titania_core::CommandEvidence::new(
        "cargo".into(),
        vec!["cargo".to_string(), "fmt".to_string(), "--check".to_string()].into_boxed_slice(),
    )
    .unwrap();
    LaneEvidence::new(
        cmd,
        "1.0.0".into(),
        ProcessTermination::Exited { code: 0 },
        Digest::from_bytes(&[0u8; 16]),
    )
    .unwrap()
}

/// Minimal helper: build a `LaneOutcome::Clean`.
fn clean_outcome() -> LaneOutcome {
    LaneOutcome::Clean { evidence: clean_evidence() }
}

/// Minimal helper: build a `LaneOutcome::Skipped` with the given reason.
fn skipped_outcome(reason: SkipReason) -> LaneOutcome {
    LaneOutcome::Skipped { reason }
}

/// Assert the obligation set exposes its scope, order, and kinds.
fn assert_contract_accessors(obligations: &ObligationSet, scope: GateScope) {
    use titania_aggregate::ObligationKind;
    assert_eq!(obligations.scope(), scope);
    assert_eq!(obligations.required_lanes(), scope.lanes());
    for lane in scope.lanes().iter().copied() {
        assert!(obligations.is_required(lane));
        assert_eq!(obligations.kind(lane), ObligationKind::Required);
    }
    assert!(ObligationSet::v1_legacy(scope).required_lanes().is_empty());
}

/// Minimal helper: build a `LaneOutcome::Failed` with a timeout termination.
fn timeout_outcome(tool: &str) -> LaneOutcome {
    LaneOutcome::Failed {
        failure: LaneFailure::Tool { tool: tool.into(), termination: ProcessTermination::TimedOut },
    }
}

/// Minimal helper: build a `LaneOutcome::Findings` with one rejecting finding.
fn rejecting_outcome(lane: Lane) -> LaneOutcome {
    let rule_id = RuleId::new("TEST_LINT_VIOLATION").unwrap();
    let loc = WorkspacePath::new("src/main.rs").unwrap();
    let location = titania_core::Location::span(loc, 1, 0, 1, 10).unwrap();
    let repair =
        RepairHint::patch("src/main.rs".into(), TextRange::new(0, 10).unwrap(), "// fix".into());
    let finding = Finding::reject(lane, rule_id, location, "lint violation".into(), repair);
    LaneOutcome::Findings { findings: Box::from([finding]) }
}

/// Minimal helper: build a `LaneOutcome::Findings` with one informational finding.
fn informational_outcome(lane: Lane) -> LaneOutcome {
    let rule_id = RuleId::new("TEST_STYLE_NOTE").unwrap();
    let loc = WorkspacePath::new("src/main.rs").unwrap();
    let location = titania_core::Location::span(loc, 1, 0, 1, 10).unwrap();
    let repair = RepairHint::patch(
        "src/main.rs".into(),
        TextRange::new(0, 10).unwrap(),
        "// style fix".into(),
    );
    let finding = Finding::informational(lane, rule_id, location, "style note".into(), repair);
    LaneOutcome::Findings { findings: Box::from([finding]) }
}

/// Build a minimal `QualityReceiptV1` for all lanes in the given scope.
fn test_receipt(scope: GateScope) -> QualityReceiptV1 {
    let digests = ReceiptDigests::new(
        Digest::from_bytes(&[1u8; 32]),
        Digest::from_bytes(&[2u8; 32]),
        Digest::from_bytes(&[3u8; 32]),
        Digest::from_bytes(&[4u8; 32]),
    );
    let lane_receipts: Box<[LaneReceipt]> = scope
        .lanes()
        .iter()
        .copied()
        .map(|lane| {
            let evidence_digest = Digest::from_bytes(&[0u8; 16]);
            LaneReceipt::new(lane, evidence_digest, true)
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    QualityReceiptV1::new(scope, digests, lane_receipts).unwrap()
}

/// Canonical clean entries for `scope`, with entry `at` replaced by `outcome`.
fn entries_with(scope: GateScope, at: usize, outcome: LaneOutcome) -> Box<[PerLaneEntry]> {
    scope
        .lanes()
        .iter()
        .copied()
        .enumerate()
        .map(|(index, lane)| {
            if index == at {
                PerLaneEntry::new(lane, outcome.clone())
            } else {
                PerLaneEntry::new(lane, clean_outcome())
            }
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

/// Canonical all-clean entries for `scope`.
fn clean_entries(scope: GateScope) -> Box<[PerLaneEntry]> {
    scope
        .lanes()
        .iter()
        .copied()
        .map(|lane| PerLaneEntry::new(lane, clean_outcome()))
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

/// Assert the single gate failure records the required-lane skip.
fn assert_required_skip_failure(failures: &[LaneFailure], lane: Lane) {
    assert_eq!(failures.len(), 1, "exactly one gate failure expected");
    match &failures[0] {
        LaneFailure::Infra { tool, reason } => {
            assert_eq!(tool, lane.name(), "failure must name the skipped lane");
            assert!(
                reason.contains("required lane skipped"),
                "reason must name the contract rule, got: {reason}"
            );
        }
        other => panic!("expected Infra gate failure for required skip, got {other:?}"),
    }
}

// ── Test 1: required NotApplicable skip rejects ────────────────────────────

#[test]
fn required_skip_not_applicable_rejects() {
    let scope = GateScope::Edit;
    let obligations = ObligationSet::all_required(scope);
    let outcomes = entries_with(scope, 0, skipped_outcome(SkipReason::NotApplicable));

    let report = assemble_report_with_obligations(
        &obligations,
        outcomes,
        test_receipt(scope),
        Box::from([]),
        Box::from([]),
    )
    .expect("report assembled");

    assert!(report.is_reject(), "required skip must reject, not pass");
    assert_eq!(report.reject_kind(), Some(titania_core::RejectKind::GateOnly));
    let failures = report.gate_failures().expect("Reject must carry gate failures");
    assert_required_skip_failure(failures, Lane::Fmt);
}

// ── Test 2: required ToolUnavailable skip rejects ──────────────────────────

#[test]
fn required_skip_tool_unavailable_rejects() {
    let scope = GateScope::Full;
    let obligations = ObligationSet::all_required(scope);
    let kani_index =
        scope.lanes().iter().position(|lane| *lane == Lane::Kani).expect("Full has Kani");
    let outcomes = entries_with(
        scope,
        kani_index,
        skipped_outcome(SkipReason::ToolUnavailable(ToolKind::CargoKani)),
    );

    let report = assemble_report_with_obligations(
        &obligations,
        outcomes,
        test_receipt(scope),
        Box::from([]),
        Box::from([]),
    )
    .expect("report assembled");

    assert!(report.is_reject(), "unproven proof lane must reject, not pass");
    let failures = report.gate_failures().expect("Reject must carry gate failures");
    assert_required_skip_failure(failures, Lane::Kani);
}

// ── Test 3: legacy v1 entry point still passes with a skip ──────────────────

#[test]
fn legacy_assemble_report_still_passes_with_skip() {
    let scope = GateScope::Edit;
    let outcomes = entries_with(scope, 0, skipped_outcome(SkipReason::NotSelectedByScope));
    let policy_diags: Box<[PolicyDiagnostic]> = Box::from([]);
    let input_diags: Box<[InputDiagnostic]> = Box::from([]);

    let report = assemble_report(scope, outcomes, test_receipt(scope), policy_diags, input_diags)
        .expect("report assembled");

    assert!(report.is_pass(), "v1 compatibility: optional skip still passes");
}

// ── Test 4: v1_legacy set through the new entry point still passes ──────────

#[test]
fn legacy_obligation_set_still_passes_with_skip() {
    let scope = GateScope::Edit;
    let obligations = ObligationSet::v1_legacy(scope);
    let outcomes = entries_with(scope, 0, skipped_outcome(SkipReason::NotApplicable));

    let report = assemble_report_with_obligations(
        &obligations,
        outcomes,
        test_receipt(scope),
        Box::from([]),
        Box::from([]),
    )
    .expect("report assembled");

    assert!(report.is_pass(), "v1_legacy set must preserve v1 skip semantics");
}

// ── Test 5: optional-lane skip still passes ──────────────────────────────────

#[test]
fn optional_skip_still_passes() {
    let scope = GateScope::Edit;
    // Every lane except Fmt is required; Fmt stays optional legacy work.
    let required: Vec<Lane> =
        scope.lanes().iter().copied().filter(|lane| *lane != Lane::Fmt).collect();
    let obligations = ObligationSet::requiring(scope, &required).expect("subset of scope is valid");
    let outcomes = entries_with(scope, 0, skipped_outcome(SkipReason::NotApplicable));

    let report = assemble_report_with_obligations(
        &obligations,
        outcomes,
        test_receipt(scope),
        Box::from([]),
        Box::from([]),
    )
    .expect("report assembled");

    assert!(report.is_pass(), "optional skip must still pass");
}

// ── Test 6: all-required + all-clean passes ─────────────────────────────────

#[test]
fn all_required_all_clean_passes() {
    let scope = GateScope::Edit;
    let obligations = ObligationSet::all_required(scope);
    assert_contract_accessors(&obligations, scope);

    let report = assemble_report_with_obligations(
        &obligations,
        clean_entries(scope),
        test_receipt(scope),
        Box::from([]),
        Box::from([]),
    )
    .expect("report assembled");

    assert!(report.is_pass(), "proven mandatory lanes must pass");
}

// ── Test 7: required timeout failure rejects ────────────────────────────────

#[test]
fn required_timeout_failure_rejects() {
    let scope = GateScope::Full;
    let obligations = ObligationSet::all_required(scope);
    let kani_index =
        scope.lanes().iter().position(|lane| *lane == Lane::Kani).expect("Full has Kani");
    let outcomes = entries_with(scope, kani_index, timeout_outcome("cargo-kani"));

    let report = assemble_report_with_obligations(
        &obligations,
        outcomes,
        test_receipt(scope),
        Box::from([]),
        Box::from([]),
    )
    .expect("report assembled");

    assert!(report.is_reject(), "timed-out mandatory lane must reject");
    assert_eq!(report.reject_kind(), Some(titania_core::RejectKind::GateOnly));
}

// ── Test 8: missing required lane yields error, never pass ──────────────────

#[test]
fn missing_required_lane_yields_error_not_pass() {
    let scope = GateScope::Edit;
    let obligations = ObligationSet::all_required(scope);
    let mut entries: Vec<PerLaneEntry> = clean_entries(scope).into_vec();
    let _removed = entries.pop();
    let outcomes: Box<[PerLaneEntry]> = entries.into_boxed_slice();

    match assemble_report_with_obligations(
        &obligations,
        outcomes,
        test_receipt(scope),
        Box::from([]),
        Box::from([]),
    ) {
        Ok(report) => panic!("missing lane must NOT yield a report, got {report:?}"),
        Err(error) => assert!(
            matches!(error, ReportAssemblyError::LaneIdentityMismatch { .. }),
            "expected LaneIdentityMismatch, got {error:?}"
        ),
    }
}

// ── Test 9: requiring rejects out-of-scope and duplicate lanes ──────────────

#[test]
fn requiring_rejects_unknown_and_duplicate_lanes() {
    // Test is not in Edit scope.
    match ObligationSet::requiring(GateScope::Edit, &[Lane::Test]) {
        Ok(_) => panic!("out-of-scope required lane must be rejected"),
        Err(error) => assert!(
            matches!(error, ObligationError::LaneNotInScope { .. }),
            "expected LaneNotInScope, got {error:?}"
        ),
    }
    match ObligationSet::requiring(GateScope::Edit, &[Lane::Fmt, Lane::Fmt]) {
        Ok(_) => panic!("duplicate required lane must be rejected"),
        Err(error) => assert!(
            matches!(error, ObligationError::DuplicateLane { .. }),
            "expected DuplicateLane, got {error:?}"
        ),
    }
}

// ── Test 10: required skip + rejecting finding yields Mixed ─────────────────

#[test]
fn required_skip_plus_reject_finding_yields_mixed() {
    let scope = GateScope::Edit;
    let obligations = ObligationSet::all_required(scope);
    let outcomes: Box<[PerLaneEntry]> = scope
        .lanes()
        .iter()
        .copied()
        .enumerate()
        .map(|(index, lane)| {
            if index == 0 {
                PerLaneEntry::new(lane, skipped_outcome(SkipReason::NotApplicable))
            } else if index == 1 {
                PerLaneEntry::new(lane, rejecting_outcome(lane))
            } else {
                PerLaneEntry::new(lane, clean_outcome())
            }
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();

    let report = assemble_report_with_obligations(
        &obligations,
        outcomes,
        test_receipt(scope),
        Box::from([]),
        Box::from([]),
    )
    .expect("report assembled");

    assert!(report.is_reject());
    assert_eq!(report.reject_kind(), Some(titania_core::RejectKind::Mixed));
    let findings = report.code_findings().expect("Mixed must carry code findings");
    assert_eq!(findings.len(), 1);
    let failures = report.gate_failures().expect("Mixed must carry gate failures");
    assert_required_skip_failure(failures, Lane::Fmt);
}

// ── Test 11: informational findings on a required lane still pass ───────────

#[test]
fn informational_findings_on_required_still_pass() {
    let scope = GateScope::Full;
    let obligations = ObligationSet::all_required(scope);
    let kani_index =
        scope.lanes().iter().position(|lane| *lane == Lane::Kani).expect("Full has Kani");
    let outcomes = entries_with(scope, kani_index, informational_outcome(Lane::Kani));

    let report = assemble_report_with_obligations(
        &obligations,
        outcomes,
        test_receipt(scope),
        Box::from([]),
        Box::from([]),
    )
    .expect("report assembled");

    assert!(report.is_pass(), "proven lane with advisories must still pass");
}
