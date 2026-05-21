
fn parse_artifact_rect(screen: &str, text: &str, key: &str) -> Result<Rect> {
    let values = parse_artifact_field(screen, text, key)?
        .split(',')
        .map(|value| value.parse::<u32>())
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| rect_parse_error(screen, key))?;
    match values.as_slice() {
        [x, y, w, h] => Rect::new(*x, *y, *w, *h).map_err(|_| rect_parse_error(screen, key)),
        _ => Err(rect_parse_error(screen, key)),
    }
}

fn rect_parse_error(screen: &str, key: &str) -> Error {
    Error::GateFailed {
        gate: format!("layout_readability:{screen}:{key}"),
        exit_code: 1,
        log: PathBuf::from(".evidence/vb-nf2u/ui-layout-report.yaml"),
    }
}

fn unknown_screen_error(screen: &'static str) -> Result<ScreenArtifactFacts> {
    Err(Error::MissingEvidence {
        gate: format!("ui_snapshot:{screen}"),
        path: PathBuf::from(".evidence/vb-nf2u/ui_snapshots/ui_snapshot_report.yaml"),
    })
}

fn digest_artifact_bytes(bytes: &[u8]) -> String {
    format!("blake3:{}", blake3::hash(bytes).to_hex())
}

fn gate_status_from_result(result: &Result<()>) -> GateStatus {
    match result {
        Ok(()) => GateStatus::Pass,
        Err(_) => GateStatus::Fail,
    }
}

fn diagnostics_from_result(result: &Result<()>) -> Vec<&'static str> {
    match result {
        Ok(()) => Vec::new(),
        Err(_) => vec!["typed validation failed"],
    }
}

fn check_outcome_for_kind(facts: &ScreenArtifactFacts, kind: &'static str) -> Result<CheckOutcome> {
    let origin = check_origin_for_kind(kind);
    match kind {
        "Overlap" | "Clipping" | "Bounds" | "ChipReadability" | "SelectedState" => {
            layout_artifact_passed(facts, kind).map(|()| CheckOutcome::passed(origin))
        }
        "FixtureArtifactProvenance" => {
            fixture_artifact_passed(facts).map(|()| CheckOutcome::passed(origin))
        }
        "Redaction" => redaction_artifact_passed(facts).map(|()| CheckOutcome::passed(origin)),
        other => unknown_check_error(other),
    }
}

fn layout_artifact_passed(facts: &ScreenArtifactFacts, kind: &str) -> Result<()> {
    execute_layout_fixture_check(facts, kind).map_err(|_| Error::GateFailed {
        gate: format!("layout_readability:{}:{kind}", facts.screen_id),
        exit_code: 1,
        log: PathBuf::from(".evidence/vb-nf2u/ui-layout-report.yaml"),
    })
}

fn fixture_artifact_passed(facts: &ScreenArtifactFacts) -> Result<()> {
    let computed = digest_artifact_bytes(&facts.provenance.payload.bytes);
    if facts
        .provenance
        .path
        .extension()
        .is_some_and(|ext| ext == "txt")
        && facts.provenance.digest == computed
    {
        Ok(())
    } else {
        missing_check_error(facts, "FixtureArtifactProvenance")
    }
}

fn execute_layout_fixture_check(
    facts: &ScreenArtifactFacts,
    kind: &str,
) -> std::result::Result<(), ()> {
    match kind {
        "Overlap" => require_no_overlap(facts),
        "Clipping" => require_no_clipping(facts),
        "Bounds" => require_in_bounds(facts),
        "ChipReadability" => require_readable_chip(facts),
        "SelectedState" => require_selected_visible(facts),
        _ => Err(()),
    }
}

fn require_no_overlap(facts: &ScreenArtifactFacts) -> std::result::Result<(), ()> {
    overlap_area_px(facts.geometry.left, facts.geometry.right)
        .map_err(|_| ())
        .and_then(no_area)
}

fn require_no_clipping(facts: &ScreenArtifactFacts) -> std::result::Result<(), ()> {
    is_clipped(facts.geometry.container, facts.geometry.label)
        .map_err(|_| ())
        .and_then(require_false)
}

fn require_in_bounds(facts: &ScreenArtifactFacts) -> std::result::Result<(), ()> {
    is_out_of_bounds(facts.geometry.viewport, facts.geometry.control)
        .map_err(|_| ())
        .and_then(require_false)
}

fn require_readable_chip(facts: &ScreenArtifactFacts) -> std::result::Result<(), ()> {
    if chip_is_readable(facts.geometry.chip, 4_500) {
        Ok(())
    } else {
        Err(())
    }
}

fn require_selected_visible(facts: &ScreenArtifactFacts) -> std::result::Result<(), ()> {
    selected_state_is_visible(
        facts.geometry.viewport,
        SelectedIndicator::Visible(facts.geometry.selected_indicator),
    )
    .map_err(|_| ())
    .and_then(require_true)
}

fn no_area(area: u32) -> std::result::Result<(), ()> {
    if area == 0 { Ok(()) } else { Err(()) }
}

fn require_false(value: bool) -> std::result::Result<(), ()> {
    if value { Err(()) } else { Ok(()) }
}

fn require_true(value: bool) -> std::result::Result<(), ()> {
    if value { Ok(()) } else { Err(()) }
}

fn redaction_artifact_passed(facts: &ScreenArtifactFacts) -> Result<()> {
    let artifact_text =
        String::from_utf8(facts.provenance.payload.bytes.clone()).map_err(|_| {
            Error::GateFailed {
                gate: format!("redaction:{}:artifact_utf8", facts.screen_id),
                exit_code: 1,
                log: PathBuf::from(".evidence/vb-nf2u/ai-release.yaml"),
            }
        })?;
    scan_redaction_text(facts.screen_id, &artifact_text)?;
    scan_redaction_text(facts.screen_id, &facts.visible_text)
}

fn unknown_check_error<T>(other: &str) -> Result<T> {
    Err(Error::GateFailed {
        gate: format!("unknown UI check kind: {other}"),
        exit_code: 1,
        log: PathBuf::from(".evidence/vb-nf2u/ui_snapshots/ui_snapshot_report.yaml"),
    })
}

fn missing_check_error<T>(facts: &ScreenArtifactFacts, kind: &str) -> Result<T> {
    Err(Error::MissingEvidence {
        gate: format!("{}:{kind}", facts.screen_id),
        path: PathBuf::from(".evidence/vb-nf2u/ui_snapshots/ui_snapshot_report.yaml"),
    })
}

fn check_origin_for_kind(kind: &str) -> SubgateOrigin {
    match kind {
        "Redaction" => SubgateOrigin::RedactionScan,
        "FixtureArtifactProvenance" => SubgateOrigin::SnapshotInventory,
        _ => SubgateOrigin::LayoutPredicates,
    }
}

fn validate_subgates(subgates: &[UiSubgateRun]) -> Result<()> {
    let present = subgates.iter().map(|gate| gate.name).collect::<Vec<_>>();
    let missing = REQUIRED_UI_SUBGATES
        .iter()
        .copied()
        .filter(|gate| !present.iter().any(|candidate| candidate == gate))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(Error::GateFailed {
            gate: format!("missing UI release subgates: {}", missing.join(",")),
            exit_code: 1,
            log: PathBuf::from(".evidence/vb-nf2u/ai-release.log"),
        })
    }
}

fn validate_screen_rows(screens: &[UiScreenEvidenceRow]) -> Result<()> {
    if screens.len() != CANONICAL_SCREENS.len() {
        return Err(Error::MissingEvidence {
            gate: "ui_snapshot".to_string(),
            path: PathBuf::from(".evidence/vb-nf2u/ui_snapshots/ui_snapshot_report.yaml"),
        });
    }
    for screen in CANONICAL_SCREENS {
        let row = screens.iter().find(|row| row.screen_id == screen);
        match row {
            Some(row) if row.checks.len() == REQUIRED_LAYOUT_CHECKS.len() => {}
            _ => {
                return Err(Error::MissingEvidence {
                    gate: format!("ui_snapshot:{screen}"),
                    path: PathBuf::from(".evidence/vb-nf2u/ui_snapshots/ui_snapshot_report.yaml"),
                });
            }
        }
    }
    Ok(())
}

fn validate_layout_check_rows(screens: &[UiScreenEvidenceRow]) -> Result<()> {
    for screen in screens {
        for required in REQUIRED_LAYOUT_CHECKS {
            match screen.checks.iter().find(|check| check.kind == required) {
                Some(check) if check.outcome.is_passed() => {}
                _ => {
                    return Err(Error::MissingEvidence {
                        gate: format!("layout_readability:{}:{required}", screen.screen_id),
                        path: PathBuf::from(".evidence/vb-nf2u/ui-layout-report.yaml"),
                    });
                }
            }
        }
    }
    Ok(())
}

fn validate_redaction_coverage(screens: &[UiScreenEvidenceRow]) -> Result<()> {
    for screen in screens {
        let artifact =
            String::from_utf8(screen.provenance.payload.bytes.clone()).map_err(|_| {
                Error::GateFailed {
                    gate: format!("redaction:{}:artifact_utf8", screen.screen_id),
                    exit_code: 1,
                    log: PathBuf::from(".evidence/vb-nf2u/ai-release.yaml"),
                }
            })?;
        scan_redaction_text(screen.screen_id, &artifact)?;
        require_redaction_placeholders(screen.screen_id, &artifact)?;
    }
    Ok(())
}

fn redaction_artifact_for_screen(screen_id: &str) -> String {
    let mut text = format!("screen_id: {screen_id}\nraw_matches: 0\n");
    for (class, placeholder) in REDACTION_CLASSES {
        text.push_str("placeholder:");
        text.push_str(class);
        text.push('=');
        text.push_str(placeholder);
        text.push('\n');
    }
    text
}

fn scan_redaction_text(screen_id: &str, text: &str) -> Result<()> {
    for (secret_class, raw_secret, _) in raw_secret_patterns() {
        if text.contains(raw_secret) {
            return Err(Error::GateFailed {
                gate: format!("redaction:{screen_id}:{secret_class}"),
                exit_code: 1,
                log: PathBuf::from(".evidence/vb-nf2u/ai-release.yaml"),
            });
        }
    }
    Ok(())
}
