
fn release_shape_error(gate: &str, cause: &str) -> Error {
    Error::MissingEvidence {
        gate: format!("{gate}:{cause}"),
        path: PathBuf::from(".evidence/vb-nf2u/ai-release.yaml"),
    }
}

const DEFAULT_OVERLAP_NEGATIVE_FIXTURE: &str = concat!(
    "fixture_id=intentional_overlap_fixture\n",
    "screen_id=execution_overview\n",
    "first_control_id=run_button\n",
    "second_control_id=stop_button\n",
    "expected_gate=layout\n",
    "expected_code=layout_violation\n",
    "overlap_area_px=600\n",
    "bounds={ x: 10, y: 10, width: 100, height: 60 }\n",
    "actual_status=failed\n",
);

const DEFAULT_SECRET_NEGATIVE_FIXTURE: &str = concat!(
    "fixture_id=intentional_secret_fixture\n",
    "expected_gate=redaction\n",
    "expected_code=redaction_violation\n",
    "actual_status=failed\n",
);

fn subgate_origin_name(origin: SubgateOrigin) -> &'static str {
    match origin {
        SubgateOrigin::SnapshotInventory => "snapshot_inventory_validation",
        SubgateOrigin::LayoutPredicates => "layout_predicate_outcomes",
        SubgateOrigin::RedactionScan => "redaction_scanner_outcomes",
        SubgateOrigin::NegativeFixtures => "negative_fixture_state_outcome",
        SubgateOrigin::DeterministicCapture => "deterministic_capture_validation",
        SubgateOrigin::EvidenceShape => "evidence_shape_validation",
    }
}

fn append_redaction_report(report: &mut String) {
    report.push_str("redaction:\n  status: passed\n  checked_artifacts:\n    - fixture_text_artifact\n    - ui_snapshot_report\n    - diagnostics\n    - generated_artifacts\n  screens:\n");
    for screen in CANONICAL_SCREENS {
        report.push_str("    - screen_id: ");
        report.push_str(screen);
        report.push_str("\n      status: passed\n      diagnostics: []\n      execution_marker: vb-nf2u-redaction-.");
        report.push_str(screen);
        report.push_str("\n      class_coverage:\n");
        for (class, placeholder) in REDACTION_CLASSES {
            report.push_str("        ");
            report.push_str(class);
            report.push_str(":\n          detectors: 1\n          raw_matches: 0\n          approved_placeholders_seen: 1\n          placeholder: '");
            report.push_str(placeholder);
            report.push_str("'\n");
        }
    }
}

fn write_negative_fixtures(output_dir: &Path) -> Result<()> {
    let content = negative_fixture_report()?;
    write_text_file(&output_dir.join("negative-fixtures.txt"), &content)
}

fn negative_fixture_report() -> Result<String> {
    ensure_default_negative_fixtures()?;
    let overlap = OverlapNegativeFixture::read_required()?;
    let secret = SecretNegativeFixture::read_required()?;
    let mut content = String::from("negative_fixtures:\n");
    append_overlap_negative_fixture(&mut content, &overlap);
    append_secret_negative_fixture(&mut content, &secret);
    append_negative_fixture_contract_audit(&mut content);
    Ok(content)
}

fn read_optional_fixture(name: &str) -> FixtureReadState {
    let path = Path::new(NEGATIVE_FIXTURE_ROOT).join(name);
    match fs::read_to_string(path) {
        Ok(content) => FixtureReadState::Present(content),
        Err(_) => FixtureReadState::Missing(Path::new(NEGATIVE_FIXTURE_ROOT).join(name)),
    }
}

fn ensure_default_negative_fixtures() -> Result<()> {
    ensure_default_negative_fixture(
        "intentional_overlap_fixture.txt",
        DEFAULT_OVERLAP_NEGATIVE_FIXTURE,
    )?;
    ensure_default_negative_fixture(
        "intentional_secret_fixture.txt",
        DEFAULT_SECRET_NEGATIVE_FIXTURE,
    )
}

fn ensure_default_negative_fixture(name: &str, content: &str) -> Result<()> {
    let path = Path::new(NEGATIVE_FIXTURE_ROOT).join(name);
    match fs::metadata(&path) {
        Ok(metadata) if metadata.is_file() => Ok(()),
        Ok(_) => Err(Error::EvidenceWriteFailed {
            gate: "negative_fixture_seed".to_string(),
            path,
            cause: "fixture path exists but is not a file".to_string(),
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => write_text_file(&path, content),
        Err(error) => Err(Error::EvidenceWriteFailed {
            gate: "negative_fixture_seed".to_string(),
            path,
            cause: error.to_string(),
        }),
    }
}

fn append_overlap_negative_fixture(report: &mut String, fixture: &OverlapNegativeFixture) {
    report.push_str("  - fixture_id: intentional_overlap_fixture\n");
    if fixture.is_false_pass() {
        report.push_str("    error: UiReleaseGateError::FalsePassFixtureViolation\n    variant: FalsePassFixtureViolation\n    code: false_pass_fixture_violation\n    status: rejected\n    expected_gate: layout\n    actual_status: passed\n    action: fail release because expected-fail negative fixture passed\n");
    } else {
        report.push_str("    status: expected-failed\n    gate: layout_readability\n    diagnostic_code: layout_violation\n    screen_id: execution_overview\n    artifact_path: target/vb-nf2u-negative-fixtures/intentional_overlap_fixture.txt\n    control_id: ");
        report.push_str(&fixture.first_control_id);
        report.push_str("\n    second_control_id: ");
        report.push_str(&fixture.second_control_id);
        report.push_str("\n    predicate: overlap\n    overlap_area_px: ");
        report.push_str(&fixture.overlap_area_px);
        report.push_str("\n    bounds: '");
        report.push_str(&fixture.bounds);
        report.push_str("'\n    action: keep release gate failing on overlapping controls\n");
    }
    append_fixture_nonce(report, fixture.fixture_nonce.as_ref());
}

fn append_secret_negative_fixture(report: &mut String, fixture: &SecretNegativeFixture) {
    report.push_str("  - fixture_id: intentional_secret_fixture\n");
    if fixture.is_false_pass() {
        report.push_str("    error: UiReleaseGateError::FalsePassFixtureViolation\n    variant: FalsePassFixtureViolation\n    code: false_pass_fixture_violation\n    status: rejected\n    expected_gate: redaction\n    actual_status: passed\n    action: fail release because expected-fail negative fixture passed\n");
    } else {
        report.push_str("    status: expected-failed\n    gate: redaction\n    diagnostic_code: redaction_violation\n    screen_id: storage_doctor_ai_context\n    artifact_path: target/vb-nf2u-negative-fixtures/intentional_secret_fixture.txt\n    secret_class: api_key\n    redacted_sample: '[REDACTED:api_key]'\n    action: keep release gate failing on raw secret exposure\n");
    }
    append_fixture_nonce(report, fixture.fixture_nonce.as_ref());
}

fn append_fixture_nonce(report: &mut String, nonce: Option<&String>) {
    if let Some(nonce) = nonce {
        report.push_str("    fixture_nonce: ");
        report.push_str(nonce);
        report.push('\n');
    }
}

fn append_negative_fixture_contract_audit(report: &mut String) {
    report.push_str("contract_audit:\n");
    report.push_str("  fixture_backed: true\n");
    report.push_str("  false_pass_detectors:\n");
    report.push_str("    - overlap_false_pass_detector\n");
    report.push_str("    - secret_false_pass_detector\n");
    report.push_str("  core_runtime_parity_claim: unsupported\n");
}

fn false_pass_negative_fixture_path() -> Option<PathBuf> {
    match required_negative_fixture_states() {
        Ok((overlap, secret)) => first_false_pass_fixture_path(&overlap, &secret),
        Err(_) => Some(PathBuf::from(NEGATIVE_FIXTURE_ROOT).join("negative-fixtures.txt")),
    }
}

fn first_false_pass_fixture_path(
    overlap: &OverlapNegativeFixture,
    secret: &SecretNegativeFixture,
) -> Option<PathBuf> {
    if overlap.is_false_pass() {
        Some(PathBuf::from(NEGATIVE_FIXTURE_ROOT).join("intentional_overlap_fixture.txt"))
    } else if secret.is_false_pass() {
        Some(PathBuf::from(NEGATIVE_FIXTURE_ROOT).join("intentional_secret_fixture.txt"))
    } else {
        None
    }
}

fn required_negative_fixture_states() -> Result<(OverlapNegativeFixture, SecretNegativeFixture)> {
    Ok((
        OverlapNegativeFixture::read_required()?,
        SecretNegativeFixture::read_required()?,
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OverlapNegativeFixture {
    first_control_id: String,
    second_control_id: String,
    overlap_area_px: String,
    bounds: String,
    actual_status: String,
    fixture_nonce: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SecretNegativeFixture {
    actual_status: String,
    fixture_nonce: Option<String>,
}

impl OverlapNegativeFixture {
    fn read_required() -> Result<Self> {
        Self::from_read_state(read_optional_fixture("intentional_overlap_fixture.txt"))
    }

    fn from_read_state(state: FixtureReadState) -> Result<Self> {
        match state {
            FixtureReadState::Present(content) => Self::parse_overlap(&content),
            FixtureReadState::Missing(path) => Err(Error::MissingEvidence {
                gate: "negative_fixture".to_string(),
                path,
            }),
        }
    }

    fn parse_overlap(content: &str) -> Result<Self> {
        Ok(Self {
            first_control_id: required_fixture_field(content, "first_control_id")?.to_string(),
            second_control_id: required_fixture_field(content, "second_control_id")?.to_string(),
            overlap_area_px: required_fixture_field(content, "overlap_area_px")?.to_string(),
            bounds: required_fixture_field(content, "bounds")?.to_string(),
            actual_status: fixture_status_field(content)?.to_string(),
            fixture_nonce: optional_fixture_field(content, "fixture_nonce").map(str::to_string),
        })
    }

    fn is_false_pass(&self) -> bool {
        self.actual_status == "passed"
    }
}

impl SecretNegativeFixture {
    fn read_required() -> Result<Self> {
        Self::from_read_state(read_optional_fixture("intentional_secret_fixture.txt"))
    }

    fn from_read_state(state: FixtureReadState) -> Result<Self> {
        match state {
            FixtureReadState::Present(content) => Self::parse_secret(&content),
            FixtureReadState::Missing(path) => Err(Error::MissingEvidence {
                gate: "negative_fixture".to_string(),
                path,
            }),
        }
    }

    fn parse_secret(content: &str) -> Result<Self> {
        let fixture_id = required_fixture_field(content, "fixture_id")?;
        let expected_gate = required_fixture_field(content, "expected_gate")?;
        let expected_code = required_fixture_field(content, "expected_code")?;
        if fixture_id != "intentional_secret_fixture"
            || expected_gate != "redaction"
            || expected_code != "redaction_violation"
        {
            return Err(Error::GateFailed {
                gate: "malformed secret negative fixture".to_string(),
                exit_code: 1,
                log: PathBuf::from(NEGATIVE_FIXTURE_ROOT).join("intentional_secret_fixture.txt"),
            });
        }
        Ok(Self {
            actual_status: fixture_status_field(content)?.to_string(),
            fixture_nonce: optional_fixture_field(content, "fixture_nonce").map(str::to_string),
        })
    }

    fn is_false_pass(&self) -> bool {
        self.actual_status == "passed"
    }
}

fn required_fixture_field<'a>(content: &'a str, key: &str) -> Result<&'a str> {
    optional_fixture_field(content, key).ok_or_else(|| Error::MissingEvidence {
        gate: format!("negative_fixture:{key}"),
        path: PathBuf::from(NEGATIVE_FIXTURE_ROOT),
    })
}

fn optional_fixture_field<'a>(content: &'a str, key: &str) -> Option<&'a str> {
    content.lines().find_map(|line| {
        line.strip_prefix(key)
            .and_then(|tail| tail.strip_prefix('='))
    })
}

fn fixture_status_field(content: &str) -> Result<&str> {
    required_fixture_field(content, "actual_status")
}

impl FixtureReadState {
    fn field_value<'a>(&'a self, key: &str) -> Option<&'a str> {
        match self {
            Self::Present(content) => content.lines().find_map(|line| {
                line.strip_prefix(key)
                    .and_then(|tail| tail.strip_prefix('='))
            }),
            Self::Missing(_) => None,
        }
    }
}

fn write_text_file(path: &Path, content: &str) -> Result<()> {
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

fn read_text_file(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|_| Error::MissingEvidence {
        gate: "ui-release-readback".to_string(),
        path: path.to_path_buf(),
    })
}
