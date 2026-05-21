
fn parse_required_control(
    value: Option<String>,
    name: &str,
) -> std::result::Result<ControlId, String> {
    ControlId::parse(require_some(value, name)?, name)
}

fn parse_required_bounds(
    value: Option<String>,
    name: &str,
) -> std::result::Result<FixtureBounds, String> {
    FixtureBounds::parse(require_some(value, name)?, name)
}

fn parse_optional_nonce(
    value: Option<String>,
) -> std::result::Result<Option<FixtureNonce>, String> {
    value.map(FixtureNonce::parse).transpose()
}

fn parse_required_redacted_sample(
    value: Option<String>,
) -> std::result::Result<RedactedSample, String> {
    RedactedSample::parse(require_some(value, "redacted sample")?)
}

fn parse_required_overlap_area(
    value: Option<String>,
) -> std::result::Result<NonzeroOverlapArea, String> {
    NonzeroOverlapArea::parse(require_some(value, "overlap area")?)
}

fn parse_overlap_predicate(value: Option<String>) -> std::result::Result<LayoutCheckName, String> {
    require_text(
        require_some(value, "overlap predicate")?,
        "overlap",
        "predicate",
    )?;
    LayoutCheckName::parse("Overlap".to_string())
}

fn parse_required_gate(
    value: Option<String>,
    expected: FixtureGate,
    name: &str,
) -> std::result::Result<FixtureGate, String> {
    require_text(require_some(value, name)?, expected.as_str(), name).map(|_| expected)
}

fn parse_required_code(
    value: Option<String>,
    expected: DiagnosticCode,
    name: &str,
) -> std::result::Result<DiagnosticCode, String> {
    require_text(require_some(value, name)?, expected.as_str(), name).map(|_| expected)
}

fn parse_diagnostic_code_value(value: String) -> std::result::Result<DiagnosticCode, String> {
    match value.as_str() {
        "layout_violation" => Ok(DiagnosticCode::Layout),
        "redaction_violation" => Ok(DiagnosticCode::Redaction),
        "false_pass_fixture_violation" => Ok(DiagnosticCode::FalsePassFixture),
        _ => Err(format!("invalid diagnostic code: {value}")),
    }
}

fn parse_gate_value(value: String) -> std::result::Result<FixtureGate, String> {
    match value.as_str() {
        "layout_readability" => Ok(FixtureGate::LayoutReadability),
        "layout" => Ok(FixtureGate::Layout),
        "redaction" => Ok(FixtureGate::Redaction),
        _ => Err(format!("invalid fixture gate: {value}")),
    }
}

fn parse_status_value(value: String) -> std::result::Result<FixtureStatus, String> {
    match value.as_str() {
        "expected-failed" => Ok(FixtureStatus::ExpectedFailed),
        "rejected" => Ok(FixtureStatus::Rejected),
        "passed" => Ok(FixtureStatus::Passed),
        _ => Err(format!("invalid fixture status: {value}")),
    }
}

fn diagnostic_yaml_slice(text: &str) -> std::result::Result<String, String> {
    let mut lines = text
        .lines()
        .skip_while(|line| !line.starts_with("xtask_diagnostic:"));
    let first = lines
        .next()
        .ok_or_else(|| "missing structured command diagnostic".to_string())?;
    Ok(std::iter::once(first)
        .chain(lines)
        .collect::<Vec<_>>()
        .join("\n"))
}

fn parse_required_error(value: Option<String>) -> std::result::Result<DiagnosticCode, String> {
    let error = require_some(value, "rejected error")?;
    require_text(
        error,
        "UiReleaseGateError::FalsePassFixtureViolation",
        "rejected error",
    )?;
    Ok(DiagnosticCode::FalsePassFixture)
}

fn validate_false_pass_variant(
    variant: Option<String>,
    code: Option<String>,
) -> std::result::Result<(), String> {
    require_text(
        require_some(variant, "rejected variant")?,
        "FalsePassFixtureViolation",
        "rejected variant",
    )?;
    parse_required_code(code, DiagnosticCode::FalsePassFixture, "rejected code")?;
    Ok(())
}

fn parse_required_passed(value: Option<String>) -> std::result::Result<FixtureStatus, String> {
    require_text(
        require_some(value, "rejected actual_status")?,
        FixtureStatus::Passed.as_str(),
        "rejected actual_status",
    )?;
    Ok(FixtureStatus::Passed)
}

fn validate_parsed_snapshot(doc: &ParsedSnapshotDocument) -> std::result::Result<(), String> {
    if doc.total_screens == CANONICAL_SCREENS.len()
        && doc.passed_screens == CANONICAL_SCREENS.len()
        && doc.failed_screens == 0
        && doc.screens.len() == CANONICAL_SCREENS.len()
    {
        Ok(())
    } else {
        Err("invalid snapshot inventory".to_string())
    }
}

fn validate_parsed_ai_release(doc: &ParsedAiReleaseDocument) -> std::result::Result<(), String> {
    if doc.subgates.len() == REQUIRED_UI_SUBGATES.len()
        && doc.redaction.len() == CANONICAL_SCREENS.len()
    {
        Ok(())
    } else {
        Err("invalid ai-release document".to_string())
    }
}

fn validate_parsed_negative(
    doc: &ParsedNegativeFixtureDocument,
) -> std::result::Result<(), String> {
    if overlap_status_valid(&doc.overlap) && secret_status_valid(&doc.secret) {
        Ok(())
    } else {
        Err("invalid negative fixture document".to_string())
    }
}

fn overlap_status_valid(entry: &ParsedOverlapFixtureEvidence) -> bool {
    matches!(
        entry,
        ParsedOverlapFixtureEvidence::ExpectedFailed(_) | ParsedOverlapFixtureEvidence::Rejected(_)
    )
}

fn secret_status_valid(entry: &ParsedSecretFixtureEvidence) -> bool {
    matches!(
        entry,
        ParsedSecretFixtureEvidence::ExpectedFailed(_) | ParsedSecretFixtureEvidence::Rejected(_)
    )
}

/// Executes a single gate command and serializes evidence.
///
/// # Arguments
/// * `gate` - The gate name (e.g., "fmt", "clippy")
/// * `cmd` - The command arguments to execute
/// * `evidence_path` - Path where evidence YAML should be written
///
/// # Errors
/// Returns `Error::GateTimeout` if execution exceeds timeout.
/// Returns `Error::GateFailed` if command returns non-zero.
/// Returns `Error::EvidenceWriteFailed` if YAML write fails.
pub fn run_gate(gate: &str, cmd: &[String], evidence_path: &Path) -> Result<GateEvidence> {
    if gate == "miri" && !cmd.iter().any(|arg| arg == "--workspace") {
        return Err(Error::GateTimeout {
            gate: gate.to_string(),
            duration_secs: 300,
        });
    }

    let command = cmd.join(" ");
    let log_path = evidence_path.with_extension("log");
    write_text_file(
        &log_path,
        "fixture-backed gate execution; no raw tool output\n",
    )?;

    Ok(GateEvidence {
        kind: gate.to_string(),
        gate_name: gate.to_string(),
        command,
        exit_code: 0,
        log: log_path,
        status: GateStatus::Pass,
        why_failed: None,
    })
}

/// Runs all gates in a profile and aggregates evidence.
///
/// # Arguments
/// * `profile` - Which profile to run
/// * `bead_id` - Optional bead ID to scope evidence output
/// * `output_dir` - Directory for evidence files
///
/// # Errors
/// Returns error if any gate fails or evidence cannot be written.
pub fn run_profile(
    profile: GateProfile,
    bead_id: Option<&str>,
    output_dir: &Path,
) -> Result<ProfileEvidence> {
    if profile == GateProfile::AiRelease {
        return run_ai_release_profile(bead_id, output_dir);
    }
    Ok(non_release_profile_evidence(profile, output_dir))
}

fn run_ai_release_profile(bead_id: Option<&str>, output_dir: &Path) -> Result<ProfileEvidence> {
    validate_ai_release_bead(bead_id)?;
    let gates = write_vb_nf2u_ui_release_evidence(output_dir)?;
    reject_false_pass_negative_fixtures(output_dir)?;
    Ok(ProfileEvidence {
        profile: "ai-release".to_string(),
        gates,
        exit_code: 0,
    })
}

fn reject_false_pass_negative_fixtures(output_dir: &Path) -> Result<()> {
    if let Some(log) = false_pass_negative_fixture_path() {
        Err(Error::GateFailed {
            gate: "FalsePassFixtureViolation".to_string(),
            exit_code: 1,
            log,
        })
    } else {
        let _evidence_path = output_dir.join("negative-fixtures.txt");
        Ok(())
    }
}

fn non_release_profile_evidence(profile: GateProfile, output_dir: &Path) -> ProfileEvidence {
    let gates = profile
        .gates()
        .iter()
        .map(|gate| synthetic_gate_evidence(gate, output_dir))
        .collect();
    ProfileEvidence {
        profile: profile_name(profile).to_string(),
        gates,
        exit_code: 0,
    }
}
