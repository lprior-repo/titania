
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawClassCoverage {
    sentinel: RawClassEvidence,
    api_key: RawClassEvidence,
    token: RawClassEvidence,
    password: RawClassEvidence,
    idempotency_key: RawClassEvidence,
    tainted_fixture_value: RawClassEvidence,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawClassEvidence {
    detectors: usize,
    raw_matches: usize,
    approved_placeholders_seen: usize,
    placeholder: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawNegativeFixtureDocument {
    negative_fixtures: Vec<RawNegativeFixtureEntry>,
    contract_audit: RawContractAudit,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawContractAudit {
    fixture_backed: bool,
    false_pass_detectors: Vec<String>,
    core_runtime_parity_claim: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawNegativeFixtureEntry {
    fixture_id: String,
    status: String,
    gate: Option<String>,
    diagnostic_code: Option<String>,
    screen_id: Option<String>,
    artifact_path: Option<String>,
    control_id: Option<String>,
    second_control_id: Option<String>,
    overlap_area_px: Option<String>,
    bounds: Option<String>,
    predicate: Option<String>,
    fixture_nonce: Option<String>,
    secret_class: Option<String>,
    redacted_sample: Option<String>,
    variant: Option<String>,
    expected_gate: Option<String>,
    actual_status: Option<String>,
    error: Option<String>,
    code: Option<String>,
    action: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDeterminismDocument {
    deterministic_capture: String,
    snapshot_timestamp: String,
    hidden_animation_state: String,
    clock_source: String,
    execution_marker: String,
    fixture_backed: bool,
    core_runtime_parity_claim: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAnimationFreezeDocument {
    hidden_animation_state: String,
    visible_animation_time_source: String,
    execution_marker: String,
}

fn parse_yaml_document<T>(text: &str) -> std::result::Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    serde_saphyr::from_str::<T>(text).map_err(|error| error.to_string())
}

fn parse_determinism_document(text: &str) -> std::result::Result<(), String> {
    let raw: RawDeterminismDocument = parse_yaml_document(text)?;
    if raw.deterministic_capture != "passed"
        || raw.snapshot_timestamp != "2026-05-09T00:00:00Z"
        || raw.hidden_animation_state != "Paused"
        || raw.clock_source != "FixedFixtureTime"
        || raw.execution_marker != "vb-nf2u-deterministic-capture"
    {
        return Err("invalid deterministic capture document".to_string());
    }
    require_fixture_backed(raw.fixture_backed)?;
    require_unsupported_parity(raw.core_runtime_parity_claim)?;
    Ok(())
}

fn parse_animation_freeze_document(text: &str) -> std::result::Result<(), String> {
    let raw: RawAnimationFreezeDocument = parse_yaml_document(text)?;
    if raw.hidden_animation_state == "Paused"
        && raw.visible_animation_time_source == "FixedFixtureTime"
        && raw.execution_marker == "vb-nf2u-animation-freeze"
    {
        Ok(())
    } else {
        Err("invalid animation freeze document".to_string())
    }
}

fn require_fixture_backed(value: bool) -> std::result::Result<FixtureBackedState, String> {
    if value {
        Ok(FixtureBackedState::FixtureBacked)
    } else {
        Err("fixture_backed must be true".to_string())
    }
}

fn require_unsupported_parity(value: String) -> std::result::Result<CoreParityClaim, String> {
    if value == "unsupported" {
        Ok(CoreParityClaim::Unsupported)
    } else {
        Err(format!("invalid core parity claim: {value}"))
    }
}

impl FixtureId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl ControlId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FixtureBounds {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FixtureNonce {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl RedactedSample {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl NonzeroOverlapArea {
    pub fn as_u32(self) -> u32 {
        self.0
    }
}

impl FixtureStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExpectedFailed => "expected-failed",
            Self::Rejected => "rejected",
            Self::Passed => "passed",
        }
    }
}

impl DiagnosticCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Layout => "layout_violation",
            Self::Redaction => "redaction_violation",
            Self::FalsePassFixture => "false_pass_fixture_violation",
        }
    }
}

impl FixtureGate {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LayoutReadability => "layout_readability",
            Self::Layout => "layout",
            Self::Redaction => "redaction",
        }
    }
}

impl TryFrom<RawSnapshotDocument> for ParsedSnapshotDocument {
    type Error = String;

    fn try_from(raw: RawSnapshotDocument) -> std::result::Result<Self, Self::Error> {
        if raw.status != "pass" {
            return Err("snapshot report status must be pass".to_string());
        }
        Ok(Self {
            total_screens: raw.total_screens,
            passed_screens: raw.passed_screens,
            failed_screens: raw.failed_screens,
            fixture_backed: require_fixture_backed(raw.fixture_backed)?,
            core_runtime_parity_claim: require_unsupported_parity(raw.core_runtime_parity_claim)?,
            screens: raw
                .screens
                .into_iter()
                .map(ParsedScreenDocument::try_from)
                .collect::<std::result::Result<Vec<_>, _>>()?,
        })
    }
}

impl TryFrom<RawScreenDocument> for ParsedScreenDocument {
    type Error = String;

    fn try_from(raw: RawScreenDocument) -> std::result::Result<Self, Self::Error> {
        if raw.fixture_id != raw.screen_name || !raw.passed || !raw.diagnostics.is_empty() {
            return Err("invalid snapshot screen row".to_string());
        }
        if !raw.artifact_path.ends_with(".fixture.txt") || !raw.digest.starts_with("blake3:") {
            return Err("invalid snapshot artifact provenance".to_string());
        }
        let screen_name = CanonicalScreenId::parse(raw.screen_name)?;
        let marker = format!("vb-nf2u-{}", screen_name.as_str());
        if raw.execution_marker != marker {
            return Err("invalid snapshot execution marker".to_string());
        }
        Ok(Self {
            screen_name,
            checks: raw
                .checks
                .into_iter()
                .map(|check| check.into_check())
                .collect::<std::result::Result<Vec<_>, _>>()?,
        })
    }
}

impl RawScreenCheck {
    fn into_check(self) -> std::result::Result<LayoutCheckName, String> {
        if !self.passed || !self.diagnostics.is_empty() || self.origin.is_empty() {
            return Err("invalid snapshot check row".to_string());
        }
        LayoutCheckName::parse(self.kind)
    }
}

impl TryFrom<RawAiReleaseDocument> for ParsedAiReleaseDocument {
    type Error = String;

    fn try_from(raw: RawAiReleaseDocument) -> std::result::Result<Self, Self::Error> {
        if raw.profile != "ai-release" || raw.bead_id != VB_NF2U || raw.status != "passed" {
            return Err("invalid ai-release document header".to_string());
        }
        if raw.command != "cargo xtask ai-release --bead vb-nf2u" {
            return Err("invalid ai-release command".to_string());
        }
        Ok(Self {
            fixture_backed: require_fixture_backed(raw.fixture_backed)?,
            core_runtime_parity_claim: require_unsupported_parity(raw.core_runtime_parity_claim)?,
            subgates: raw
                .subgates
                .into_iter()
                .map(RawSubgateDocument::into_subgate)
                .collect::<std::result::Result<Vec<_>, _>>()?,
            redaction: raw.redaction.into_screens()?,
        })
    }
}
