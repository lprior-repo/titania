use xtask::evidence::{
    SnapshotDeterminismConfig, UiReleaseGateConfig, UiReleaseGateError, UiReleaseInventory,
    canonical_ui_release_inventory, check_redaction_artifacts, enter_release_snapshot_mode,
    include_ui_gates_in_ai_release, run_ui_negative_fixtures, validate_screen_bijection,
};

#[test]
fn invalid_screen_inventory_error_returns_typed_variant_and_diagnostic() {
    let inventory =
        UiReleaseInventory::from_screen_ids(["execution_overview", "execution_overview"]);
    let result = validate_screen_bijection(&inventory);

    assert_eq!(
        result,
        Err(UiReleaseGateError::InvalidScreenInventory {
            code: "invalid_screen_inventory",
            screen_id_or_count: "execution_overview",
            reason: "duplicate screen id",
            action: "provide each canonical UI release screen exactly once",
        })
    );
}

#[test]
fn unreachable_screen_error_returns_typed_variant_and_diagnostic() {
    let inventory = canonical_ui_release_inventory()
        .map(|inventory| inventory.without_fixture_edge("storage_doctor_ai_context"));
    let result = inventory.and_then(|inventory| validate_screen_bijection(&inventory));

    assert_eq!(
        result,
        Err(UiReleaseGateError::UnreachableScreen {
            code: "unreachable_screen",
            screen_id: "storage_doctor_ai_context",
            mapping_edge: "fixture_id",
            action: "restore one-to-one ShellNav Screen UiScreenKind fixture and report mapping",
        })
    );
}

#[test]
fn snapshot_determinism_violation_returns_typed_variant_and_diagnostic() {
    let config = SnapshotDeterminismConfig::wall_clock_for_screen("execution_overview");
    let result = enter_release_snapshot_mode(config).map(|guard| guard.evidence_marker());

    assert_eq!(
        result,
        Err(UiReleaseGateError::SnapshotDeterminismViolation {
            code: "snapshot_determinism_violation",
            screen_id: "execution_overview",
            expected_field: "snapshot_timestamp",
            expected_value: "2026-05-09T00:00:00Z",
            actual_field: "snapshot_timestamp_source",
            actual_value: "wall_clock",
            action: "set fixed snapshot timestamp before capture",
        })
    );
}

#[test]
fn missing_evidence_error_returns_typed_variant_and_diagnostic() {
    let result = UiReleaseGateConfig::for_bead("vb-nf2u")
        .map(|config| config.without_negative_fixture_evidence())
        .and_then(|config| run_ui_negative_fixtures(config).map(|evidence| evidence.status));

    assert_eq!(
        result,
        Err(UiReleaseGateError::MissingEvidence {
            code: "missing_evidence",
            screen_id: "execution_overview",
            artifact_path: "target/vb-nf2u-negative-fixtures/intentional_overlap_fixture.txt",
            evidence_kind: "negative_fixture",
            action: "create required negative fixture evidence before release",
        })
    );
}

#[test]
fn redaction_violation_error_returns_typed_variant_without_raw_secret_echo() {
    let result = UiReleaseGateConfig::for_bead("vb-nf2u")
        .map(|config| config.with_artifact_text("target/report.yaml", "password=hunter2"))
        .and_then(|evidence| {
            check_redaction_artifacts(&evidence.release_evidence(), &evidence.secret_denylist())
                .map(|redaction| redaction.status)
        });

    assert_eq!(
        result,
        Err(UiReleaseGateError::RedactionViolation {
            code: "redaction_violation",
            screen_id: "execution_overview",
            artifact_path: "target/report.yaml",
            secret_class: "password",
            redacted_sample: "[REDACTED:password]",
            action: "redact raw secret before emitting UI evidence",
        })
    );
}

#[test]
fn false_pass_fixture_violation_returns_typed_variant_and_diagnostic() {
    let result = UiReleaseGateConfig::for_bead("vb-nf2u")
        .map(|config| {
            config.with_negative_fixture_status("intentional_overlap_fixture", "layout", "passed")
        })
        .and_then(|config| run_ui_negative_fixtures(config).map(|evidence| evidence.status));

    assert_eq!(
        result,
        Err(UiReleaseGateError::FalsePassFixtureViolation {
            code: "false_pass_fixture_violation",
            fixture_id: "intentional_overlap_fixture",
            expected_gate: "layout",
            actual_status: "passed",
            action: "fail release because expected-fail negative fixture passed",
        })
    );
}

#[test]
fn release_profile_incomplete_returns_typed_variant_and_diagnostic() {
    let result = include_ui_gates_in_ai_release("vb-nf2u")
        .and_then(|profile| profile.without_subgate("redaction").validate());

    assert_eq!(
        result,
        Err(UiReleaseGateError::ReleaseProfileIncomplete {
            code: "release_profile_incomplete",
            bead_id: "vb-nf2u",
            missing_subgates: vec!["redaction"],
            action: "include all UI release gates in ai-release",
        })
    );
}

#[test]
fn unknown_release_bead_config_fails_closed_before_defaulting() {
    let result = UiReleaseGateConfig::for_bead("vb-nf2u-missing");

    assert_eq!(
        result,
        Err(UiReleaseGateError::ReleaseProfileIncomplete {
            code: "release_profile_incomplete",
            bead_id: "unknown",
            missing_subgates: vec![
                "ui_snapshot",
                "layout_readability",
                "redaction",
                "negative_fixture",
                "deterministic_capture",
                "evidence_shape",
            ],
            action: "reject unknown bead id before generating release evidence",
        })
    );
}

#[test]
fn core_parity_unsupported_returns_typed_variant_when_fixture_evidence_overclaims() {
    let result = include_ui_gates_in_ai_release("vb-nf2u").and_then(|profile| {
        profile
            .with_core_runtime_parity_claim("live-cli-parity")
            .validate()
    });

    assert_eq!(
        result,
        Err(UiReleaseGateError::CoreParityUnsupported {
            code: "core_parity_unsupported",
            claim: "live-cli-parity",
            blocker: "blocked-by-core",
            action: "keep evidence fixture-backed until live Makepad/core parity exists",
        })
    );
}
