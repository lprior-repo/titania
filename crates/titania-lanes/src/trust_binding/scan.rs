use super::classify::TrustMarker;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileSignals {
    has_path_attr: bool,
    path_target: Option<String>,
    assume_spec_lines: Vec<u32>,
    assume_lines: Vec<u32>,
    external_body_lines: Vec<u32>,
    admit_lines: Vec<u32>,
    trusted_lines: Vec<u32>,
    production_ref_count: u32,
    proof_fn_count: u32,
    is_smoke: bool,
}

impl FileSignals {
    #[must_use]
    pub const fn has_path_attr(&self) -> bool {
        self.has_path_attr
    }

    #[must_use]
    pub fn path_target(&self) -> Option<&str> {
        self.path_target.as_deref()
    }

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

    #[must_use]
    pub const fn production_ref_count(&self) -> u32 {
        self.production_ref_count
    }

    #[must_use]
    pub const fn proof_fn_count(&self) -> u32 {
        self.proof_fn_count
    }

    #[must_use]
    pub const fn is_smoke(&self) -> bool {
        self.is_smoke
    }

    #[must_use]
    pub fn path_binds_production(&self, production_path: &str) -> bool {
        self.path_target.as_deref().is_some_and(|target| target.contains(production_path))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedFile {
    path: String,
    signals: FileSignals,
}

impl ScannedFile {
    #[must_use]
    pub fn new(path: impl Into<String>, signals: FileSignals) -> Self {
        Self { path: path.into(), signals }
    }

    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub const fn signals(&self) -> &FileSignals {
        &self.signals
    }
}

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

fn note_references(signals: &mut FileSignals, trimmed: &str) {
    if trimmed.contains("production::") {
        signals.production_ref_count = signals.production_ref_count.saturating_add(1);
    }
    if trimmed.contains("proof fn") {
        signals.proof_fn_count = signals.proof_fn_count.saturating_add(1);
    }
}

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

fn extract_path_target(trimmed: &str) -> Option<String> {
    trimmed.split('"').nth(1).map(str::to_owned)
}
