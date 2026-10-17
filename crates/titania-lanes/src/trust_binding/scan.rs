//! Line-oriented scanner for verification sources.
//!
//! The scanner is deliberately syntactic, never semantic: it records which
//! trust markers, path bindings, and production references a file contains
//! so the audit can classify linkage versus body proof. It performs no I/O.

use super::classify::TrustMarker;

/// Scan result for one verification source file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileSignals {
    /// `true` when any line carries a `#[path ...]` attribute.
    has_path_attr: bool,
    /// Quoted target of the first `#[path = "..."]` attribute, if any.
    path_target: Option<String>,
    /// 1-indexed lines carrying an `assume_specification` bridge.
    assume_spec_lines: Vec<u32>,
    /// 1-indexed lines carrying a bare `assume(`.
    assume_lines: Vec<u32>,
    /// 1-indexed lines carrying `external_body`.
    external_body_lines: Vec<u32>,
    /// 1-indexed lines carrying `admit`.
    admit_lines: Vec<u32>,
    /// 1-indexed lines carrying a `trusted` attribute.
    trusted_lines: Vec<u32>,
    /// Number of lines referencing the `production::` namespace.
    production_ref_count: u32,
    /// Number of lines declaring `proof fn` items.
    proof_fn_count: u32,
    /// `true` for explicit smoke fixtures, which never satisfy obligations.
    is_smoke: bool,
}

impl FileSignals {
    /// `true` when any line carries a `#[path ...]` attribute.
    #[must_use]
    pub const fn has_path_attr(&self) -> bool {
        self.has_path_attr
    }

    /// Quoted target of the first `#[path = "..."]` attribute, if any.
    #[must_use]
    pub fn path_target(&self) -> Option<&str> {
        self.path_target.as_deref()
    }

    /// 1-indexed lines carrying the given trust marker.
    #[must_use]
    pub fn marker_lines(&self, marker: TrustMarker) -> &[u32] {
        match marker {
            TrustMarker::AssumeSpecification => &self.assume_spec_lines,
            TrustMarker::Assume => &self.assume_lines,
            TrustMarker::ExternalBody => &self.external_body_lines,
            TrustMarker::Admit => &self.admit_lines,
            TrustMarker::Trusted => &self.trusted_lines,
        }
    }

    /// Number of lines referencing the `production::` namespace.
    #[must_use]
    pub const fn production_ref_count(&self) -> u32 {
        self.production_ref_count
    }

    /// Number of lines declaring `proof fn` items.
    #[must_use]
    pub const fn proof_fn_count(&self) -> u32 {
        self.proof_fn_count
    }

    /// `true` for explicit smoke fixtures, which never satisfy obligations.
    #[must_use]
    pub const fn is_smoke(&self) -> bool {
        self.is_smoke
    }

    /// `true` when the `#[path]` target names the obligation's production
    /// file (the attribute target contains the workspace-relative path).
    #[must_use]
    pub fn path_binds_production(&self, production_path: &str) -> bool {
        self.path_target.as_deref().is_some_and(|target| target.contains(production_path))
    }
}

/// One scanned verification source: its path plus its [`FileSignals`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedFile {
    /// Workspace-relative file path.
    path: String,
    /// Syntactic signals observed in the file content.
    signals: FileSignals,
}

impl ScannedFile {
    /// Construct a scanned file from a path and already-computed signals.
    #[must_use]
    pub fn new(path: impl Into<String>, signals: FileSignals) -> Self {
        Self { path: path.into(), signals }
    }

    /// Workspace-relative file path.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Syntactic signals observed in the file content.
    #[must_use]
    pub const fn signals(&self) -> &FileSignals {
        &self.signals
    }
}

/// Scan verification source `content` for trust markers, path bindings,
/// production references, proof items, and smoke-fixture markers.
///
/// Full-line `//` comments are skipped, except that the explicit
/// `fixture-smoke` binding marker is honored wherever it appears. A file
/// whose path contains `smoke` is likewise a fixture.
#[must_use]
pub fn scan_proof_content(path: &str, content: &str) -> ScannedFile {
    let mut signals: FileSignals = FileSignals::default();
    if path.contains("smoke") {
        signals.is_smoke = true;
    }
    let mut line_no: u32 = 0;
    for line in content.lines() {
        line_no = line_no.saturating_add(1);
        note_line(&mut signals, line_no, line);
    }
    ScannedFile::new(path, signals)
}

/// Record the signals carried by one source line: smoke markers, trust
/// markers, path bindings, production references, and proof items.
///
/// Full-line `//` comments carry no code signals.
fn note_line(signals: &mut FileSignals, line_no: u32, line: &str) {
    if line.contains("fixture-smoke") {
        signals.is_smoke = true;
    }
    let trimmed: &str = line.trim_start();
    if trimmed.starts_with("//") {
        return;
    }
    note_markers(signals, line_no, trimmed);
    note_references(signals, trimmed);
}

/// Count production references and proof items on one code line.
fn note_references(signals: &mut FileSignals, trimmed: &str) {
    if trimmed.contains("production::") {
        signals.production_ref_count = signals.production_ref_count.saturating_add(1);
    }
    if trimmed.contains("proof fn") {
        signals.proof_fn_count = signals.proof_fn_count.saturating_add(1);
    }
}

/// Record the trust markers and path bindings carried by one code line.
fn note_markers(signals: &mut FileSignals, line_no: u32, trimmed: &str) {
    if trimmed.contains("#[path") && !signals.has_path_attr {
        signals.has_path_attr = true;
        signals.path_target = extract_path_target(trimmed);
    }
    if trimmed.contains("assume_specification") {
        signals.assume_spec_lines.push(line_no);
    } else if trimmed.contains("assume(") {
        signals.assume_lines.push(line_no);
    }
    if trimmed.contains("external_body") {
        signals.external_body_lines.push(line_no);
    }
    if trimmed.contains("admit") {
        signals.admit_lines.push(line_no);
    }
    if trimmed.contains("#[trusted") || trimmed.contains("::trusted") {
        signals.trusted_lines.push(line_no);
    }
}

/// Extract the quoted target of a `#[path = "..."]` attribute line.
///
/// Returns `None` when the line carries no quoted target.
fn extract_path_target(trimmed: &str) -> Option<String> {
    trimmed.split('"').nth(1).map(str::to_owned)
}
