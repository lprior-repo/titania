
fn build_subgate_rows(screens: &[UiScreenEvidenceRow]) -> Result<Vec<UiSubgateRun>> {
    let outcomes = subgate_outcomes(screens);
    let rows = outcomes
        .iter()
        .map(|(name, origin, result)| UiSubgateRun::from_result(name, *origin, result))
        .collect();
    require_all_outcomes(outcomes)?;
    Ok(rows)
}

type SubgateOutcome = (&'static str, SubgateOrigin, Result<()>);

fn subgate_outcomes(screens: &[UiScreenEvidenceRow]) -> [SubgateOutcome; 6] {
    [
        snapshot_subgate(screens),
        layout_subgate(screens),
        redaction_subgate(screens),
        negative_fixture_subgate(),
        deterministic_subgate(screens),
        evidence_shape_subgate(screens),
    ]
}

fn snapshot_subgate(screens: &[UiScreenEvidenceRow]) -> SubgateOutcome {
    (
        "ui_snapshot",
        SubgateOrigin::SnapshotInventory,
        validate_screen_rows(screens),
    )
}

fn layout_subgate(screens: &[UiScreenEvidenceRow]) -> SubgateOutcome {
    (
        "layout_readability",
        SubgateOrigin::LayoutPredicates,
        validate_layout_check_rows(screens),
    )
}

fn redaction_subgate(screens: &[UiScreenEvidenceRow]) -> SubgateOutcome {
    (
        "redaction",
        SubgateOrigin::RedactionScan,
        validate_redaction_coverage(screens),
    )
}

fn negative_fixture_subgate() -> SubgateOutcome {
    (
        "negative_fixture",
        SubgateOrigin::NegativeFixtures,
        validate_negative_fixture_inputs(),
    )
}

fn deterministic_subgate(screens: &[UiScreenEvidenceRow]) -> SubgateOutcome {
    (
        "deterministic_capture",
        SubgateOrigin::DeterministicCapture,
        validate_deterministic_capture_state(screens),
    )
}

fn evidence_shape_subgate(screens: &[UiScreenEvidenceRow]) -> SubgateOutcome {
    (
        "evidence_shape",
        SubgateOrigin::EvidenceShape,
        validate_screen_rows(screens),
    )
}

fn require_all_outcomes(outcomes: [SubgateOutcome; 6]) -> Result<()> {
    for (_, _, outcome) in outcomes {
        outcome?;
    }
    Ok(())
}

fn screen_evidence_row(screen: &'static str, snapshot_dir: &Path) -> Result<UiScreenEvidenceRow> {
    let facts = ScreenArtifactFacts::read_for_screen(screen, snapshot_dir)?;
    let checks = build_check_rows(&facts)?;
    Ok(UiScreenEvidenceRow::from_facts(&facts, checks))
}

fn build_check_rows(facts: &ScreenArtifactFacts) -> Result<Vec<UiCheckEvidenceRow>> {
    let checks = REQUIRED_LAYOUT_CHECKS
        .iter()
        .map(|kind| check_row_for_kind(facts, kind))
        .collect::<Result<Vec<_>>>()?;
    Ok(checks)
}

fn check_row_for_kind(
    facts: &ScreenArtifactFacts,
    kind: &'static str,
) -> Result<UiCheckEvidenceRow> {
    let outcome = check_outcome_for_kind(facts, kind)?;
    Ok(UiCheckEvidenceRow {
        kind,
        outcome,
        diagnostics: Vec::new(),
    })
}

impl UiSubgateRun {
    fn from_result(name: &'static str, origin: SubgateOrigin, result: &Result<()>) -> Self {
        Self {
            name,
            command: "cargo xtask ai-release --bead vb-nf2u",
            origin,
            status: gate_status_from_result(result),
            diagnostics: diagnostics_from_result(result),
        }
    }
}

impl ScreenArtifactFacts {
    fn from_source_fixture(source: &SourceFixtureArtifact) -> Result<Self> {
        let provenance = ReadArtifactProvenance::from_payload(
            source.output_path.clone(),
            source.payload.clone(),
        );
        Self::from_provenance(source.screen_id, provenance)
    }

    fn read_for_screen(screen: &'static str, snapshot_dir: &Path) -> Result<Self> {
        if !CANONICAL_SCREENS
            .iter()
            .any(|candidate| candidate == &screen)
        {
            return unknown_screen_error(screen);
        }
        let path = snapshot_dir.join(format!("{screen}.fixture.txt"));
        let provenance = ReadArtifactProvenance::read(path)?;
        Self::from_provenance(screen, provenance)
    }

    fn from_provenance(screen: &'static str, provenance: ReadArtifactProvenance) -> Result<Self> {
        let text =
            String::from_utf8(provenance.payload.bytes.clone()).map_err(|_| Error::GateFailed {
                gate: format!("ui_snapshot:{screen}:artifact_utf8"),
                exit_code: 1,
                log: provenance.path.clone(),
            })?;
        Ok(Self {
            screen_id: screen,
            timestamp: parse_capture_timestamp(screen, &text)?,
            animation_state: parse_animation_state(screen, &text)?,
            clock_source: parse_clock_source(screen, &text)?,
            visible_text: parse_artifact_field(screen, &text, "visible_text")?.to_string(),
            geometry: ScreenGeometry::parse(screen, &text)?,
            provenance,
        })
    }
}

impl UiScreenEvidenceRow {
    fn from_facts(facts: &ScreenArtifactFacts, checks: Vec<UiCheckEvidenceRow>) -> Self {
        Self {
            screen_id: facts.screen_id,
            fixture_id: facts.screen_id,
            artifact_path: facts.provenance.path.display().to_string(),
            digest: facts.provenance.digest.clone(),
            provenance: facts.provenance.clone(),
            checks,
        }
    }
}

impl ReadArtifactProvenance {
    fn from_payload(path: PathBuf, payload: ArtifactPayload) -> Self {
        let digest = digest_artifact_bytes(&payload.bytes);
        Self {
            path,
            digest,
            payload,
        }
    }

    fn read(path: PathBuf) -> Result<Self> {
        let bytes = fs::read(&path).map_err(|_| Error::MissingEvidence {
            gate: "ui_snapshot:artifact_read".to_string(),
            path: path.clone(),
        })?;
        Ok(Self::from_payload(path, ArtifactPayload { bytes }))
    }
}

impl ArtifactPayload {
    fn for_screen(screen: &str) -> Result<Self> {
        let path = checked_source_fixture_path(screen)?;
        let bytes = fs::read(&path).map_err(|_| Error::MissingEvidence {
            gate: format!("source_fixture:{screen}"),
            path,
        })?;
        Ok(Self { bytes })
    }
}

fn checked_source_fixture_path(screen: &str) -> Result<PathBuf> {
    if !CANONICAL_SCREENS
        .iter()
        .any(|candidate| candidate == &screen)
    {
        return Err(Error::MissingEvidence {
            gate: format!("source_fixture:{screen}"),
            path: PathBuf::from("fixtures/vb-nf2u-ui"),
        });
    }
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("vb-nf2u-ui")
        .join(format!("{screen}.fixture.txt")))
}

impl ScreenGeometry {
    fn parse(screen: &'static str, text: &str) -> Result<Self> {
        Ok(Self {
            left: parse_artifact_rect(screen, text, "left_rect")?,
            right: parse_artifact_rect(screen, text, "right_rect")?,
            container: parse_artifact_rect(screen, text, "container_rect")?,
            label: parse_artifact_rect(screen, text, "label_rect")?,
            viewport: parse_artifact_rect(screen, text, "viewport_rect")?,
            control: parse_artifact_rect(screen, text, "control_rect")?,
            chip: parse_artifact_rect(screen, text, "chip_rect")?,
            selected_indicator: parse_artifact_rect(screen, text, "selected_rect")?,
        })
    }
}

fn parse_artifact_field<'a>(screen: &str, text: &'a str, key: &str) -> Result<&'a str> {
    text.lines()
        .find_map(|line| {
            line.strip_prefix(key)
                .and_then(|tail| tail.strip_prefix('='))
        })
        .ok_or_else(|| Error::MissingEvidence {
            gate: format!("ui_snapshot:{screen}:{key}"),
            path: PathBuf::from(".evidence/vb-nf2u/ui_snapshots"),
        })
}

fn parse_capture_timestamp(screen: &str, text: &str) -> Result<CaptureTimestamp> {
    require_artifact_value(screen, text, "snapshot_timestamp", "2026-05-09T00:00:00Z")?;
    Ok(CaptureTimestamp::Fixed("2026-05-09T00:00:00Z"))
}

fn parse_animation_state(screen: &str, text: &str) -> Result<HiddenAnimationState> {
    require_artifact_value(screen, text, "hidden_animation_state", "Paused")?;
    Ok(HiddenAnimationState::Paused)
}

fn parse_clock_source(screen: &str, text: &str) -> Result<ClockSource> {
    require_artifact_value(screen, text, "clock_source", "FixedFixtureTime")?;
    Ok(ClockSource::FixedFixtureTime)
}

fn require_artifact_value(screen: &str, text: &str, key: &str, expected: &str) -> Result<()> {
    let actual = parse_artifact_field(screen, text, key)?;
    if actual == expected {
        Ok(())
    } else {
        artifact_value_error(screen, key)
    }
}

fn artifact_value_error(screen: &str, key: &str) -> Result<()> {
    Err(Error::GateFailed {
        gate: format!("deterministic_capture:{screen}:{key}"),
        exit_code: 1,
        log: PathBuf::from(".evidence/vb-nf2u/determinism.txt"),
    })
}
