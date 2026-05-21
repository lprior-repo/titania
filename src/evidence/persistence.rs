
fn write_bytes_file(path: &Path, content: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| Error::EvidenceWriteFailed {
            gate: "ui-release".to_string(),
            path: parent.to_path_buf(),
            cause: error.to_string(),
        })?;
    }
    fs::write(path, content).map_err(|error| Error::EvidenceWriteFailed {
        gate: "ui-release".to_string(),
        path: path.to_path_buf(),
        cause: error.to_string(),
    })
}

/// Generates a `WhyFailed` diagnostic from a failed gate evidence.
///
/// # Arguments
/// * `evidence` - The evidence for a failed gate
///
/// # Returns
/// `WhyFailed` with gate_name, hint, and repair_command populated.
/// Returns `None` if the gate did not fail.
pub fn explain_failure(evidence: &GateEvidence) -> Option<WhyFailed> {
    match evidence.status {
        GateStatus::Fail => {
            let mut why_failed = WhyFailed {
                gate_name: evidence.gate_name.clone(),
                hint: failure_hint(&evidence.gate_name).to_string(),
                repair_command: failure_repair_command(&evidence.gate_name).to_string(),
                variant: None,
                fixture_id: None,
                expected_gate: None,
            };
            // Embed variant-specific diagnostic fields for false-pass errors.
            // The log path identifies the actual failing fixture (overlap vs secret).
            if evidence.gate_name == "FalsePassFixtureViolation" {
                let (variant, fixture_id, expected_gate) =
                    false_pass_diagnostic_for_path(&evidence.log);
                why_failed.variant = Some(variant);
                why_failed.fixture_id = Some(fixture_id.to_string());
                why_failed.expected_gate = Some(expected_gate.to_string());
            }
            Some(why_failed)
        }
        GateStatus::Pass | GateStatus::Skipped { .. } => None,
    }
}

fn failure_hint(gate_name: &str) -> &'static str {
    match gate_name {
        "fmt" => "Rust formatting drift was detected.",
        "clippy" => "Clippy found warnings or policy violations.",
        "miri" => "Miri found undefined-behavior-sensitive test failure.",
        "test" | "nextest" => {
            "A Rust test failed; inspect the captured log for the first failing case."
        }
        "supply-chain" => "Supply-chain policy gate failed; inspect dependency policy output.",
        _ => "Gate failed; inspect the captured log and rerun the named gate locally.",
    }
}

fn failure_repair_command(gate_name: &str) -> &'static str {
    match gate_name {
        "fmt" => "cargo +nightly fmt --all",
        "clippy" => "cargo +nightly clippy --workspace --all-targets --all-features",
        "miri" => "moon run velvet-ballastics:miri",
        "test" | "nextest" => "moon run velvet-ballastics:test",
        "supply-chain" => "moon run velvet-ballastics:supply-chain",
        _ => "moon ci --base HEAD --head HEAD",
    }
}

/// Validates that all required evidence files exist in a directory.
///
/// Implements fail-closed behavior: missing evidence is treated as failure.
///
/// # Arguments
/// * `dir` - Directory to check for evidence files
/// * `required_gates` - List of gate names that must have evidence
///
/// # Errors
/// Returns `Error::MissingEvidence` for each missing evidence file.
/// Returns `Error::BeadDirectoryCreationFailed` if directory cannot be accessed.
pub fn validate_evidence_dir(dir: &Path, required_gates: &[&str]) -> Result<Vec<Error>> {
    let errors = required_gates
        .iter()
        .filter_map(|gate| {
            let path = dir.join(format!("{gate}.yaml"));
            (!path.exists()).then(|| Error::MissingEvidence {
                gate: (*gate).to_string(),
                path,
            })
        })
        .collect();
    Ok(errors)
}

/// Constructs the evidence file path for a given bead and gate.
///
/// Path is always scoped to `.evidence/<bead-id>/<gate-name>.yaml`
///
/// # Arguments
/// * `bead_id` - The bead identifier
/// * `gate_name` - The gate name
///
/// # Returns
/// PathBuf within `.evidence/<bead_id>/` directory.
pub fn evidence_path(bead_id: &str, gate_name: &str) -> PathBuf {
    // RED_PHASE: Simple implementation that follows the contract
    PathBuf::from(".evidence")
        .join(bead_id)
        .join(format!("{}.yaml", gate_name))
}

/// Writes evidence to a YAML file.
///
/// # Arguments
/// * `evidence` - The evidence to serialize and write
/// * `path` - Target file path
///
/// # Errors
/// Returns `Error::YamlSerializationFailed` if serialization fails.
/// Returns `Error::EvidenceWriteFailed` if file write fails.
pub fn write_evidence(evidence: &GateEvidence, path: &Path) -> Result<()> {
    let yaml =
        serde_saphyr::to_string(evidence).map_err(|error| Error::YamlSerializationFailed {
            gate: evidence.gate_name.clone(),
            cause: error.to_string(),
        })?;
    write_text_file(path, &yaml)
}
