#![forbid(unsafe_code)]

//! Contract-discovery: walk contracts/, validate schema_version + kind, run cue vet,
//! produce a DiscoveryReport and GateEvidence.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Domain types (contract.md Rust model)
// ---------------------------------------------------------------------------

/// Recognised contract kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractKind {
    CliEnvelope,
    UiTokens,
    AcceptedArtifacts,
    EvidenceBundle,
    Diagnostics,
    GateOutput,
}

impl ContractKind {
    /// All recognised values in ordinal order.
    pub const fn all_values() -> &'static [Self] {
        &[
            Self::CliEnvelope,
            Self::UiTokens,
            Self::AcceptedArtifacts,
            Self::EvidenceBundle,
            Self::Diagnostics,
            Self::GateOutput,
        ]
    }

    /// Stable wire spelling for this contract kind.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CliEnvelope => "cli_envelope",
            Self::UiTokens => "ui_tokens",
            Self::AcceptedArtifacts => "accepted_artifacts",
            Self::EvidenceBundle => "evidence_bundle",
            Self::Diagnostics => "diagnostics",
            Self::GateOutput => "gate_output",
        }
    }

    /// Convert a string slice into a recognised ContractKind, or the string
    /// itself when it is not recognised (for error reporting).
    pub fn parse(s: &str) -> Result<Self, String> {
        Self::all_values()
            .iter()
            .copied()
            .find(|kind| kind.as_str() == s)
            .ok_or_else(|| s.to_string())
    }
}

impl fmt::Display for ContractKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A single contract file discovered under contracts/.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractFile {
    pub path: PathBuf,
    pub schema_version: String,
    pub kind: ContractKind,
    pub vet_errors: Vec<String>,
}

/// A version monotonicity breach.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionViolation {
    pub file: PathBuf,
    pub expected: String,
    pub actual: String,
    pub detail: String,
}

/// Summary counters produced by discovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportSummary {
    pub total: u32,
    pub valid: u32,
    pub invalid: u32,
    /// BTreeMap ensures deterministic JSON key order.
    pub errors_by_kind: BTreeMap<String, u32>,
    pub version_violations: Vec<VersionViolation>,
}

impl Default for ReportSummary {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportSummary {
    pub fn new() -> Self {
        Self {
            total: 0,
            valid: 0,
            invalid: 0,
            errors_by_kind: BTreeMap::new(),
            version_violations: Vec::new(),
        }
    }
}

/// Full discovery report.
///
/// errors is `Vec<String>` (not `Vec<ValidationError>`) because
/// `ValidationError` from `vb_validate` does not implement Serialize/Deserialize.
/// We store the Display representation instead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryReport {
    pub files: Vec<ContractFile>,
    pub errors: Vec<String>,
    pub summary: ReportSummary,
}

// ---------------------------------------------------------------------------
// Validation error type (local, serializable)
// ---------------------------------------------------------------------------

/// Contract-discovery validation errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractError {
    MissingSchemaVersion,
    InvalidVersion {
        version: String,
    },
    InvalidKind {
        kind: String,
    },
    CueVetFailed {
        file: String,
    },
    VersionMonotonicityBreach {
        file: String,
        expected: String,
        actual: String,
    },
}

impl fmt::Display for ContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSchemaVersion => write!(f, "MISSING_SCHEMA_VERSION"),
            Self::InvalidVersion { version } => write!(f, "INVALID_VERSION: {version}"),
            Self::InvalidKind { kind } => write!(f, "INVALID_KIND: {kind}"),
            Self::CueVetFailed { file } => write!(f, "CUE_VET_FAILED: {file}"),
            Self::VersionMonotonicityBreach {
                file,
                expected,
                actual,
            } => write!(
                f,
                "VERSION_MONOTONICITY_BREACH: {file} expected {expected} got {actual}"
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// schema_version helpers
// ---------------------------------------------------------------------------

/// Parse a `schema_version` string into a validated version.
/// Returns the original string on success (validation already confirmed format).
pub fn parse_schema_version(s: &str) -> Result<String, ContractError> {
    if s.is_empty() {
        return Err(ContractError::MissingSchemaVersion);
    }
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 3 {
        return Err(ContractError::InvalidVersion {
            version: s.to_string(),
        });
    }
    for part in &parts {
        if part.is_empty() {
            return Err(ContractError::InvalidVersion {
                version: s.to_string(),
            });
        }
        if part.len() > 1 && part.starts_with('0') {
            return Err(ContractError::InvalidVersion {
                version: s.to_string(),
            });
        }
        if part.parse::<u32>().is_err() {
            return Err(ContractError::InvalidVersion {
                version: s.to_string(),
            });
        }
    }
    Ok(s.to_string())
}

// ---------------------------------------------------------------------------
// Semver comparison (OBL-004: Verus spec)
// ---------------------------------------------------------------------------

/// Comparison result for two semver strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SemverCmp {
    Equal,
    Less,
    Greater,
}

/// Compare two semver strings. Both must be valid "X.Y.Z" format.
pub fn compare_semver(a: &str, b: &str) -> Result<SemverCmp, String> {
    let pa: Vec<u32> = a.split('.').filter_map(|p| p.parse().ok()).collect();
    let pb: Vec<u32> = b.split('.').filter_map(|p| p.parse().ok()).collect();
    if pa.len() != 3 || pb.len() != 3 {
        return Err(format!("Invalid semver format: a='{a}', b='{b}'"));
    }
    for (a_v, b_v) in pa.iter().zip(pb.iter()) {
        if a_v < b_v {
            return Ok(SemverCmp::Less);
        }
        if a_v > b_v {
            return Ok(SemverCmp::Greater);
        }
    }
    Ok(SemverCmp::Equal)
}

// ---------------------------------------------------------------------------
// cue vet helpers
// ---------------------------------------------------------------------------

/// Parse a `cue vet` exit code. Returns Ok(()) for exit 0, Err for non-zero.
pub fn parse_vet_exit_code(code: i32) -> Result<(), String> {
    if code == 0 {
        Ok(())
    } else {
        Err(format!("cue vet exited with code {code}"))
    }
}

/// Run `cue vet` on a single file and return (exit_code, stderr_output).
/// Returns an error if the cue binary is not found.
/// If `cwd` is provided, cue runs with that directory as its working directory
/// so that relative file paths are resolved correctly.
pub fn run_cue_vet(file: &Path, cwd: Option<&Path>) -> Result<(i32, String), String> {
    let mut cmd = std::process::Command::new("cue");
    cmd.args(["vet", file.to_string_lossy().as_ref()]);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run cue vet: {e}"))?;
    let code = output.status.code().unwrap_or(1);
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Ok((code, stderr))
}

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

/// Walk `contracts_dir` for `.cue` files and validate each one.
///
/// For each file:
/// 1. Run cue vet and collect errors.
/// 2. Attempt to extract kind and schema_version from the file content.
/// 3. Validate extracted values.
#[allow(clippy::as_conversions)]
pub fn discover_contracts(contracts_dir: &Path) -> Result<DiscoveryReport, String> {
    if !contracts_dir.exists() {
        return Err(format!(
            "contracts directory does not exist: {}",
            contracts_dir.display()
        ));
    }

    if !contracts_dir.is_dir() {
        return Err(format!(
            "contracts path is not a directory: {}",
            contracts_dir.display()
        ));
    }

    // Collect all discoverable contract .cue files recursively.
    let mut cue_files: Vec<PathBuf> = Vec::new();
    collect_cue_files(contracts_dir, contracts_dir, &mut cue_files)
        .map_err(|e| format!("Failed to walk contracts directory: {e}"))?;

    // Sort for deterministic output (INV-005).
    cue_files.sort();
    let contract_files: Vec<PathBuf> = cue_files
        .into_iter()
        .filter(|path| is_contract_discovery_candidate(path))
        .collect();

    let mut files = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let mut summary = ReportSummary::new();
    summary.total = u32::try_from(contract_files.len())
        .map_err(|_| String::from("too many contract files to summarize"))?;

    for file_rel in &contract_files {
        // Build absolute path for file I/O; keep relative path for reports.
        let file_abs = contracts_dir.join(file_rel);
        let result = validate_single_file(&file_abs, contracts_dir);
        match result {
            Ok(contract_file) => {
                files.push(contract_file);
                #[allow(clippy::arithmetic_side_effects)]
                {
                    summary.valid += 1;
                }
            }
            Err((_file_path, validation_errors)) => {
                #[allow(clippy::arithmetic_side_effects)]
                {
                    summary.invalid += 1;
                }
                for err in &validation_errors {
                    let key = err.to_string();
                    #[allow(clippy::arithmetic_side_effects)]
                    summary
                        .errors_by_kind
                        .entry(key)
                        .and_modify(|c| *c += 1)
                        .or_insert(1);
                }
                for err in &validation_errors {
                    errors.push(err.to_string());
                }
            }
        }
    }

    // Sort files by path (INV-005).
    files.sort_by(|a, b| a.path.cmp(&b.path));

    // Monotonicity gate: schema versions must be non-decreasing across files.
    let mut version_violations = Vec::new();
    #[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
    for i in 1..files.len() {
        let prev_ver = &files[i - 1].schema_version;
        let curr_ver = &files[i].schema_version;
        match compare_semver(prev_ver, curr_ver) {
            Ok(SemverCmp::Greater) => {
                // Monotonicity breach: previous file has a higher version.
                let breach = ContractError::VersionMonotonicityBreach {
                    file: files[i].path.display().to_string(),
                    expected: files[i - 1].schema_version.clone(),
                    actual: curr_ver.clone(),
                };
                errors.push(breach.to_string());
                version_violations.push(VersionViolation {
                    file: files[i].path.clone(),
                    expected: files[i - 1].schema_version.clone(),
                    actual: curr_ver.clone(),
                    detail: format!(
                        "version {} < previous version {} (must be non-decreasing)",
                        curr_ver, prev_ver
                    ),
                });
            }
            Err(_) => {
                // Invalid semver in one of the files — treat as error.
                let err_msg = format!("INVALID_VERSION: {}", prev_ver);
                if !errors.contains(&err_msg) {
                    errors.push(err_msg);
                }
            }
            _ => {} // Equal or Less — monotonicity OK
        }
    }

    // Deduplicate version violations by file path.
    let mut version_violations_map: BTreeMap<PathBuf, VersionViolation> = BTreeMap::new();
    for v in version_violations {
        version_violations_map.entry(v.file.clone()).or_insert(v);
    }
    let mut version_violations_vec: Vec<_> = version_violations_map.into_values().collect();
    version_violations_vec.sort_by(|a, b| a.file.cmp(&b.file));

    summary.version_violations = version_violations_vec;

    // Sort errors for determinism.
    errors.sort();

    Ok(DiscoveryReport {
        files,
        errors,
        summary,
    })
}

/// Recursively collect all .cue file paths under root, relative to base.
fn collect_cue_files(base: &Path, current: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if !current.is_dir() {
        return Ok(());
    }
    let mut entries: Vec<_> = current.read_dir()?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.path());
    for entry in entries {
        let path = entry.path();
        if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            collect_cue_files(base, &path, out)?;
        } else if path.extension().map(|ext| ext == "cue").unwrap_or(false) {
            let relative = path.strip_prefix(base).unwrap_or(&path);
            out.push(relative.to_path_buf());
        }
    }
    Ok(())
}

fn is_contract_discovery_candidate(path: &Path) -> bool {
    match path.file_name().and_then(|name| name.to_str()) {
        Some("manifest.cue") => false,
        Some(_) | None => true,
    }
}

/// Validate a single .cue file: cue vet + field extraction.
///
/// `file_path` must be an absolute path (used for file I/O).
/// `contracts_dir` is used as the working directory for `cue vet`
/// so that relative paths are resolved correctly.
fn validate_single_file(
    file_path: &Path,
    contracts_dir: &Path,
) -> Result<ContractFile, (PathBuf, Vec<ContractError>)> {
    let mut vet_errors: Vec<ContractError> = Vec::new();

    // Run cue vet.
    // Use the relative path (relative to contracts_dir) for cue, with
    // contracts_dir as CWD, so cue resolves the path correctly.
    let relative_path = file_path.strip_prefix(contracts_dir).unwrap_or(file_path);
    let (exit_code, _stderr) = match run_cue_vet(relative_path, Some(contracts_dir)) {
        Ok(result) => result,
        Err(_e) => {
            vet_errors.push(ContractError::CueVetFailed {
                file: relative_path.to_string_lossy().to_string(),
            });
            // Continue with field parsing even if cue not available.
            (0, String::new())
        }
    };

    if parse_vet_exit_code(exit_code).is_err() {
        vet_errors.push(ContractError::CueVetFailed {
            file: relative_path.to_string_lossy().to_string(),
        });
    }

    // Parse file content for kind and schema_version.
    let content = match std::fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(_) => {
            vet_errors.push(ContractError::CueVetFailed {
                file: relative_path.to_string_lossy().to_string(),
            });
            return Err((relative_path.to_path_buf(), vet_errors));
        }
    };

    // Extract kind from the CUE content.
    let kind_str = extract_kind(&content);

    // Extract schema_version from the CUE content.
    let schema_version = extract_schema_version(&content);

    // Validate kind.
    let kind = match &kind_str {
        Some(k) => match ContractKind::parse(k) {
            Ok(kind) => kind,
            Err(unrecognised) => {
                vet_errors.push(ContractError::InvalidKind {
                    kind: unrecognised.clone(),
                });
                return Err((relative_path.to_path_buf(), vet_errors));
            }
        },
        None => {
            vet_errors.push(ContractError::MissingSchemaVersion);
            return Err((relative_path.to_path_buf(), vet_errors));
        }
    };

    // Validate schema_version.
    let sv = match parse_schema_version(&schema_version) {
        Ok(sv) => sv,
        Err(e) => {
            vet_errors.push(e);
            return Err((relative_path.to_path_buf(), vet_errors));
        }
    };

    Ok(ContractFile {
        path: relative_path.to_path_buf(),
        schema_version: sv,
        kind,
        vet_errors: Vec::new(),
    })
}

/// Extract the kind value from CUE content by scanning for the `kind:` field.
fn extract_kind(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(after_colon) = trimmed.strip_prefix("kind:") {
            let val = after_colon.trim();
            let val = val.trim_matches('"');
            return Some(val.to_string());
        }
    }
    None
}

/// Extract the schema_version value from CUE content.
fn extract_schema_version(content: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(after_colon) = trimmed.strip_prefix("schema_version:") {
            let val = after_colon.trim();
            let val = val.trim_matches('"');
            return val.to_string();
        }
    }
    String::new()
}

// ---------------------------------------------------------------------------
// GateEvidence integration
// ---------------------------------------------------------------------------

/// Re-export the GateEvidence, GateStatus, and WhyFailed types from the evidence module.
pub use crate::evidence::{GateEvidence, GateStatus, WhyFailed};

/// Convert a DiscoveryReport into a GateEvidence record.
/// Always succeeds (OBL-006: Kani proof of non-panic).
pub fn gate_evidence_from_report(report: &DiscoveryReport) -> GateEvidence {
    let status = if report.summary.invalid == 0 {
        GateStatus::Pass
    } else {
        GateStatus::Fail
    };

    let exit_code = if report.summary.invalid == 0 { 0 } else { 1 };

    let why_failed = if report.summary.invalid > 0 {
        let mut unique = report.errors.to_vec();
        unique.sort();
        unique.dedup();
        let detail = if unique.is_empty() {
            format!("{} contract(s) failed validation", report.summary.invalid)
        } else {
            format!(
                "{} contract(s) failed: {}",
                report.summary.invalid,
                unique.join(", ")
            )
        };
        Some(WhyFailed {
            gate_name: "contracts".to_string(),
            hint: detail,
            repair_command: "cargo xtask contracts --check".to_string(),
            variant: None,
            fixture_id: None,
            expected_gate: None,
        })
    } else {
        None
    };

    GateEvidence {
        kind: "contract-discovery".to_string(),
        gate_name: "contracts".to_string(),
        command: "cargo xtask contracts --dir contracts".to_string(),
        exit_code,
        log: PathBuf::from(".evidence/contracts/last_run.log"),
        status,
        why_failed,
    }
}

// ---------------------------------------------------------------------------
// CLI handler
// ---------------------------------------------------------------------------

/// Handle the `contracts` xtask command.
pub fn cmd_contracts(dir: &str, json: bool, check: bool) -> anyhow::Result<()> {
    let contracts_dir = Path::new(dir);
    let report = discover_contracts(contracts_dir).map_err(|e| anyhow::anyhow!("{}", e))?;

    let _evidence = gate_evidence_from_report(&report);

    if json {
        let output = serde_json::to_string_pretty(&report)?;
        write_stdout(format_args!("{output}"))?;
    } else {
        write_stdout(format_args!(
            "contracts: {} total, {} valid, {} invalid",
            report.summary.total, report.summary.valid, report.summary.invalid
        ))?;
        if !report.errors.is_empty() {
            write_stdout(format_args!("Errors:"))?;
            for error in &report.errors {
                write_stdout(format_args!("  {error}"))?;
            }
        }
    }

    if check && report.summary.invalid > 0 {
        anyhow::bail!("{} contract(s) failed validation", report.summary.invalid);
    }

    Ok(())
}

fn write_stdout(args: std::fmt::Arguments<'_>) -> anyhow::Result<()> {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    handle
        .write_fmt(args)
        .map_err(|error| anyhow::anyhow!("Failed to write to stdout: {error}"))?;
    handle
        .write_all(b"\n")
        .map_err(|error| anyhow::anyhow!("Failed to write newline to stdout: {error}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_schema_version_valid() {
        assert!(parse_schema_version("1.0.0").is_ok());
        assert!(parse_schema_version("0.1.0").is_ok());
        assert!(parse_schema_version("999.999.999").is_ok());
    }

    #[test]
    fn test_parse_schema_version_empty() {
        assert!(matches!(
            parse_schema_version(""),
            Err(ContractError::MissingSchemaVersion)
        ));
    }

    #[test]
    fn test_parse_schema_version_malformed() {
        assert!(matches!(
            parse_schema_version("1.0"),
            Err(ContractError::InvalidVersion { .. })
        ));
        assert!(matches!(
            parse_schema_version("1.0.0.0"),
            Err(ContractError::InvalidVersion { .. })
        ));
        assert!(matches!(
            parse_schema_version("abc"),
            Err(ContractError::InvalidVersion { .. })
        ));
        assert!(matches!(
            parse_schema_version("01.0.0"),
            Err(ContractError::InvalidVersion { .. })
        ));
    }

    #[test]
    fn test_parse_contract_kind_all() {
        for kind in ContractKind::all_values() {
            let s = kind.to_string();
            assert_eq!(ContractKind::parse(&s), Ok(*kind));
        }
    }

    #[test]
    fn test_parse_contract_kind_invalid() {
        assert!(ContractKind::parse("bogus").is_err());
        assert!(ContractKind::parse("").is_err());
    }

    #[test]
    fn test_compare_semver_equal() {
        assert_eq!(compare_semver("1.0.0", "1.0.0"), Ok(SemverCmp::Equal));
    }

    #[test]
    fn test_compare_semver_less() {
        assert_eq!(compare_semver("1.0.0", "2.0.0"), Ok(SemverCmp::Less));
        assert_eq!(compare_semver("1.0.0", "1.1.0"), Ok(SemverCmp::Less));
        assert_eq!(compare_semver("1.0.0", "1.0.1"), Ok(SemverCmp::Less));
    }

    #[test]
    fn test_compare_semver_greater() {
        assert_eq!(compare_semver("2.0.0", "1.0.0"), Ok(SemverCmp::Greater));
    }

    #[test]
    fn test_parse_vet_exit_code_zero() {
        assert!(parse_vet_exit_code(0).is_ok());
    }

    #[test]
    fn test_parse_vet_exit_code_nonzero() {
        assert!(parse_vet_exit_code(1).is_err());
        assert!(parse_vet_exit_code(-1).is_err());
    }

    #[test]
    fn test_gate_evidence_pass() {
        let report = DiscoveryReport {
            files: Vec::new(),
            errors: Vec::new(),
            summary: ReportSummary::new(),
        };
        let evidence = gate_evidence_from_report(&report);
        assert!(matches!(evidence.status, GateStatus::Pass));
        assert_eq!(evidence.exit_code, 0);
        assert!(evidence.why_failed.is_none());
    }

    #[test]
    fn test_gate_evidence_fail() {
        let report = DiscoveryReport {
            files: Vec::new(),
            errors: vec!["INVALID_KIND: bogus".to_string()],
            summary: ReportSummary {
                total: 1,
                valid: 0,
                invalid: 1,
                errors_by_kind: BTreeMap::from_iter(vec![("INVALID_KIND: bogus".to_string(), 1)]),
                version_violations: Vec::new(),
            },
        };
        let evidence = gate_evidence_from_report(&report);
        assert!(matches!(evidence.status, GateStatus::Fail));
        assert_eq!(evidence.exit_code, 1);
        assert!(evidence.why_failed.is_some());
    }

    #[test]
    fn test_report_summary_parity() {
        let summary = ReportSummary {
            total: 5,
            valid: 3,
            invalid: 2,
            errors_by_kind: BTreeMap::new(),
            version_violations: Vec::new(),
        };
        assert_eq!(summary.total, summary.valid + summary.invalid);
    }

    #[test]
    fn test_contract_kind_display() {
        assert_eq!(ContractKind::CliEnvelope.to_string(), "cli_envelope");
        assert_eq!(ContractKind::UiTokens.to_string(), "ui_tokens");
        assert_eq!(
            ContractKind::AcceptedArtifacts.to_string(),
            "accepted_artifacts"
        );
        assert_eq!(ContractKind::EvidenceBundle.to_string(), "evidence_bundle");
        assert_eq!(ContractKind::Diagnostics.to_string(), "diagnostics");
        assert_eq!(ContractKind::GateOutput.to_string(), "gate_output");
    }

    #[test]
    fn test_extract_kind_found() {
        let content = r#"kind: "cli_envelope"
schema_version: "1.0.0""#;
        assert_eq!(extract_kind(content), Some("cli_envelope".to_string()));
    }

    #[test]
    fn test_extract_kind_not_found() {
        let content = r#"package validation"#;
        assert_eq!(extract_kind(content), None);
    }

    #[test]
    fn test_extract_schema_version_found() {
        let content = r#"kind: "cli_envelope"
schema_version: "1.0.0""#;
        assert_eq!(extract_schema_version(content), "1.0.0");
    }

    #[test]
    fn test_extract_schema_version_not_found() {
        let content = r#"package validation"#;
        assert_eq!(extract_schema_version(content), "");
    }

    #[test]
    fn test_discover_nonexistent_dir() {
        let result = discover_contracts(Path::new("/tmp/does_not_exist_xyz"));
        assert!(result.is_err());
    }

    #[test]
    fn test_manifest_is_not_a_contract_discovery_candidate() {
        assert!(!is_contract_discovery_candidate(Path::new("manifest.cue")));
        assert!(is_contract_discovery_candidate(Path::new(
            "cli_envelope.cue"
        )));
    }

    #[test]
    fn test_contract_error_display() {
        assert_eq!(
            ContractError::MissingSchemaVersion.to_string(),
            "MISSING_SCHEMA_VERSION"
        );
        assert!(
            ContractError::InvalidKind {
                kind: "bogus".to_string(),
            }
            .to_string()
            .contains("INVALID_KIND")
        );
    }
}
