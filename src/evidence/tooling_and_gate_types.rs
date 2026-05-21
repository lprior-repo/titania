
pub fn check_redaction_artifacts(
    evidence: &UiReleaseEvidence,
    _denylist: &SecretDenylist,
) -> std::result::Result<RedactionEvidence, UiReleaseGateError> {
    if let ReleaseArtifactWorkflow::Text { path, text } = evidence.artifact {
        for (secret_class, raw_secret, redacted_sample) in raw_secret_patterns() {
            if text.contains(raw_secret) {
                return Err(UiReleaseGateError::RedactionViolation {
                    code: "redaction_violation",
                    screen_id: "execution_overview",
                    artifact_path: path,
                    secret_class,
                    redacted_sample,
                    action: "redact raw secret before emitting UI evidence",
                });
            }
        }
    }
    Ok(RedactionEvidence { status: "passed" })
}

fn raw_secret_patterns() -> [(&'static str, &'static str, &'static str); 6] {
    [
        ("sentinel", "vb_nf2u_secret_sentinel", "[REDACTED:sentinel]"),
        (
            "api_key",
            "sk_test_vb_nf2u_raw_secret",
            "[REDACTED:api_key]",
        ),
        ("token", "Bearer vb_nf2u_token", "[REDACTED:token]"),
        ("password", "password=hunter2", "[REDACTED:password]"),
        (
            "idempotency_key",
            "Idempotency-Key: idem_vb_nf2u_secret",
            "[REDACTED:idempotency_key]",
        ),
        (
            "tainted_fixture_value",
            "tainted_fixture_value_vb_nf2u",
            "[REDACTED:tainted_fixture_value]",
        ),
    ]
}

pub fn include_ui_gates_in_ai_release(
    bead_id: &'static str,
) -> std::result::Result<ReleaseProfileEvidence, UiReleaseGateError> {
    let release_bead = ReleaseBeadId::parse(bead_id)?;
    Ok(ReleaseProfileEvidence {
        bead_id: release_bead,
        subgates: REQUIRED_UI_SUBGATES.to_vec(),
        parity_claim: ReleaseParityClaim::FixtureBacked,
    })
}
const UI_RELEASE_TOOLING_LANES: [UiReleaseToolingLane; 7] = [
    UiReleaseToolingLane {
        name: "kani-inventory",
        command: "cargo kani -p vb_ui_snapshot --harness inventory",
        kind: UiReleaseToolingLaneKind::ExternalMachineGate,
        blocker: Some("requires Kani runner outside this bead-scoped nextest suite"),
    },
    UiReleaseToolingLane {
        name: "kani-layout-predicates",
        command: "cargo kani -p vb_ui_snapshot --harness layout_",
        kind: UiReleaseToolingLaneKind::ExternalMachineGate,
        blocker: Some("requires Kani runner outside this bead-scoped nextest suite"),
    },
    UiReleaseToolingLane {
        name: "redaction-fuzz",
        command: "cargo fuzz run ui_redaction_artifact",
        kind: UiReleaseToolingLaneKind::ExternalMachineGate,
        blocker: Some("cargo-fuzz sanitizer target is an external machine gate"),
    },
    UiReleaseToolingLane {
        name: "miri",
        command: "cargo +nightly miri test -p vb_ui_snapshot",
        kind: UiReleaseToolingLaneKind::ExecutableGate,
        blocker: None,
    },
    UiReleaseToolingLane {
        name: "mutants",
        command: "cargo mutants -p vb_ui_snapshot",
        kind: UiReleaseToolingLaneKind::ExecutableGate,
        blocker: None,
    },
    UiReleaseToolingLane {
        name: "coverage",
        command: "cargo llvm-cov nextest",
        kind: UiReleaseToolingLaneKind::ExecutableGate,
        blocker: None,
    },
    UiReleaseToolingLane {
        name: "moon-ci",
        command: "moon ci",
        kind: UiReleaseToolingLaneKind::ExternalMachineGate,
        blocker: Some("repository-level machine gate, not a bead-local unit behavior"),
    },
];

pub fn ui_release_tooling_lanes() -> &'static [UiReleaseToolingLane] {
    &UI_RELEASE_TOOLING_LANES
}

/// Evidence bundle for a single gate execution.
///
/// Contains all fields required by POST-004:
/// - `kind`: Category of the gate (e.g., "fmt", "clippy", "ai-fast")
/// - `gate_name`: Specific gate name within the category
/// - `command`: Full command string that was executed
/// - `exit_code`: Numeric exit code from the command
/// - `log`: Path to the log file with raw tool output
/// - `status`: Pass/Fail/Skipped status
/// - `why_failed`: Optional failure diagnostic with hint and repair command
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateEvidence {
    pub kind: String,
    pub gate_name: String,
    pub command: String,
    pub exit_code: i32,
    pub log: PathBuf,
    pub status: GateStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub why_failed: Option<WhyFailed>,
}

/// Variant tag for structured false-pass diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FalsePassDiagnosticVariant {
    Overlap,
    Secret,
}

/// Failure diagnostic with hint and repair command.
///
/// Populated when a gate fails, providing actionable remediation steps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WhyFailed {
    pub gate_name: String,
    pub hint: String,
    pub repair_command: String,
    /// Variant tag for false-pass diagnostics. Present when gate is
    /// `FalsePassFixtureViolation` to disambiguate overlap vs secret false-pass.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<FalsePassDiagnosticVariant>,
    /// Fixture ID from a false-pass diagnostic. Present when variant is Some.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixture_id: Option<String>,
    /// Expected gate from a false-pass diagnostic. Present when variant is Some.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_gate: Option<String>,
}

/// Status of a gate execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", content = "reason")]
pub enum GateStatus {
    Pass,
    Fail,
    Skipped { reason: String },
}

/// All error variants for xtask gate operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Gate exceeded its configured timeout duration.
    GateTimeout { gate: String, duration_secs: u64 },
    /// Underlying command returned non-zero exit code.
    GateFailed {
        gate: String,
        exit_code: i32,
        log: PathBuf,
    },
    /// Evidence file for a required gate does not exist (fail-closed).
    MissingEvidence { gate: String, path: PathBuf },
    /// YAML serialization or file write failed.
    EvidenceWriteFailed {
        gate: String,
        path: PathBuf,
        cause: String,
    },
    /// Requested xtask subcommand does not exist.
    SubcommandNotFound { name: String },
    /// Could not create `.evidence/<bead>/` directory.
    BeadDirectoryCreationFailed { bead: String, cause: String },
    /// saphyr error during evidence serialization.
    YamlSerializationFailed { gate: String, cause: String },
    /// moon run task returned non-zero.
    UpstreamMoonFailed { task: String, cause: String },
    /// just recipe returned non-zero.
    UpstreamJustFailed { recipe: String, cause: String },

    /// Schema version string could not be parsed as major.minor.
    SchemaVersionParseFailed { version: String },

    /// A required bundle field was missing on deserialisation.
    MissingRequiredField { field: String },

    /// Bundle-level serialisation failed for the chosen format.
    BundleSerializationFailed { format: String, cause: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XtaskCommandDiagnostic {
    pub error_code: DiagnosticCode,
    pub fixture_id: FixtureId,
    pub expected_gate: FixtureGate,
    pub actual_status: FixtureStatus,
    pub variant: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCommandDiagnosticEnvelope {
    xtask_diagnostic: RawCommandDiagnostic,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCommandDiagnostic {
    error_code: String,
    fixture_id: String,
    expected_gate: String,
    actual_status: String,
    #[serde(default)]
    variant: Option<String>,
}

impl XtaskCommandDiagnostic {
    pub fn parse_output(text: &str) -> std::result::Result<Self, String> {
        let yaml = diagnostic_yaml_slice(text)?;
        let raw: RawCommandDiagnosticEnvelope = parse_yaml_document(&yaml)?;
        Self::try_from(raw.xtask_diagnostic)
    }
}

impl TryFrom<RawCommandDiagnostic> for XtaskCommandDiagnostic {
    type Error = String;

    fn try_from(raw: RawCommandDiagnostic) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            error_code: parse_diagnostic_code_value(raw.error_code)?,
            fixture_id: FixtureId::parse(raw.fixture_id, "diagnostic fixture_id")?,
            expected_gate: parse_gate_value(raw.expected_gate)?,
            actual_status: parse_status_value(raw.actual_status)?,
            variant: raw.variant,
        })
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::GateTimeout {
                gate,
                duration_secs,
            } => write_gate_timeout(f, gate, duration_secs),
            Error::GateFailed {
                gate,
                exit_code,
                log,
            } => write_gate_failed(f, gate, *exit_code, log),
            Error::MissingEvidence { gate, path } => write_missing_evidence(f, gate, path),
            Error::EvidenceWriteFailed { gate, path, cause } => {
                write_evidence_failed(f, gate, path, cause)
            }
            Error::SubcommandNotFound { name } => write!(f, "Subcommand not found: '{}'", name),
            Error::BeadDirectoryCreationFailed { bead, cause } => {
                write_bead_dir_failed(f, bead, cause)
            }
            Error::YamlSerializationFailed { gate, cause } => write_yaml_failed(f, gate, cause),
            Error::UpstreamMoonFailed { task, cause } => write_moon_failed(f, task, cause),
            Error::UpstreamJustFailed { recipe, cause } => write_just_failed(f, recipe, cause),
            Error::SchemaVersionParseFailed { version } => {
                write!(f, "Schema version parse failed: '{version}'")
            }
            Error::MissingRequiredField { field } => {
                write!(f, "Missing required field: '{field}'")
            }
            Error::BundleSerializationFailed { format, cause } => {
                write!(f, "Bundle serialization ({format}) failed: {cause}")
            }
        }
    }
}
