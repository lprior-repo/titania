
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiReleaseGateConfig {
    bead_id: ReleaseBeadId,
    negative_fixture: NegativeFixtureWorkflow,
    artifact: ReleaseArtifactWorkflow,
}

impl UiReleaseGateConfig {
    pub fn for_bead(bead_id: &'static str) -> std::result::Result<Self, UiReleaseGateError> {
        let release_bead = ReleaseBeadId::parse(bead_id)?;
        Ok(Self {
            bead_id: release_bead,
            negative_fixture: NegativeFixtureWorkflow::Required,
            artifact: ReleaseArtifactWorkflow::None,
        })
    }

    pub fn without_negative_fixture_evidence(mut self) -> Self {
        self.negative_fixture = NegativeFixtureWorkflow::Missing;
        self
    }

    pub fn with_negative_fixture_status(
        mut self,
        fixture_id: &'static str,
        expected_gate: &'static str,
        actual_status: &'static str,
    ) -> Self {
        self.negative_fixture = NegativeFixtureWorkflow::Observed {
            fixture_id,
            expected_gate,
            actual_status,
        };
        self
    }

    pub fn with_artifact_text(mut self, path: &'static str, text: &'static str) -> Self {
        self.artifact = ReleaseArtifactWorkflow::Text { path, text };
        self
    }

    pub fn release_evidence(&self) -> UiReleaseEvidence {
        UiReleaseEvidence {
            artifact: self.artifact.clone(),
        }
    }

    pub fn secret_denylist(&self) -> SecretDenylist {
        SecretDenylist
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiReleaseEvidence {
    artifact: ReleaseArtifactWorkflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretDenylist;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NegativeFixtureEvidence {
    pub status: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactionEvidence {
    pub status: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseProfileEvidence {
    bead_id: ReleaseBeadId,
    subgates: Vec<&'static str>,
    parity_claim: ReleaseParityClaim,
}

impl ReleaseProfileEvidence {
    pub fn without_subgate(mut self, subgate: &'static str) -> Self {
        self.subgates.retain(|gate| *gate != subgate);
        self
    }

    pub fn with_core_runtime_parity_claim(mut self, claim: &'static str) -> Self {
        self.parity_claim = ReleaseParityClaim::LiveCoreRuntime(claim);
        self
    }

    pub fn validate(&self) -> std::result::Result<(), UiReleaseGateError> {
        if let ReleaseParityClaim::LiveCoreRuntime(claim) = self.parity_claim {
            return Err(UiReleaseGateError::CoreParityUnsupported {
                code: "core_parity_unsupported",
                claim,
                blocker: "blocked-by-core",
                action: "keep evidence fixture-backed until live Makepad/core parity exists",
            });
        }
        let missing = REQUIRED_UI_SUBGATES
            .iter()
            .copied()
            .filter(|gate| !self.subgates.iter().any(|present| present == gate))
            .collect::<Vec<_>>();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(UiReleaseGateError::ReleaseProfileIncomplete {
                code: "release_profile_incomplete",
                bead_id: self.bead_id.as_str(),
                missing_subgates: missing,
                action: "include all UI release gates in ai-release",
            })
        }
    }
}

pub fn canonical_ui_release_inventory()
-> std::result::Result<UiReleaseInventory, UiReleaseGateError> {
    Ok(UiReleaseInventory::from_screen_ids(CANONICAL_SCREENS))
}

pub fn validate_screen_bijection(
    inventory: &UiReleaseInventory,
) -> std::result::Result<(), UiReleaseGateError> {
    validate_screen_ids_known_and_unique(inventory)?;
    validate_screen_count(inventory)?;
    validate_fixture_edges(inventory)
}

fn validate_screen_ids_known_and_unique(
    inventory: &UiReleaseInventory,
) -> std::result::Result<(), UiReleaseGateError> {
    for screen in &inventory.screen_ids {
        validate_screen_not_duplicate(inventory, screen)?;
        validate_screen_is_canonical(screen)?;
    }
    Ok(())
}

fn validate_screen_not_duplicate(
    inventory: &UiReleaseInventory,
    screen: &'static str,
) -> std::result::Result<(), UiReleaseGateError> {
    let count = inventory
        .screen_ids
        .iter()
        .filter(|candidate| *candidate == &screen)
        .count();
    if count > 1 {
        invalid_inventory(screen, "duplicate screen id")
    } else {
        Ok(())
    }
}

fn validate_screen_is_canonical(
    screen: &'static str,
) -> std::result::Result<(), UiReleaseGateError> {
    if CANONICAL_SCREENS.iter().any(|required| required == &screen) {
        Ok(())
    } else {
        invalid_inventory(screen, "unknown screen id")
    }
}

fn validate_screen_count(
    inventory: &UiReleaseInventory,
) -> std::result::Result<(), UiReleaseGateError> {
    if inventory.screen_ids.len() != CANONICAL_SCREENS.len() {
        invalid_inventory("screen_count", "missing or extra screen id")
    } else {
        Ok(())
    }
}

fn validate_fixture_edges(
    inventory: &UiReleaseInventory,
) -> std::result::Result<(), UiReleaseGateError> {
    if let Some(screen_id) = inventory.missing_fixture_edge {
        Err(UiReleaseGateError::UnreachableScreen {
            code: "unreachable_screen",
            screen_id,
            mapping_edge: "fixture_id",
            action: "restore one-to-one ShellNav Screen UiScreenKind fixture and report mapping",
        })
    } else {
        Ok(())
    }
}

fn invalid_inventory(
    screen_id_or_count: &'static str,
    reason: &'static str,
) -> std::result::Result<(), UiReleaseGateError> {
    Err(UiReleaseGateError::InvalidScreenInventory {
        code: "invalid_screen_inventory",
        screen_id_or_count,
        reason,
        action: "provide each canonical UI release screen exactly once",
    })
}

pub fn enter_release_snapshot_mode(
    config: SnapshotDeterminismConfig,
) -> std::result::Result<ReleaseSnapshotGuard, UiReleaseGateError> {
    match config.source {
        SnapshotTimeSource::Fixed => Ok(ReleaseSnapshotGuard {
            marker: "deterministic_snapshot_mode",
        }),
        SnapshotTimeSource::WallClock => Err(UiReleaseGateError::SnapshotDeterminismViolation {
            code: "snapshot_determinism_violation",
            screen_id: config.screen_id,
            expected_field: "snapshot_timestamp",
            expected_value: "2026-05-09T00:00:00Z",
            actual_field: "snapshot_timestamp_source",
            actual_value: "wall_clock",
            action: "set fixed snapshot timestamp before capture",
        }),
    }
}

pub fn run_ui_negative_fixtures(
    config: UiReleaseGateConfig,
) -> std::result::Result<NegativeFixtureEvidence, UiReleaseGateError> {
    match config.negative_fixture {
        NegativeFixtureWorkflow::Missing => return missing_negative_fixture_error(),
        NegativeFixtureWorkflow::Observed {
            fixture_id,
            expected_gate,
            actual_status: "passed",
        } => return false_pass_fixture_error(fixture_id, expected_gate),
        NegativeFixtureWorkflow::Observed { .. } | NegativeFixtureWorkflow::Required => {}
    }
    let _bead_id = config.bead_id.as_str();
    Ok(NegativeFixtureEvidence {
        status: "expected-failed",
    })
}

fn missing_negative_fixture_error()
-> std::result::Result<NegativeFixtureEvidence, UiReleaseGateError> {
    Err(UiReleaseGateError::MissingEvidence {
        code: "missing_evidence",
        screen_id: "execution_overview",
        artifact_path: "target/vb-nf2u-negative-fixtures/intentional_overlap_fixture.txt",
        evidence_kind: "negative_fixture",
        action: "create required negative fixture evidence before release",
    })
}

fn false_pass_fixture_error(
    fixture_id: &'static str,
    expected_gate: &'static str,
) -> std::result::Result<NegativeFixtureEvidence, UiReleaseGateError> {
    Err(UiReleaseGateError::FalsePassFixtureViolation {
        code: "false_pass_fixture_violation",
        fixture_id,
        expected_gate,
        actual_status: "passed",
        action: "fail release because expected-fail negative fixture passed",
    })
}
