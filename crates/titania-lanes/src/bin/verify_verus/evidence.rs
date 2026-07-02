use std::{
    fmt::Write,
    fs,
    io,
    path::Path,
};

pub struct SummaryStatus<'a> {
    pub target_failures: &'a [String],
    pub forbidden_count: usize,
    pub external_marker_count: usize,
    pub external_markers_waived: bool,
}

/// Write a summary header line to an evidence file, creating it if it does not exist.
///
/// # Errors
///
/// - Fails if the file cannot be written to disk (I/O error).
pub fn write_summary_header(path: &Path, target_count: usize) -> io::Result<()> {
    let evidence = path
        .parent()
        .map_or_else(|| ".".to_owned(), |parent| parent.display().to_string());
    let body = format!("VERUS_REGISTRY evidence={evidence}\nVERUS_TARGET_COUNT {target_count}\n");
    fs::write(path, body)
}

/// Append a "not applicable" marker to the evidence file.
///
/// # Errors
///
/// - Fails if the evidence file cannot be read or written (I/O error).
pub fn append_not_applicable(path: &Path, reason: &str) -> io::Result<()> {
    append(path, &format!("VERUS_REGISTRY_NOT_APPLICABLE {reason}\n"))
}

/// Append a summary status block to the evidence file.
///
/// # Errors
///
/// - Fails if the evidence file cannot be read or written (I/O error).
pub fn append_summary_status(path: &Path, status: &SummaryStatus<'_>) -> io::Result<()> {
    let mut existing = read_existing(path)?;
    append_target_status(&mut existing, status.target_failures);
    append_forbidden_status(&mut existing, status.forbidden_count);
    append_external_status(
        &mut existing,
        status.external_marker_count,
        status.external_markers_waived,
    );
    if registry_ok(status) {
        existing.push_str("VERUS_REGISTRY_OK\n");
    } else {
        existing.push_str("VERUS_REGISTRY_FAILED\n");
    }
    fs::write(path, existing)
}

/// Write an external marker inventory file.
///
/// # Errors
///
/// - Fails if the file cannot be written to disk (I/O error).
pub fn write_external_marker_inventory(
    evidence_dir: &Path,
    file_name: &str,
    lines: &[String],
) -> io::Result<()> {
    let body = if lines.is_empty() { String::new() } else { lines.join("\n") };
    fs::write(evidence_dir.join(file_name), body)
}

/// Append text to an existing file, creating it or reading what's there first.
///
/// # Errors
///
/// - Fails if the file cannot be read or written (I/O error).
fn append(path: &Path, text: &str) -> io::Result<()> {
    let mut existing = read_existing(path)?;
    existing.push_str(text);
    fs::write(path, existing)
}

/// Read a file into a String; returns an empty string if the file does not exist.
///
/// # Errors
///
/// - Fails if the file exists but cannot be read (I/O error).
fn read_existing(path: &Path) -> io::Result<String> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(e),
    }
}

fn append_target_status(existing: &mut String, target_failures: &[String]) {
    if target_failures.is_empty() {
        existing.push_str("VERUS_TARGETS_OK\n");
    } else {
        let _ = write!(existing, "VERUS_TARGET_FAILURE_COUNT {}\n", target_failures.len());
        for failure in target_failures {
            existing.push_str("VERUS_TARGET_FAILED ");
            existing.push_str(failure);
            existing.push('\n');
        }
    }
}

fn append_forbidden_status(existing: &mut String, forbidden_count: usize) {
    if forbidden_count == 0 {
        existing.push_str("VERUS_FORBIDDEN_TRUST_SCAN_OK\n");
    } else {
        let _ = write!(existing, "VERUS_FORBIDDEN_TRUST_FAILURE_COUNT {forbidden_count}\n");
    }
}

fn append_external_status(existing: &mut String, external_marker_count: usize, waived: bool) {
    match (external_marker_count, waived) {
        (0, _) => existing.push_str("VERUS_EXTERNAL_MARKER_SCAN_OK\n"),
        (count, true) => {
            let _ = write!(existing, "VERUS_EXTERNAL_MARKER_WAIVED_COUNT {count}\n");
        }
        (count, false) => {
            let _ = write!(existing, "VERUS_EXTERNAL_MARKER_FAILURE_COUNT {count}\n");
        }
    }
}

const fn registry_ok(status: &SummaryStatus<'_>) -> bool {
    status.target_failures.is_empty()
        && status.forbidden_count == 0
        && (status.external_marker_count == 0 || status.external_markers_waived)
}
