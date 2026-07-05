//! Behaviour tests for pure report assembly from in-memory `LaneOutcome` values.
//!
//! The production function `titania_aggregate::assemble_report` does not yet exist;
//! these tests name the intended API and fail at compile time, proving the API gap.
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
//! 6. Empty outcomes return `Err(ReportAssemblyError::EmptyOutcomes)`, never `Ok(Pass)`.
//! 7. Informational-only findings do NOT reject — yields Pass with receipt.
//! 8. Empty outcomes + diagnostics still returns `EmptyOutcomes` (guard before diagnostics).

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
        RepairHint::patch("src/main.rs".into(), TextRange::new(0, 10).unwrap(), "// fix".into())
            .unwrap();
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
    )
    .unwrap();
    let finding = Finding::informational(lane, rule_id, location, "style note".into(), repair);
    LaneOutcome::Findings { findings: Box::from([finding]) }
}

/// Minimal helper: build a `LaneOutcome::Failed` with an infra failure.
fn infra_failure_outcome(tool: &str, reason: &str) -> LaneOutcome {
    LaneOutcome::Failed(LaneFailure::Infra { tool: tool.into(), reason: reason.into() })
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
    let outcomes: Box<[LaneOutcome]> = Box::from(
        scope
            .lanes()
            .iter()
            .copied()
            .enumerate()
            .map(
                |(index, lane)| {
                    if index == 0 { findings_outcome(lane) } else { clean_outcome(lane) }
                },
            )
            .collect::<Vec<_>>(),
    );
    let policy_diags: Box<[PolicyDiagnostic]> = Box::from([]);
    let input_diags: Box<[InputDiagnostic]> = Box::from([]);

    let report = assemble_report(scope, outcomes, test_receipt(scope), policy_diags, input_diags)
        .expect("report assembled");

    // Must be a Reject, not Pass.
    match &report {
        titania_core::Report::Reject { code_findings, gate_failures, .. } => {
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
        other => panic!("expected Report::Reject, got {:?}", other),
    }
}

// ── Test 2: gate failures produce Reject with GateOnly ──────────────────────

#[test]
fn gate_failures_yield_reject_gateonly() {
    let scope = GateScope::Edit;
    let outcomes: Box<[LaneOutcome]> = Box::from(
        scope
            .lanes()
            .iter()
            .copied()
            .enumerate()
            .map(|(index, lane)| {
                if index == 0 {
                    infra_failure_outcome("clippy", "binary not found")
                } else {
                    clean_outcome(lane)
                }
            })
            .collect::<Vec<_>>(),
    );
    let policy_diags: Box<[PolicyDiagnostic]> = Box::from([]);
    let input_diags: Box<[InputDiagnostic]> = Box::from([]);

    let report = assemble_report(scope, outcomes, test_receipt(scope), policy_diags, input_diags)
        .expect("report assembled");

    match &report {
        titania_core::Report::Reject { code_findings, gate_failures, .. } => {
            assert!(code_findings.is_empty(), "code_findings must be empty for GateOnly");
            assert_eq!(gate_failures.len(), 1, "must have one gate failure");
            assert!(gate_failures[0].is_infra(), "failure must be infra type");

            assert_eq!(
                report.reject_kind(),
                Some(titania_core::RejectKind::GateOnly),
                "RejectKind must be GateOnly"
            );
        }
        other => panic!("expected Report::Reject, got {:?}", other),
    }
}

// ── Test 3: both findings and failures produce Reject with Mixed ────────────

#[test]
fn both_findings_and_failures_yield_reject_mixed() {
    let scope = GateScope::Edit;
    let outcomes: Box<[LaneOutcome]> = Box::from(
        scope
            .lanes()
            .iter()
            .copied()
            .enumerate()
            .map(|(index, lane)| match index {
                0 => findings_outcome(lane),
                1 => infra_failure_outcome("dylint", "ABI mismatch"),
                _ => clean_outcome(lane),
            })
            .collect::<Vec<_>>(),
    );
    let policy_diags: Box<[PolicyDiagnostic]> = Box::from([]);
    let input_diags: Box<[InputDiagnostic]> = Box::from([]);

    let report = assemble_report(scope, outcomes, test_receipt(scope), policy_diags, input_diags)
        .expect("report assembled");

    match &report {
        titania_core::Report::Reject { code_findings, gate_failures, .. } => {
            assert_eq!(
                report.reject_kind(),
                Some(titania_core::RejectKind::Mixed),
                "RejectKind must be Mixed"
            );
            assert_eq!(code_findings.len(), 1, "must contain the code finding");
            assert_eq!(gate_failures.len(), 1, "must contain the infra failure");
        }
        other => panic!("expected Report::Reject, got {:?}", other),
    }
}

// ── Test 4: Pass when every scope lane is Clean + valid receipt ─────────────

#[test]
fn all_scope_lanes_clean_yields_pass() {
    let scope = GateScope::Edit;
    let outcomes: Box<[LaneOutcome]> =
        Box::from(scope.lanes().iter().copied().map(clean_outcome).collect::<Vec<_>>());
    let policy_diags: Box<[PolicyDiagnostic]> = Box::from([]);
    let input_diags: Box<[InputDiagnostic]> = Box::from([]);
    let receipt = test_receipt(scope);

    let report = assemble_report(scope, outcomes, receipt, policy_diags, input_diags)
        .expect("report assembled");

    match &report {
        titania_core::Report::Pass { per_lane, .. } => {
            assert_eq!(per_lane.len(), scope.lanes().len());
            for outcome in per_lane.iter() {
                assert!(outcome.is_pass());
            }
        }
        other => panic!("expected Report::Pass, got {:?}", other),
    }
}

// ── Test 5: skipped lanes do not prevent Pass ───────────────────────────────

#[test]
fn skipped_lanes_do_not_prevent_pass() {
    let scope = GateScope::Edit;
    let outcomes: Vec<LaneOutcome> = scope
        .lanes()
        .iter()
        .copied()
        .enumerate()
        .map(|(index, lane)| if index == 0 { skipped_outcome() } else { clean_outcome(lane) })
        .collect::<Vec<_>>();

    let outcomes: Box<[LaneOutcome]> = Box::from(outcomes);
    let policy_diags: Box<[PolicyDiagnostic]> = Box::from([]);
    let input_diags: Box<[InputDiagnostic]> = Box::from([]);
    let receipt = test_receipt(scope);

    let report = assemble_report(scope, outcomes, receipt, policy_diags, input_diags)
        .expect("report assembled");

    match &report {
        titania_core::Report::Pass { .. } => {}
        other => panic!("expected Report::Pass (skipped lanes ok), got {:?}", other),
    }
}

// ── Test 6: policy diagnostics produce PolicyError, not Reject ──────────────

#[test]
fn policy_diagnostics_yield_policy_error_not_reject() {
    let scope = GateScope::Edit;
    let outcomes: Box<[LaneOutcome]> = Box::from([clean_outcome(Lane::Fmt)]);
    let policy_diags: Box<[PolicyDiagnostic]> = Box::from([PolicyDiagnostic {
        message: "missing required tool config".into(),
        file: None,
        severity: titania_core::DiagnosticSeverity::Error,
    }]);
    let input_diags: Box<[InputDiagnostic]> = Box::from([]);
    let receipt = test_receipt(scope);

    let report = assemble_report(scope, outcomes, receipt, policy_diags, input_diags)
        .expect("report assembled");

    match &report {
        titania_core::Report::PolicyError { diagnostics } => {
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(diagnostics[0].message, "missing required tool config");
        }
        other => panic!(
            "expected Report::PolicyError, got {:?} (policy diags must not encode into Reject)",
            other
        ),
    }
}

// ── Test 7: input diagnostics produce InputError, not Reject ────────────────

#[test]
fn input_diagnostics_yield_input_error_not_reject() {
    let scope = GateScope::Edit;
    let outcomes: Box<[LaneOutcome]> = Box::from([clean_outcome(Lane::Fmt)]);
    let policy_diags: Box<[PolicyDiagnostic]> = Box::from([]);
    let input_diags: Box<[InputDiagnostic]> = Box::from([InputDiagnostic::new(
        "invalid gate scope".into(),
        Some("prepush".into()),
        titania_core::DiagnosticSeverity::Error,
    )]);
    let receipt = test_receipt(scope);

    let report = assemble_report(scope, outcomes, receipt, policy_diags, input_diags)
        .expect("report assembled");

    match &report {
        titania_core::Report::InputError { diagnostics } => {
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(diagnostics[0].message, "invalid gate scope");
        }
        other => panic!(
            "expected Report::InputError, got {:?} (input diags must not encode into Reject)",
            other
        ),
    }
}

// ── Test 8: empty outcomes return exact error, never Pass ───────────────────

#[test]
fn empty_outcomes_return_error_not_pass() {
    let scope = GateScope::Edit;
    let outcomes: Box<[LaneOutcome]> = Box::from([]);
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
    let outcomes: Box<[LaneOutcome]> = Box::from(
        scope.lanes().iter().copied().map(|lane| informational_outcome(lane)).collect::<Vec<_>>(),
    );
    let policy_diags: Box<[PolicyDiagnostic]> = Box::from([]);
    let input_diags: Box<[InputDiagnostic]> = Box::from([]);
    let receipt = test_receipt(scope);

    let report = assemble_report(scope, outcomes, receipt, policy_diags, input_diags)
        .expect("report assembled");

    // Informational findings should NOT produce Reject — yields Pass with receipt.
    match &report {
        titania_core::Report::Pass { .. } => {}
        other => panic!(
            "informational findings must NOT reject — expected Report::Pass, got {:?}",
            other
        ),
    }
}

// ── Test 10: policy diagnostics take precedence over findings ───────────────

#[test]
fn policy_diagnostics_precede_findings_in_report() {
    let scope = GateScope::Edit;
    let outcomes: Box<[LaneOutcome]> = Box::from([findings_outcome(Lane::Fmt)]);
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
    match &report {
        titania_core::Report::PolicyError { .. } => {}
        other => panic!(
            "expected Report::PolicyError (policy takes precedence over findings), got {:?}",
            other
        ),
    }
}
// ── Test 11: empty outcomes + diagnostics still returns EmptyOutcomes ───────

#[test]
fn empty_outcomes_with_diagnostics_returns_empty_outcomes_error() {
    let scope = GateScope::Edit;
    let outcomes: Box<[LaneOutcome]> = Box::from([]);
    let policy_diags: Box<[PolicyDiagnostic]> = Box::from([PolicyDiagnostic {
        message: "misconfigured scope".into(),
        file: None,
        severity: titania_core::DiagnosticSeverity::Error,
    }]);
    let input_diags: Box<[InputDiagnostic]> = Box::from([InputDiagnostic::new(
        "invalid gate scope".into(),
        Some("prepush".into()),
        titania_core::DiagnosticSeverity::Error,
    )]);

    let result = assemble_report(scope, outcomes, test_receipt(scope), policy_diags, input_diags);

    // The empty-outcomes guard must run before diagnostics.
    // Even with policy/input diagnostics present, result must be EmptyOutcomes.
    match result {
        Ok(report) => {
            panic!("empty outcomes with diagnostics must NOT yield Report::Ok, got {:?}", report);
        }
        Err(e) => {
            assert!(
                matches!(e, ReportAssemblyError::EmptyOutcomes),
                "expected ReportAssemblyError::EmptyOutcomes (guard before diagnostics), got {:?}",
                e
            );
        }
    }
}
