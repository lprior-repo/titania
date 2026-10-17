use std::{
    io,
    path::{Path, PathBuf},
};

use titania_aggregate::{
    ReaderError, ReportAssemblyError, assemble_report, build_quality_receipt,
    compute_evidence_digest, read_lane_artifacts,
};
use titania_core::{
    Digest, GateScope, Lane, LaneOutcome, LaneReceipt, QualityReceipt, ReceiptDigests, Report,
    ReportKind,
};
use titania_policy::{PolicyDefaults, PolicyDigest};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReportJson {
    json: String,
    status: ReportStatus,
}

impl ReportJson {
    #[must_use]
    pub(crate) fn json(&self) -> &str {
        &self.json
    }

    #[must_use]
    pub(crate) const fn status(&self) -> ReportStatus {
        self.status
    }

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
fn gate_failure_lines(parsed: &serde_json::Value) -> Vec<String> {
    parsed.get("gate_failures").and_then(|v| v.as_array()).map_or_else(Vec::new, |gates| {
        gates.iter().enumerate().filter_map(gate_failure_reason).collect::<Vec<_>>()
    })
}

fn gate_failure_reason((i, gate): (usize, &serde_json::Value)) -> Option<String> {
    gate.get("InfraFailure")
        .and_then(|f| f.get("reason"))
        .and_then(|r| r.as_str())
        .map(|reason_str| format!("  gate failure {i}: {}", reason_str.trim()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReportStatus {
    Pass,
    Reject,
    PolicyError,
    InputError,
}

impl ReportStatus {
    #[must_use]
    const fn from_report(report: &Report) -> Self {
        match report.kind() {
            ReportKind::Pass => Self::Pass,
            ReportKind::Reject => Self::Reject,
            ReportKind::PolicyError => Self::PolicyError,
            ReportKind::InputError => Self::InputError,
        }
    }
}

pub(crate) fn report_json(
    target_root: &Path,
    scope: GateScope,
) -> Result<ReportJson, AggregateError> {
    let lane_artifacts = read_lane_artifacts(target_root, scope).map_err(AggregateError::Read)?;
    let receipt = quality_receipt(target_root, scope, &lane_artifacts)?;
    let entries: Box<[_]> = lane_artifacts
        .into_iter()
        .map(|(lane, outcome)| titania_core::PerLaneEntry::new(lane, outcome))
        .collect();
    let report = assemble_report(scope, entries, receipt, Box::new([]), Box::new([]))?;
    let status = ReportStatus::from_report(&report);
    serde_json::to_string(&report)
        .map(|json| ReportJson { json, status })
        .map_err(AggregateError::Serialize)
}

#[derive(Debug)]
pub(crate) enum AggregateError {
    Read(ReaderError),
    Receipt(titania_aggregate::ReceiptBuilderError),
    Report(ReportAssemblyError),
    Serialize(serde_json::Error),
    Digest(std::io::Error),
    ToolchainProbe(String),
}

impl AggregateError {
    #[must_use]
    pub(crate) fn diagnostic(&self) -> String {
        match self {
            Self::Read(error) => format!("InputError: aggregate artifact read failed: {error}"),
            Self::Receipt(error) => format!("InputError: aggregate receipt failed: {error}"),
            Self::Report(error) => format!("InputError: aggregate report failed: {error}"),
            Self::Serialize(error) => serialize_diagnostic(error),
            Self::Digest(error) => format!("InputError: aggregate digest failed: {error}"),
            Self::ToolchainProbe(error) => toolchain_probe_diagnostic(error),
        }
    }
}

fn serialize_diagnostic(error: &serde_json::Error) -> String {
    format!("InputError: aggregate serialization failed: {error}")
}

fn toolchain_probe_diagnostic(error: &str) -> String {
    format!("InternalError: toolchain digest probe failed: {error}")
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

fn quality_receipt(
    target_root: &Path,
    scope: GateScope,
    lane_artifacts: &[(Lane, LaneOutcome)],
) -> Result<QualityReceipt, AggregateError> {
    let digests = receipt_digests(target_root)?;
    let lanes =
        lane_artifacts.iter().map(lane_receipt).collect::<Result<Vec<_>, _>>()?.into_boxed_slice();
    build_quality_receipt(scope, digests, lanes).map_err(Into::into)
}

fn lane_receipt((lane, outcome): &(Lane, LaneOutcome)) -> Result<LaneReceipt, AggregateError> {
    let digest = lane_outcome_digest(outcome)?;
    Ok(LaneReceipt::new(*lane, digest, outcome.is_pass()))
}

fn lane_outcome_digest(outcome: &LaneOutcome) -> Result<Digest, AggregateError> {
    match outcome {
        LaneOutcome::Clean { evidence } => compute_evidence_digest(evidence).map_err(Into::into),
        LaneOutcome::Findings { .. } | LaneOutcome::Failed { .. } | LaneOutcome::Skipped { .. } => {
            serde_json::to_vec(outcome)
                .map(|payload| Digest::from_bytes(&payload))
                .map_err(AggregateError::Serialize)
        }
    }
}

fn receipt_digests(target_root: &Path) -> Result<ReceiptDigests, AggregateError> {
    let source = source_tree_digest(target_root)?;
    let lock = digest_optional_file(&target_root.join("Cargo.lock"), b"missing-cargo-lock")?;
    let policy = policy_digest(target_root)?;
    let toolchain = toolchain_digest(target_root)?;
    Ok(ReceiptDigests::new(source, lock, policy, toolchain))
}

fn source_tree_digest(target_root: &Path) -> Result<Digest, AggregateError> {
    let files = source_digest_files(target_root)?;
    let mut payload = String::new();
    files.iter().try_for_each(|path| append_source_file(target_root, path, &mut payload))?;
    Ok(Digest::from_bytes(payload.as_bytes()))
}

fn source_digest_files(target_root: &Path) -> Result<Vec<PathBuf>, AggregateError> {
    let mut files = Vec::new();
    collect_source_digest_files(target_root, target_root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_source_digest_files(
    target_root: &Path,
    dir: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), AggregateError> {
    std::fs::read_dir(dir)
        .map_err(AggregateError::Digest)?
        .try_for_each(|entry| collect_source_digest_entry(target_root, entry, files))
}

fn collect_source_digest_entry(
    target_root: &Path,
    entry: io::Result<std::fs::DirEntry>,
    files: &mut Vec<PathBuf>,
) -> Result<(), AggregateError> {
    let entry = entry.map_err(AggregateError::Digest)?;
    let path = entry.path();
    if path.is_dir() && !skip_source_digest_dir(&path) {
        return collect_source_digest_files(target_root, &path, files);
    }
    if path.is_file() && include_source_digest_file(&path) {
        files.push(path);
    }
    Ok(())
}

fn skip_source_digest_dir(path: &Path) -> bool {
    path.file_name().and_then(|name| name.to_str()).is_some_and(|name| {
        matches!(name, ".beads" | ".git" | ".moon" | ".titania" | ".worktrees" | "target")
    })
}

fn include_source_digest_file(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "rs")
        || path.file_name().and_then(|name| name.to_str()).is_some_and(|name| {
            matches!(name, "Cargo.toml" | "rustfmt.toml" | "clippy.toml" | "deny.toml")
        })
}

fn append_source_file(
    target_root: &Path,
    path: &Path,
    payload: &mut String,
) -> Result<(), AggregateError> {
    let relative = path.strip_prefix(target_root).map_err(|err| {
        AggregateError::Digest(io::Error::other(format!("source path escaped root: {err}")))
    })?;
    let bytes = std::fs::read(path).map_err(AggregateError::Digest)?;
    push_segment(payload, "source_path", &relative.to_string_lossy());
    push_segment(payload, "source_bytes", &String::from_utf8_lossy(&bytes));
    Ok(())
}

fn policy_digest(target_root: &Path) -> Result<Digest, AggregateError> {
    let profile_dir = target_root.join(".titania").join("profiles").join("strict-ai");
    let policy_toml = read_optional_text(&profile_dir.join("policy.toml"))?;
    let exceptions_toml = read_optional_text(&profile_dir.join("exceptions.toml"))?;
    let deny_toml = read_optional_text(&target_root.join("deny.toml"))?;
    let clippy_toml = read_optional_text(&target_root.join("clippy.toml"))?;
    let digest = PolicyDigest::compute(
        &PolicyDefaults::embedded(),
        policy_toml.as_deref(),
        exceptions_toml.as_deref(),
        deny_toml.as_deref(),
        clippy_toml.as_deref(),
    );
    Ok(digest.digest().clone())
}

fn toolchain_digest(target_root: &Path) -> Result<Digest, AggregateError> {
    let rustc_version = probe_version(target_root, "rustc", &["--version"])?;
    let cargo_version = probe_version(target_root, "cargo", &["--version"])?;
    let toolchain_toml = read_optional_text(&target_root.join("rust-toolchain.toml"))?;
    let toolchain_legacy = read_optional_text(&target_root.join("rust-toolchain"))?;
    let toolchain_text = toolchain_toml.or(toolchain_legacy);

    let mut payload = String::new();
    push_segment(&mut payload, "rustc_version", &rustc_version);
    push_segment(&mut payload, "cargo_version", &cargo_version);
    push_optional_segment(&mut payload, "rust_toolchain_toml", toolchain_text.as_deref());
    Ok(Digest::from_bytes(payload.as_bytes()))
}

fn probe_version(
    target_root: &Path,
    program: &str,
    args: &[&str],
) -> Result<String, AggregateError> {
    let output = std::process::Command::new(program)
        .args(args)
        .current_dir(target_root)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .map_err(|error| AggregateError::ToolchainProbe(error.to_string()))?;
    if !output.status.success() {
        return Err(AggregateError::ToolchainProbe(format!(
            "{program} {} exited with {status}",
            args.join(" "),
            status = output.status,
        )));
    }
    let text = String::from_utf8(output.stdout)
        .map_err(|error| AggregateError::ToolchainProbe(error.to_string()))?
        .trim()
        .to_owned();
    if text.is_empty() {
        return Err(AggregateError::ToolchainProbe(format!(
            "{program} {} produced empty stdout",
            args.join(" "),
        )));
    }
    Ok(text)
}

fn push_segment(payload: &mut String, label: &str, text: &str) {
    payload.push_str(label);
    push_len(payload, text.len());
    payload.push_str(text);
}

fn push_optional_segment(payload: &mut String, label: &str, text: Option<&str>) {
    const ABSENT_MARKER: &str = "<absent>";
    push_segment(payload, label, text.map_or(ABSENT_MARKER, std::convert::identity));
}

fn push_len(payload: &mut String, len: usize) {
    payload.push(':');
    payload.push_str(&len.to_string());
    payload.push(':');
}

fn read_optional_text(path: &Path) -> Result<Option<String>, AggregateError> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(AggregateError::Digest(err)),
    }
}

fn digest_optional_file(path: &Path, missing_marker: &[u8]) -> Result<Digest, AggregateError> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Digest::from_bytes(&bytes)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Digest::from_bytes(missing_marker)),
        Err(err) => Err(AggregateError::Digest(err)),
    }
}
