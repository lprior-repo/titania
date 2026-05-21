
impl Error {
    /// Extract false-pass diagnostic variant and fields if this is a false-pass error.
    ///
    /// Returns `Some((variant, fixture_id, expected_gate))` when `gate` is
    /// `"FalsePassFixtureViolation"`, otherwise `None`.
    pub fn false_pass_diagnostic(&self) -> Option<(FalsePassDiagnosticVariant, &str, &str)> {
        let Error::GateFailed { gate, log, .. } = self else {
            return None;
        };
        if gate != "FalsePassFixtureViolation" {
            return None;
        }
        let path_text = log.display().to_string();
        if path_text.contains("intentional_secret_fixture") {
            Some((
                FalsePassDiagnosticVariant::Secret,
                "intentional_secret_fixture",
                "redaction",
            ))
        } else {
            Some((
                FalsePassDiagnosticVariant::Overlap,
                "intentional_overlap_fixture",
                "layout",
            ))
        }
    }
}

fn write_gate_timeout(f: &mut std::fmt::Formatter<'_>, gate: &str, secs: &u64) -> std::fmt::Result {
    write!(f, "Gate '{}' exceeded timeout of {}s", gate, secs)
}

fn write_gate_failed(
    f: &mut std::fmt::Formatter<'_>,
    gate: &str,
    exit_code: i32,
    log: &Path,
) -> std::fmt::Result {
    if gate == "FalsePassFixtureViolation" {
        return write_false_pass_diagnostic(f, log);
    }
    write!(
        f,
        "Gate '{}' failed with exit code {} (log: {})",
        gate,
        exit_code,
        log.display()
    )
}

fn write_false_pass_diagnostic(f: &mut std::fmt::Formatter<'_>, log: &Path) -> std::fmt::Result {
    let (variant, fixture_id, expected_gate) = false_pass_diagnostic_for_path(log);
    let variant_str = match variant {
        FalsePassDiagnosticVariant::Overlap => "OverlapFalsePass",
        FalsePassDiagnosticVariant::Secret => "SecretFalsePass",
    };
    write!(
        f,
        "UI release gate failed; evidence_path: {}\nxtask_diagnostic:\n  variant: {}\n  error_code: false_pass_fixture_violation\n  fixture_id: {}\n  expected_gate: {}\n  actual_status: passed",
        log.display(),
        variant_str,
        fixture_id,
        expected_gate
    )
}

fn false_pass_diagnostic_for_path(
    log: &Path,
) -> (FalsePassDiagnosticVariant, &'static str, &'static str) {
    let path_text = log.display().to_string();
    if path_text.contains("intentional_secret_fixture") {
        (
            FalsePassDiagnosticVariant::Secret,
            "intentional_secret_fixture",
            "redaction",
        )
    } else {
        (
            FalsePassDiagnosticVariant::Overlap,
            "intentional_overlap_fixture",
            "layout",
        )
    }
}

fn write_missing_evidence(
    f: &mut std::fmt::Formatter<'_>,
    gate: &str,
    path: &Path,
) -> std::fmt::Result {
    write!(
        f,
        "Missing evidence for gate '{}' at {}",
        gate,
        path.display()
    )
}

fn write_evidence_failed(
    f: &mut std::fmt::Formatter<'_>,
    gate: &str,
    path: &Path,
    cause: &str,
) -> std::fmt::Result {
    write!(
        f,
        "Failed to write evidence for '{}' to {}: {}",
        gate,
        path.display(),
        cause
    )
}

fn write_bead_dir_failed(
    f: &mut std::fmt::Formatter<'_>,
    bead: &str,
    cause: &str,
) -> std::fmt::Result {
    write!(
        f,
        "Failed to create evidence directory for bead '{}': {}",
        bead, cause
    )
}

fn write_yaml_failed(f: &mut std::fmt::Formatter<'_>, gate: &str, cause: &str) -> std::fmt::Result {
    write!(f, "YAML serialization failed for '{}': {}", gate, cause)
}

fn write_moon_failed(f: &mut std::fmt::Formatter<'_>, task: &str, cause: &str) -> std::fmt::Result {
    write!(f, "Moon task '{}' failed: {}", task, cause)
}

fn write_just_failed(
    f: &mut std::fmt::Formatter<'_>,
    recipe: &str,
    cause: &str,
) -> std::fmt::Result {
    write!(f, "Just recipe '{}' failed: {}", recipe, cause)
}

impl std::error::Error for Error {}

/// Result type alias for evidence operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Profile of gates to run together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(clippy::enum_variant_names)]
pub enum GateProfile {
    /// Fast gates: fmt, check, clippy, nextest, forbidden-scan, hotpath-scan
    AiFast,
    /// Deep gates: miri, mutants, llvm-cov, fuzz-build
    AiDeep,
    /// Release gates: check, test, supply-chain, miri, fuzz-smoke, coverage,
    /// mutants-smoke, bench-build, feature-powerset, source-length, maxperf
    AiRelease,
}

impl GateProfile {
    /// Returns the list of gates in this profile.
    pub fn gates(self) -> &'static [&'static str] {
        match self {
            GateProfile::AiFast => AI_FAST_GATES,
            GateProfile::AiDeep => &["miri", "mutants", "llvm-cov", "fuzz-build"],
            GateProfile::AiRelease => AI_RELEASE_GATES,
        }
    }

    /// Returns the evidence file name for this profile.
    pub fn evidence_file(self) -> &'static str {
        match self {
            GateProfile::AiFast => "ai-fast.yaml",
            GateProfile::AiDeep => "ai-deep.yaml",
            GateProfile::AiRelease => "ai-release.yaml",
        }
    }
}

/// Aggregated evidence for a full profile run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileEvidence {
    pub profile: String,
    pub gates: Vec<GateEvidence>,
    pub exit_code: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalScreenId(String);

impl CanonicalScreenId {
    fn parse(value: String) -> std::result::Result<Self, String> {
        if CANONICAL_SCREENS
            .iter()
            .any(|screen| screen == &value.as_str())
        {
            Ok(Self(value))
        } else {
            Err(format!("invalid canonical screen id: {value}"))
        }
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiSubgateName(String);

impl UiSubgateName {
    fn parse(value: String) -> std::result::Result<Self, String> {
        if REQUIRED_UI_SUBGATES
            .iter()
            .any(|gate| gate == &value.as_str())
        {
            Ok(Self(value))
        } else {
            Err(format!("invalid UI release subgate: {value}"))
        }
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutCheckName(String);

impl LayoutCheckName {
    fn parse(value: String) -> std::result::Result<Self, String> {
        if REQUIRED_LAYOUT_CHECKS
            .iter()
            .any(|check| check == &value.as_str())
        {
            Ok(Self(value))
        } else {
            Err(format!("invalid layout check: {value}"))
        }
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureBackedState {
    FixtureBacked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreParityClaim {
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedactionClass {
    Sentinel,
    ApiKey,
    Token,
    Password,
    IdempotencyKey,
    TaintedFixtureValue,
}
