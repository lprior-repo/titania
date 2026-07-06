//! Aggregate dispatch for existing lane artifacts.
//!
//! This shell module performs no lane execution. It reads artifacts already
//! written under `.titania/out/<scope>/`, delegates classification to
//! `titania-aggregate`, and renders the typed report as JSON.

use std::path::{Path, PathBuf};

use titania_aggregate::{
    ReaderError, ReportAssemblyError, assemble_report, build_quality_receipt,
    compute_evidence_digest, read_lane_artifacts,
};
use titania_core::{
    Digest, GateScope, Lane, LaneOutcome, LaneReceipt, QualityReceipt, ReceiptDigests, Report,
};

/// Serialized aggregate report plus its CLI exit classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReportJson {
    json: String,
    status: ReportStatus,
}

impl ReportJson {
    /// Borrow the serialized report.
    #[must_use]
    pub(crate) fn json(&self) -> &str {
        &self.json
    }

    /// Return the report status classification.
    #[must_use]
    pub(crate) const fn status(&self) -> ReportStatus {
        self.status
    }

    /// Render the report as human-readable text.
    #[must_use]
    pub(crate) fn render_human(&self) -> String {
        render_report_human(&self.json)
    }
}

fn render_report_human(json: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(json) {
        Ok(parsed) => render_parsed_report(&parsed),
        Err(error) => render_unparseable_report(json, &error),
    }
}

fn render_unparseable_report(json: &str, error: &serde_json::Error) -> String {
    format!(
        "titania-check aggregate report — status: unknown\nUnparseable report JSON: {error}\n\nRaw report:\n{json}"
    )
}

fn render_parsed_report(parsed: &serde_json::Value) -> String {
    let variant = parsed
        .get("variant")
        .and_then(serde_json::Value::as_str)
        .map_or("unknown", std::convert::identity);
    let gate_failures = parsed
        .get("gate_failures")
        .and_then(serde_json::Value::as_array)
        .map_or(0, std::vec::Vec::len);
    let code_findings = parsed
        .get("code_findings")
        .and_then(serde_json::Value::as_array)
        .map_or(0, std::vec::Vec::len);
    let per_lane =
        parsed.get("per_lane").and_then(serde_json::Value::as_array).map_or(0, std::vec::Vec::len);

    let mut lines = Vec::new();
    lines.push(format!("titania-check aggregate report — status: {variant}"));
    lines.push(String::new());
    lines.push(format!("Gate failures: {gate_failures}"));
    lines.push(format!("Code findings: {code_findings}"));
    lines.push(format!("Lanes evaluated: {per_lane}"));
    lines.extend(gate_failure_lines(parsed));
    lines.push(String::new());
    lines.push(format!("Report variant: {variant}"));
    lines.join("\n")
}
/// Extract gate failure reason lines from the parsed report JSON.
fn gate_failure_lines(parsed: &serde_json::Value) -> Vec<String> {
    parsed.get("gate_failures").and_then(|v| v.as_array()).map_or_else(Vec::new, |gates| {
        gates.iter().enumerate().filter_map(gate_failure_reason).collect::<Vec<_>>()
    })
}

/// Extract a single gate failure reason line from a gate JSON object.
fn gate_failure_reason((i, gate): (usize, &serde_json::Value)) -> Option<String> {
    gate.get("infra_failure")
        .and_then(|f| f.get("reason"))
        .and_then(|r| r.as_str())
        .map(|reason_str| format!("  gate failure {i}: {}", reason_str.trim()))
}

/// Exit-code classification for a typed report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReportStatus {
    /// All scoped lanes passed.
    Pass,
    /// Code findings or gate failures rejected the run.
    Reject,
    /// Policy diagnostics rejected evaluation before lane classification.
    PolicyError,
    /// Input diagnostics rejected evaluation before lane classification.
    InputError,
}

impl ReportStatus {
    #[must_use]
    const fn from_report(report: &Report) -> Self {
        match report {
            Report::Pass { .. } => Self::Pass,
            Report::Reject { .. } => Self::Reject,
            Report::PolicyError { .. } => Self::PolicyError,
            Report::InputError { .. } => Self::InputError,
        }
    }
}

/// Build aggregate report JSON for the target workspace root.
///
/// # Errors
///
/// Returns [`AggregateError`] when artifacts cannot be read, report invariants
/// fail, receipt construction fails, or report serialization fails.
pub(crate) fn report_json(
    target_root: &Path,
    scope: GateScope,
) -> Result<ReportJson, AggregateError> {
    // No Cargo.toml gate here: both `aggregate` and `check` read existing lane
    // artifacts (or report them missing).  Lane execution that needs a project
    // is handled separately by the `run-lane` subcommand.
    let lane_artifacts = read_lane_artifacts(target_root, scope).map_err(AggregateError::Read)?;
    let receipt = quality_receipt(target_root, scope, &lane_artifacts)?;
    let entries: Box<[_]> = lane_artifacts
        .into_iter()
        .map(|(lane, outcome)| titania_core::PerLaneEntry { lane, outcome })
        .collect();
    let report = assemble_report(scope, entries, receipt, Box::new([]), Box::new([]))?;
    let status = ReportStatus::from_report(&report);
    serde_json::to_string(&report)
        .map(|json| ReportJson { json, status })
        .map_err(AggregateError::Serialize)
}

/// Aggregate dispatch failure.
#[derive(Debug)]
pub(crate) enum AggregateError {
    /// Lane artifacts could not be read or parsed.
    Read(ReaderError),
    /// Receipt construction failed.
    Receipt(titania_aggregate::ReceiptBuilderError),
    /// Report construction failed.
    Report(ReportAssemblyError),
    /// Report serialization failed.
    Serialize(serde_json::Error),
}

impl AggregateError {
    /// Render the failure as a stable CLI diagnostic.
    #[must_use]
    pub(crate) fn diagnostic(&self) -> String {
        match self {
            Self::Read(error) => format!("InputError: aggregate artifact read failed: {error}"),
            Self::Receipt(error) => format!("InputError: aggregate receipt failed: {error}"),
            Self::Report(error) => format!("InputError: aggregate report failed: {error}"),
            Self::Serialize(error) => serialize_diagnostic(error),
        }
    }
}

fn serialize_diagnostic(error: &serde_json::Error) -> String {
    format!("InputError: aggregate serialization failed: {error}")
}

impl From<titania_aggregate::ReceiptBuilderError> for AggregateError {
    fn from(error: titania_aggregate::ReceiptBuilderError) -> Self {
        Self::Receipt(error)
    }
}

impl From<ReportAssemblyError> for AggregateError {
    fn from(error: ReportAssemblyError) -> Self {
        Self::Report(error)
    }
}

/// Build the pass-path quality receipt from lane outcomes.
///
/// # Errors
///
/// Returns [`AggregateError`] when lane evidence cannot be digested or the
/// receipt constructor rejects scope/digest invariants.
fn quality_receipt(
    target_root: &Path,
    scope: GateScope,
    lane_artifacts: &[(Lane, LaneOutcome)],
) -> Result<QualityReceipt, AggregateError> {
    let digests = receipt_digests(target_root);
    let lanes =
        lane_artifacts.iter().map(lane_receipt).collect::<Result<Vec<_>, _>>()?.into_boxed_slice();
    build_quality_receipt(scope, digests, lanes).map_err(Into::into)
}

/// Build one lane receipt entry from a lane outcome.
///
/// # Errors
///
/// Returns [`AggregateError`] when the lane outcome digest cannot be computed.
fn lane_receipt((lane, outcome): &(Lane, LaneOutcome)) -> Result<LaneReceipt, AggregateError> {
    let digest = lane_outcome_digest(outcome)?;
    Ok(LaneReceipt::new(*lane, digest, outcome.is_pass()))
}

/// Compute the stable digest for one lane outcome.
///
/// # Errors
///
/// Returns [`AggregateError`] when a non-clean outcome cannot be serialized or
/// clean evidence cannot be digested.
fn lane_outcome_digest(outcome: &LaneOutcome) -> Result<Digest, AggregateError> {
    match outcome {
        LaneOutcome::Clean { evidence } => compute_evidence_digest(evidence).map_err(Into::into),
        LaneOutcome::Findings { .. } | LaneOutcome::Failed(_) | LaneOutcome::Skipped { .. } => {
            serde_json::to_vec(outcome)
                .map(|payload| Digest::from_bytes(&payload))
                .map_err(AggregateError::Serialize)
        }
    }
}

fn receipt_digests(target_root: &Path) -> ReceiptDigests {
    ReceiptDigests::new(
        digest_optional_file(target_root.join("Cargo.toml"), b"missing-cargo-toml"),
        digest_optional_file(target_root.join("Cargo.lock"), b"missing-cargo-lock"),
        digest_optional_file(
            target_root.join(".titania").join("profiles").join("strict-ai").join("policy.toml"),
            b"missing-strict-ai-policy",
        ),
        Digest::from_bytes(env!("CARGO_PKG_VERSION").as_bytes()),
    )
}

fn digest_optional_file(path: PathBuf, missing_marker: &[u8]) -> Digest {
    std::fs::read(path)
        .map_or_else(|_| Digest::from_bytes(missing_marker), |bytes| Digest::from_bytes(&bytes))
}
