
impl RedactionClass {
    fn parse(value: &str) -> std::result::Result<Self, String> {
        match value {
            "sentinel" => Ok(Self::Sentinel),
            "api_key" => Ok(Self::ApiKey),
            "token" => Ok(Self::Token),
            "password" => Ok(Self::Password),
            "idempotency_key" => Ok(Self::IdempotencyKey),
            "tainted_fixture_value" => Ok(Self::TaintedFixtureValue),
            _ => Err(format!("invalid redaction class: {value}")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sentinel => "sentinel",
            Self::ApiKey => "api_key",
            Self::Token => "token",
            Self::Password => "password",
            Self::IdempotencyKey => "idempotency_key",
            Self::TaintedFixtureValue => "tainted_fixture_value",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedSnapshotDocument {
    pub total_screens: usize,
    pub passed_screens: usize,
    pub failed_screens: usize,
    pub screens: Vec<ParsedScreenDocument>,
    pub fixture_backed: FixtureBackedState,
    pub core_runtime_parity_claim: CoreParityClaim,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedScreenDocument {
    pub screen_name: CanonicalScreenId,
    pub checks: Vec<LayoutCheckName>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedAiReleaseDocument {
    pub subgates: Vec<UiSubgateName>,
    pub redaction: Vec<ParsedRedactionScreen>,
    pub fixture_backed: FixtureBackedState,
    pub core_runtime_parity_claim: CoreParityClaim,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedRedactionScreen {
    pub screen_id: CanonicalScreenId,
    pub classes: Vec<RedactionClass>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedNegativeFixtureDocument {
    pub overlap: ParsedOverlapFixtureEvidence,
    pub secret: ParsedSecretFixtureEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedOverlapFixtureEvidence {
    ExpectedFailed(ParsedOverlapExpectedFailure),
    Rejected(ParsedRejectedFixtureEvidence),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedSecretFixtureEvidence {
    ExpectedFailed(ParsedSecretExpectedFailure),
    Rejected(ParsedRejectedFixtureEvidence),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureId(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlId(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureBounds(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureNonce(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactedSample(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NonzeroOverlapArea(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureStatus {
    ExpectedFailed,
    Rejected,
    Passed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCode {
    Layout,
    Redaction,
    FalsePassFixture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureGate {
    LayoutReadability,
    Layout,
    Redaction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedOverlapRejection {
    pub fixture_id: FixtureId,
    pub status: FixtureStatus,
    pub error: DiagnosticCode,
    pub expected_gate: FixtureGate,
    pub actual_status: FixtureStatus,
    pub action: String,
    pub fixture_nonce: Option<FixtureNonce>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedSecretRejection {
    pub fixture_id: FixtureId,
    pub status: FixtureStatus,
    pub error: DiagnosticCode,
    pub expected_gate: FixtureGate,
    pub actual_status: FixtureStatus,
    pub action: String,
    pub fixture_nonce: Option<FixtureNonce>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedOverlapExpectedFailure {
    pub fixture_id: FixtureId,
    pub status: FixtureStatus,
    pub gate: FixtureGate,
    pub diagnostic_code: DiagnosticCode,
    pub screen_id: CanonicalScreenId,
    pub control_id: ControlId,
    pub second_control_id: ControlId,
    pub overlap_area_px: NonzeroOverlapArea,
    pub bounds: FixtureBounds,
    pub predicate: LayoutCheckName,
    pub fixture_nonce: Option<FixtureNonce>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedSecretExpectedFailure {
    pub fixture_id: FixtureId,
    pub status: FixtureStatus,
    pub gate: FixtureGate,
    pub diagnostic_code: DiagnosticCode,
    pub screen_id: CanonicalScreenId,
    pub secret_class: RedactionClass,
    pub redacted_sample: RedactedSample,
    pub fixture_nonce: Option<FixtureNonce>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedRejectedFixtureEvidence {
    Overlap(ParsedOverlapRejection),
    Secret(ParsedSecretRejection),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedNegativeFixtureEntry {
    Overlap(ParsedOverlapFixtureEvidence),
    Secret(ParsedSecretFixtureEvidence),
}

// ============================================================================
// Core orchestration functions
// ============================================================================

pub fn parse_snapshot_document(text: &str) -> std::result::Result<ParsedSnapshotDocument, String> {
    let raw: RawSnapshotDocument = parse_yaml_document(text)?;
    let doc = ParsedSnapshotDocument::try_from(raw)?;
    validate_parsed_snapshot(&doc)?;
    Ok(doc)
}

pub fn parse_ai_release_document(
    text: &str,
) -> std::result::Result<ParsedAiReleaseDocument, String> {
    let raw: RawAiReleaseDocument = parse_yaml_document(text)?;
    let doc = ParsedAiReleaseDocument::try_from(raw)?;
    validate_parsed_ai_release(&doc)?;
    Ok(doc)
}

pub fn parse_negative_fixture_document(
    text: &str,
) -> std::result::Result<ParsedNegativeFixtureDocument, String> {
    let raw: RawNegativeFixtureDocument = parse_yaml_document(text)?;
    let doc = ParsedNegativeFixtureDocument::try_from(raw)?;
    validate_parsed_negative(&doc)?;
    Ok(doc)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSnapshotDocument {
    status: String,
    total_screens: usize,
    passed_screens: usize,
    failed_screens: usize,
    fixture_backed: bool,
    core_runtime_parity_claim: String,
    screens: Vec<RawScreenDocument>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawScreenDocument {
    screen_name: String,
    fixture_id: String,
    artifact_path: String,
    digest: String,
    passed: bool,
    diagnostics: Vec<String>,
    execution_marker: String,
    checks: Vec<RawScreenCheck>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawScreenCheck {
    kind: String,
    passed: bool,
    diagnostics: Vec<String>,
    execution_marker: String,
    origin: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAiReleaseDocument {
    profile: String,
    bead_id: String,
    status: String,
    fixture_backed: bool,
    core_runtime_parity_claim: String,
    command: String,
    subgates: Vec<RawSubgateDocument>,
    redaction: RawRedactionDocument,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSubgateDocument {
    name: String,
    status: String,
    command: String,
    origin: String,
    diagnostics: Vec<String>,
    execution_marker: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRedactionDocument {
    status: String,
    checked_artifacts: Vec<String>,
    screens: Vec<RawRedactionScreen>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRedactionScreen {
    screen_id: String,
    status: String,
    diagnostics: Vec<String>,
    execution_marker: String,
    class_coverage: RawClassCoverage,
}
