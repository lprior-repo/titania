//! Behaviour tests for pure report assembly from in-memory `LaneOutcome` values.
//!
//! The production function `titania_aggregate::assemble_report` is the pure
//! classification boundary between lane outcomes, diagnostics, and `Report`.
//!
//! Contracts defended:
//! 1. Code findings → `Report::Reject { code_findings, gate_failures: [] }` with
//!    `RejectKind::CodeOnly`.
//! 2. Gate failures → `Report::Reject { code_findings: [], gate_failures }` with
//!    `RejectKind::GateOnly`.
//! 3. Both present → `Report::Reject` with `RejectKind::Mixed`.
//! 4. `Report::Pass` when every required lane outcome is Clean or Skipped + valid receipt.
//! 5. Policy / input diagnostics are emitted as `PolicyError` / `InputError`,
//!    never encoded into a `Reject`.
//! 6. Empty outcomes without diagnostics return `Err(ReportAssemblyError::EmptyOutcomes)`.
//! 7. Informational-only findings do NOT reject — yields Pass with receipt.
//! 8. Diagnostics take precedence even when no lane outcome was produced.

use titania_aggregate::{ReportAssemblyError, assemble_report};
use titania_core::{
    Digest, Finding, GateScope, InputDiagnostic, Lane, LaneEvidence, LaneFailure, LaneOutcome,
    LaneReceipt, PolicyDiagnostic, ProcessTermination, QualityReceiptV1, ReceiptDigests,
    RepairHint, RuleId, TextRange, WorkspacePath,
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

/// Minimal helper: build a single `LaneOutcome::Clean` for the given lane.
fn clean_outcome(_lane: Lane) -> LaneOutcome {
    LaneOutcome::Clean { evidence: clean_evidence() }
}

/// Minimal helper: build a `LaneOutcome::Findings` with one rejecting finding.
fn findings_outcome(lane: Lane) -> LaneOutcome {
    let rule_id = RuleId::new("TEST_LINT_VIOLATION").unwrap();
    let loc = WorkspacePath::new("src/main.rs").unwrap();
    let location = titania_core::Location::span(loc, 1, 0, 1, 10).unwrap();
    let repair =
        RepairHint::patch("src/main.rs".into(), TextRange::new(0, 10).unwrap(), "// fix".into());
    let finding = Finding::reject(lane, rule_id, location, "lint violation".into(), repair);
    LaneOutcome::Findings { findings: Box::from([finding]) }
}

/// Minimal helper: build a `LaneOutcome::Findings` with one informational finding only.
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

/// Minimal helper: build a `LaneOutcome::Failed` with an infra failure.
fn infra_failure_outcome(tool: &str, reason: &str) -> LaneOutcome {
    LaneOutcome::Failed { failure: LaneFailure::Infra { tool: tool.into(), reason: reason.into() } }
}

/// Minimal helper: build a skipped outcome.
fn skipped_outcome() -> LaneOutcome {
    LaneOutcome::Skipped { reason: titania_core::SkipReason::NotSelectedByScope }
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

// ── Test 1: code findings produce Reject with CodeOnly ──────────────────────

#[test]
fn code_findings_yield_reject_codonly() {
    let scope = GateScope::Edit;
    let outcomes: Box<[titania_core::PerLaneEntry]> = Box::from(
        scope
            .lanes()
            .iter()
            .copied()
            .enumerate()
            .map(|(index, lane)| {
                if index == 0 {
                    titania_core::PerLaneEntry::new(lane, findings_outcome(lane))
                } else {
                    titania_core::PerLaneEntry::new(lane, clean_outcome(lane))
                }
            })
            .collect::<Vec<_>>(),
    );
    let policy_diags: Box<[PolicyDiagnostic]> = Box::from([]);
    let input_diags: Box<[InputDiagnostic]> = Box::from([]);

    let report = assemble_report(scope, outcomes, test_receipt(scope), policy_diags, input_diags)
        .expect("report assembled");

    // Must be a Reject, not Pass.
    assert!(report.is_reject(), "report must be Reject");
    let code_findings = report.code_findings().expect("Reject must have code_findings");
    let gate_failures = report.gate_failures().expect("Reject must have gate_failures");
    assert!(!code_findings.is_empty(), "code_findings must be non-empty");
    assert!(gate_failures.is_empty(), "gate_failures must be empty for CodeOnly");

    assert_eq!(
        report.reject_kind(),
        Some(titania_core::RejectKind::CodeOnly),
        "RejectKind must be CodeOnly"
    );

    assert_eq!(code_findings.len(), 1);
    assert_eq!(code_findings[0].message(), "lint violation");
    assert_eq!(code_findings[0].lane(), Lane::Fmt);
}

// ── Test 2: gate failures produce Reject with GateOnly ──────────────────────

#[test]
fn gate_failures_yield_reject_gateonly() {
    let scope = GateScope::Edit;
    let outcomes: Box<[titania_core::PerLaneEntry]> = Box::from(
        scope
            .lanes()
            .iter()
            .copied()
            .enumerate()
            .map(|(index, lane)| {
                if index == 0 {
                    titania_core::PerLaneEntry::new(
                        lane,
                        infra_failure_outcome("clippy", "binary not found"),
                    )
                } else {
                    titania_core::PerLaneEntry::new(lane, clean_outcome(lane))
                }
            })
            .collect::<Vec<_>>(),
    );
    let policy_diags: Box<[PolicyDiagnostic]> = Box::from([]);
    let input_diags: Box<[InputDiagnostic]> = Box::from([]);

    let report = assemble_report(scope, outcomes, test_receipt(scope), policy_diags, input_diags)
        .expect("report assembled");

    assert!(report.is_reject(), "report must be Reject");
    let code_findings = report.code_findings().expect("Reject must have code_findings");
    let gate_failures = report.gate_failures().expect("Reject must have gate_failures");
    assert!(code_findings.is_empty(), "code_findings must be empty for GateOnly");
    assert_eq!(gate_failures.len(), 1, "must have one gate failure");
    assert!(gate_failures[0].is_infra(), "failure must be infra type");

    assert_eq!(
        report.reject_kind(),
        Some(titania_core::RejectKind::GateOnly),
        "RejectKind must be GateOnly"
    );
}
// ── Test 3: both findings and failures produce Reject with Mixed ────────────

#[test]
fn both_findings_and_failures_yield_reject_mixed() {
    let scope = GateScope::Edit;
    let outcomes: Box<[titania_core::PerLaneEntry]> = Box::from(
        scope
            .lanes()
            .iter()
            .copied()
            .enumerate()
            .map(|(index, lane)| match index {
                0 => titania_core::PerLaneEntry::new(lane, findings_outcome(lane)),
                1 => titania_core::PerLaneEntry::new(
                    lane,
                    infra_failure_outcome("dylint", "ABI mismatch"),
                ),
                _ => titania_core::PerLaneEntry::new(lane, clean_outcome(lane)),
            })
            .collect::<Vec<_>>(),
    );
    let policy_diags: Box<[PolicyDiagnostic]> = Box::from([]);
    let input_diags: Box<[InputDiagnostic]> = Box::from([]);

    let report = assemble_report(scope, outcomes, test_receipt(scope), policy_diags, input_diags)
        .expect("report assembled");

    assert!(report.is_reject(), "report must be Reject");
    let code_findings = report.code_findings().expect("Reject must have code_findings");
    let gate_failures = report.gate_failures().expect("Reject must have gate_failures");
    assert_eq!(
        report.reject_kind(),
        Some(titania_core::RejectKind::Mixed),
        "RejectKind must be Mixed"
    );
    assert_eq!(code_findings.len(), 1, "must contain the code finding");
    assert_eq!(gate_failures.len(), 1, "must contain the infra failure");
}

// ── Test 4: Pass when every scope lane is Clean + valid receipt ─────────────

#[test]
fn all_scope_lanes_clean_yields_pass() {
    let scope = GateScope::Edit;
    let outcomes: Box<[titania_core::PerLaneEntry]> = Box::from(
        scope
            .lanes()
            .iter()
            .copied()
            .map(|lane| titania_core::PerLaneEntry::new(lane, clean_outcome(lane)))
            .collect::<Vec<_>>(),
    );
    let policy_diags: Box<[PolicyDiagnostic]> = Box::from([]);
    let input_diags: Box<[InputDiagnostic]> = Box::from([]);
    let receipt = test_receipt(scope);

    let report = assemble_report(scope, outcomes, receipt, policy_diags, input_diags).unwrap();

    assert!(report.is_pass(), "report must be Pass");
    let per_lane = report.per_lane().expect("Pass must have per_lane");
    assert_eq!(per_lane.len(), scope.lanes().len());
    for outcome in per_lane.iter() {
        assert!(outcome.outcome().is_pass());
    }
}
// ── Test 5: skipped lanes do not prevent Pass ───────────────────────────────

#[test]
fn skipped_lanes_do_not_prevent_pass() {
    let scope = GateScope::Edit;
    let outcomes: Vec<titania_core::PerLaneEntry> = scope
        .lanes()
        .iter()
        .copied()
        .enumerate()
        .map(|(index, lane)| {
            if index == 0 {
                titania_core::PerLaneEntry::new(lane, skipped_outcome())
            } else {
                titania_core::PerLaneEntry::new(lane, clean_outcome(lane))
            }
        })
        .collect::<Vec<_>>();

    let outcomes: Box<[titania_core::PerLaneEntry]> = Box::from(outcomes);
    let policy_diags: Box<[PolicyDiagnostic]> = Box::from([]);
    let input_diags: Box<[InputDiagnostic]> = Box::from([]);
    let receipt = test_receipt(scope);

    let report = assemble_report(scope, outcomes, receipt, policy_diags, input_diags)
        .expect("report assembled");

    assert!(report.is_pass(), "report must be Pass (skipped lanes ok)");
}

// ── Test 6: policy diagnostics produce PolicyError, not Reject ──────────────

#[test]
fn policy_diagnostics_yield_policy_error_not_reject() {
    let scope = GateScope::Edit;
    let outcomes: Box<[titania_core::PerLaneEntry]> =
        Box::from([titania_core::PerLaneEntry::new(Lane::Fmt, clean_outcome(Lane::Fmt))]);
    let policy_diags: Box<[PolicyDiagnostic]> = Box::from([PolicyDiagnostic {
        message: "missing required tool config".into(),
        file: None,
        severity: titania_core::DiagnosticSeverity::Error,
    }]);
    let input_diags: Box<[InputDiagnostic]> = Box::from([]);
    let receipt = test_receipt(scope);

    let report = assemble_report(scope, outcomes, receipt, policy_diags, input_diags)
        .expect("report assembled");

    assert!(report.is_policy_error(), "expected PolicyError");
    let diagnostics = report.policy_diagnostics().expect("PolicyError must have diagnostics");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message, "missing required tool config");
}

// ── Test 7: input diagnostics produce InputError, not Reject ────────────────

#[test]
fn input_diagnostics_yield_input_error_not_reject() {
    let scope = GateScope::Edit;
    let outcomes: Box<[titania_core::PerLaneEntry]> =
        Box::from([titania_core::PerLaneEntry::new(Lane::Fmt, clean_outcome(Lane::Fmt))]);
    let policy_diags: Box<[PolicyDiagnostic]> = Box::from([]);
    let input_diags: Box<[InputDiagnostic]> = Box::from([InputDiagnostic::new(
        "invalid gate scope".into(),
        Some("prepush".into()),
        titania_core::DiagnosticSeverity::Error,
    )]);
    let receipt = test_receipt(scope);

    let report = assemble_report(scope, outcomes, receipt, policy_diags, input_diags)
        .expect("report assembled");

    assert!(report.is_input_error(), "expected InputError");
    let diagnostics = report.input_diagnostics().expect("InputError must have diagnostics");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message, "invalid gate scope");
}

// ── Test 8: empty outcomes return exact error, never Pass ───────────────────

#[test]
fn empty_outcomes_return_error_not_pass() {
    let scope = GateScope::Edit;
    let outcomes: Box<[titania_core::PerLaneEntry]> = Box::from([]);
    let policy_diags: Box<[PolicyDiagnostic]> = Box::from([]);
    let input_diags: Box<[InputDiagnostic]> = Box::from([]);

    let result = assemble_report(scope, outcomes, test_receipt(scope), policy_diags, input_diags);

    // Must return Err(EmptyOutcomes), never Ok(Pass) or Ok(Reject).
    // Report::pass rejects empty per_lane and Report::reject rejects empty collections,
    // so the only valid outcome is an error.
    match result {
        Ok(report) => panic!("empty outcomes must NOT yield Report::Ok, got {:?}", report),
        Err(e) => {
            // Must be the EmptyOutcomes error variant.
            assert!(
                matches!(e, ReportAssemblyError::EmptyOutcomes),
                "expected ReportAssemblyError::EmptyOutcomes, got {:?}",
                e
            );
        }
    }
}

// ── Test 9: informational findings do not reject ────────────────────────────

#[test]
fn informational_findings_do_not_reject() {
    let scope = GateScope::Edit;
    let outcomes: Box<[titania_core::PerLaneEntry]> = Box::from(
        scope
            .lanes()
            .iter()
            .copied()
            .map(|lane| titania_core::PerLaneEntry::new(lane, informational_outcome(lane)))
            .collect::<Vec<_>>(),
    );
    let policy_diags: Box<[PolicyDiagnostic]> = Box::from([]);
    let input_diags: Box<[InputDiagnostic]> = Box::from([]);
    let receipt = test_receipt(scope);

    let report = assemble_report(scope, outcomes, receipt, policy_diags, input_diags)
        .expect("report assembled");

    // Informational findings should NOT produce Reject — yields Pass with receipt.
    assert!(report.is_pass(), "informational findings must NOT reject");
}

// ── Test 10: policy diagnostics take precedence over findings ───────────────

#[test]
fn policy_diagnostics_precede_findings_in_report() {
    let scope = GateScope::Edit;
    let outcomes: Box<[titania_core::PerLaneEntry]> =
        Box::from([titania_core::PerLaneEntry::new(Lane::Fmt, findings_outcome(Lane::Fmt))]);
    let policy_diags: Box<[PolicyDiagnostic]> = Box::from([PolicyDiagnostic {
        message: "misconfigured scope".into(),
        file: None,
        severity: titania_core::DiagnosticSeverity::Error,
    }]);
    let input_diags: Box<[InputDiagnostic]> = Box::from([]);
    let receipt = test_receipt(scope);

    let report = assemble_report(scope, outcomes, receipt, policy_diags, input_diags)
        .expect("report assembled");

    // PolicyError should take precedence: even though findings exist,
    // the report should be PolicyError, not Reject.
    assert!(
        report.is_policy_error(),
        "expected PolicyError (policy takes precedence over findings)"
    );
}
// ── Test 11: diagnostic-only outcomes preserve diagnostic variants ─────────

#[test]
fn empty_outcomes_with_input_diagnostics_yield_input_error() {
    let scope = GateScope::Edit;
    let outcomes: Box<[titania_core::PerLaneEntry]> = Box::from([]);
    let policy_diags: Box<[PolicyDiagnostic]> = Box::from([]);
    let input_diags: Box<[InputDiagnostic]> = Box::from([InputDiagnostic::new(
        "invalid gate scope".into(),
        Some("prepush".into()),
        titania_core::DiagnosticSeverity::Error,
    )]);

    let report = assemble_report(scope, outcomes, test_receipt(scope), policy_diags, input_diags)
        .expect("input diagnostics must produce a report");

    assert!(report.is_input_error(), "diagnostic-only input must be InputError");
    assert_eq!(report.input_diagnostics().expect("InputError must carry diagnostics").len(), 1);
}

#[test]
fn empty_outcomes_with_policy_diagnostics_yield_policy_error() {
    let scope = GateScope::Edit;
    let outcomes: Box<[titania_core::PerLaneEntry]> = Box::from([]);
    let policy_diags: Box<[PolicyDiagnostic]> = Box::from([PolicyDiagnostic {
        message: "misconfigured scope".into(),
        file: None,
        severity: titania_core::DiagnosticSeverity::Error,
    }]);
    let input_diags: Box<[InputDiagnostic]> = Box::from([]);

    let report = assemble_report(scope, outcomes, test_receipt(scope), policy_diags, input_diags)
        .expect("policy diagnostics must produce a report");

    assert!(report.is_policy_error(), "diagnostic-only policy must be PolicyError");
    assert_eq!(report.policy_diagnostics().expect("PolicyError must carry diagnostics").len(), 1);
}

// ── Lane identity tests: per-lane outcomes ──────────────────────────────
// These tests defend the new identity check: `assemble_report` rejects
// outcomes whose lane sequence does not match `GateScope::lanes()` exactly
// (no duplicates, no omissions, no substitutions, no reorderings). The
// `test_receipt` helper produces a valid receipt, so each test below
// isolates the divergence in the outcome list.

/// Build outcomes in the canonical order of `scope`. Helpers below
/// derive alternate orderings from this base.
fn canonical_clean_outcomes(scope: GateScope) -> Vec<titania_core::PerLaneEntry> {
    scope
        .lanes()
        .iter()
        .copied()
        .map(|lane| titania_core::PerLaneEntry::new(lane, clean_outcome(lane)))
        .collect()
}

/// Build a `QualityReceiptV1` with an arbitrary list of lanes (used to
/// inject misordering/duplication into the receipt). All receipts carry
/// dummy digests and a `clean` flag — those fields are not validated by
/// assembly.
fn receipt_for_lanes(scope: GateScope, lanes: &[Lane]) -> QualityReceiptV1 {
    let digests = ReceiptDigests::new(
        Digest::from_bytes(&[1u8; 32]),
        Digest::from_bytes(&[2u8; 32]),
        Digest::from_bytes(&[3u8; 32]),
        Digest::from_bytes(&[4u8; 32]),
    );
    let lane_receipts: Box<[LaneReceipt]> = lanes
        .iter()
        .copied()
        .map(|lane| LaneReceipt::new(lane, Digest::from_bytes(&[0u8; 16]), true))
        .collect::<Vec<_>>()
        .into_boxed_slice();
    QualityReceiptV1::new(scope, digests, lane_receipts).unwrap()
}

#[test]
fn duplicate_lane_in_outcomes_is_rejected_with_identity_mismatch() {
    let scope = GateScope::Edit;
    let mut entries = canonical_clean_outcomes(scope);
    // Replace the SECOND entry (index 1 = Compile) with a duplicate of
    // the FIRST entry's lane (Fmt). Outcome count and per-slot shape
    // still look valid — only the lane label diverges.
    let first_lane = scope.lanes()[0];
    entries[1] = titania_core::PerLaneEntry::new(first_lane, clean_outcome(first_lane));
    let outcomes: Box<[titania_core::PerLaneEntry]> = entries.into_boxed_slice();
    let receipt = test_receipt(scope);

    let result = assemble_report(scope, outcomes, receipt, Box::from([]), Box::from([]));

    match result {
        Ok(report) => panic!("duplicate lane must NOT yield a report, got {:?}", report),
        Err(ReportAssemblyError::LaneIdentityMismatch { scope: s, index, expected, found }) => {
            assert_eq!(s, scope);
            assert_eq!(index, 1, "divergence at index 1 (duplicate of index 0)");
            assert_eq!(expected, scope.lanes()[1]);
            assert_eq!(found, Some(scope.lanes()[0]));
        }
        Err(other) => panic!("expected LaneIdentityMismatch, got {:?}", other),
    }
}

#[test]
fn wrong_lane_in_outcomes_is_rejected_with_identity_mismatch() {
    let scope = GateScope::Edit;
    let mut entries = canonical_clean_outcomes(scope);
    // Substitute a known-valid scope lane that does not match the
    // required lane at that index. The substitute is in `scope.lanes()`,
    // so the count and set still look fine — only the slot is wrong.
    let wrong_lane = scope.lanes()[3];
    entries[2] = titania_core::PerLaneEntry::new(wrong_lane, clean_outcome(wrong_lane));
    let outcomes: Box<[titania_core::PerLaneEntry]> = entries.into_boxed_slice();
    let receipt = test_receipt(scope);

    let result = assemble_report(scope, outcomes, receipt, Box::from([]), Box::from([]));

    match result {
        Ok(report) => panic!("wrong lane must NOT yield a report, got {:?}", report),
        Err(ReportAssemblyError::LaneIdentityMismatch { scope: s, index, expected, found }) => {
            assert_eq!(s, scope);
            assert_eq!(index, 2);
            assert_eq!(expected, scope.lanes()[2]);
            assert_eq!(found, Some(wrong_lane));
        }
        Err(other) => panic!("expected LaneIdentityMismatch, got {:?}", other),
    }
}

#[test]
fn reordered_valid_lanes_in_outcomes_is_rejected_with_identity_mismatch() {
    let scope = GateScope::Edit;
    let canonical: Vec<Lane> = scope.lanes().to_vec();
    // Rotate: move the first lane to the end, keep the rest in order.
    // Every lane is still a valid scope lane, but the sequence no longer
    // matches `scope.lanes()` exactly.
    let mut reordered = canonical.clone();
    let first = reordered.remove(0);
    reordered.push(first);
    let outcomes: Box<[titania_core::PerLaneEntry]> = reordered
        .iter()
        .copied()
        .map(|lane| titania_core::PerLaneEntry::new(lane, clean_outcome(lane)))
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let receipt = test_receipt(scope);

    let result = assemble_report(scope, outcomes, receipt, Box::from([]), Box::from([]));

    match result {
        Ok(report) => panic!("reordered lanes must NOT yield a report, got {:?}", report),
        Err(ReportAssemblyError::LaneIdentityMismatch { scope: s, index, expected, found }) => {
            assert_eq!(s, scope);
            assert_eq!(index, 0, "first divergence at index 0");
            assert_eq!(expected, canonical[0]);
            assert_eq!(found, Some(reordered[0]));
            assert_ne!(expected, reordered[0], "reorder must differ at index 0");
        }
        Err(other) => panic!("expected LaneIdentityMismatch, got {:?}", other),
    }
}

// ── Lane identity tests: receipt lanes ──────────────────────────────────
// The receipt's lane list must also match `GateScope::lanes()` exactly
// when a pass is requested. Each test below uses canonical outcomes so
// only the receipt diverges.

#[test]
fn duplicate_lane_in_receipt_is_rejected_with_identity_mismatch() {
    let scope = GateScope::Edit;
    let outcomes: Box<[titania_core::PerLaneEntry]> =
        canonical_clean_outcomes(scope).into_boxed_slice();
    // Build a receipt that duplicates the first lane in place of the
    // second — receipt count still matches, but the per-slot lane is
    // wrong.
    let canonical: Vec<Lane> = scope.lanes().to_vec();
    let mut wrong_lanes = canonical.clone();
    wrong_lanes[1] = canonical[0];
    let receipt = receipt_for_lanes(scope, &wrong_lanes);

    let result = assemble_report(scope, outcomes, receipt, Box::from([]), Box::from([]));

    match result {
        Ok(report) => panic!("duplicate receipt lane must NOT yield a report, got {:?}", report),
        Err(ReportAssemblyError::ReceiptLaneIdentityMismatch {
            scope: s,
            index,
            expected,
            found,
        }) => {
            assert_eq!(s, scope);
            assert_eq!(index, 1);
            assert_eq!(expected, canonical[1]);
            assert_eq!(found, Some(canonical[0]));
        }
        Err(other) => panic!("expected ReceiptLaneIdentityMismatch, got {:?}", other),
    }
}

#[test]
fn wrong_lane_in_receipt_is_rejected_with_identity_mismatch() {
    let scope = GateScope::Edit;
    let outcomes: Box<[titania_core::PerLaneEntry]> =
        canonical_clean_outcomes(scope).into_boxed_slice();
    let canonical: Vec<Lane> = scope.lanes().to_vec();
    // Substitute lane[3] in place of lane[2] in the receipt.
    let mut wrong_lanes = canonical.clone();
    let substitute = canonical[3];
    wrong_lanes[2] = substitute;
    let receipt = receipt_for_lanes(scope, &wrong_lanes);

    let result = assemble_report(scope, outcomes, receipt, Box::from([]), Box::from([]));

    match result {
        Ok(report) => panic!("wrong receipt lane must NOT yield a report, got {:?}", report),
        Err(ReportAssemblyError::ReceiptLaneIdentityMismatch {
            scope: s,
            index,
            expected,
            found,
        }) => {
            assert_eq!(s, scope);
            assert_eq!(index, 2);
            assert_eq!(expected, canonical[2]);
            assert_eq!(found, Some(substitute));
        }
        Err(other) => panic!("expected ReceiptLaneIdentityMismatch, got {:?}", other),
    }
}

#[test]
fn reordered_valid_lanes_in_receipt_is_rejected_with_identity_mismatch() {
    let scope = GateScope::Edit;
    let outcomes: Box<[titania_core::PerLaneEntry]> =
        canonical_clean_outcomes(scope).into_boxed_slice();
    let canonical: Vec<Lane> = scope.lanes().to_vec();
    // Rotate the receipt's lane list: move the first lane to the end.
    let mut reordered = canonical.clone();
    let first = reordered.remove(0);
    reordered.push(first);
    let receipt = receipt_for_lanes(scope, &reordered);

    let result = assemble_report(scope, outcomes, receipt, Box::from([]), Box::from([]));

    match result {
        Ok(report) => panic!("reordered receipt lanes must NOT yield a report, got {:?}", report),
        Err(ReportAssemblyError::ReceiptLaneIdentityMismatch {
            scope: s,
            index,
            expected,
            found,
        }) => {
            assert_eq!(s, scope);
            assert_eq!(index, 0);
            assert_eq!(expected, canonical[0]);
            assert_eq!(found, Some(reordered[0]));
        }
        Err(other) => panic!("expected ReceiptLaneIdentityMismatch, got {:?}", other),
    }
}
