use std::{fs, io, path::Path};

pub(crate) struct SummaryStatus<'a> {
    pub(crate) target_failures: &'a [String],
    pub(crate) forbidden_count: usize,
    pub(crate) external_marker_count: usize,
    pub(crate) external_markers_waived: bool,
}

/// Write the initial Verus evidence summary.
///
/// # Errors
///
/// Returns filesystem errors from writing the summary file.
pub(crate) fn write_summary_header(path: &Path, target_count: usize) -> io::Result<()> {
    let evidence =
        path.parent().map_or_else(|| ".".to_owned(), |parent| parent.display().to_string());
    let body = format!("VERUS_REGISTRY evidence={evidence}\nVERUS_TARGET_COUNT {target_count}\n");
    fs::write(path, body)
}

/// Append a not-applicable status to the summary.
///
/// # Errors
///
/// Returns filesystem errors from reading or writing the summary file.
pub(crate) fn append_not_applicable(path: &Path, reason: &str) -> io::Result<()> {
    append(path, &format!("VERUS_REGISTRY_NOT_APPLICABLE {reason}\n"))
}

/// Append final verification status lines to the summary.
///
/// # Errors
///
/// Returns filesystem errors from reading or writing the summary file.
pub(crate) fn append_summary_status(path: &Path, status: &SummaryStatus<'_>) -> io::Result<()> {
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

/// Write the external-marker inventory evidence file.
///
/// # Errors
///
/// Returns filesystem errors from writing the inventory file.
pub(crate) fn write_external_marker_inventory(
    evidence_dir: &Path,
    file_name: &str,
    lines: &[String],
) -> io::Result<()> {
    let body = if lines.is_empty() { String::new() } else { lines.join("\n") };
    fs::write(evidence_dir.join(file_name), body)
}

/// Append text to an evidence file by reading and rewriting it.
///
/// # Errors
///
/// Returns filesystem errors from reading or writing the file.
fn append(path: &Path, text: &str) -> io::Result<()> {
    let mut existing = read_existing(path)?;
    existing.push_str(text);
    fs::write(path, existing)
}

/// Read an evidence file, treating absence as empty content.
///
/// # Errors
///
/// Returns filesystem errors other than a missing file.
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
        append_count_line(existing, "VERUS_TARGET_FAILURE_COUNT", target_failures.len());
        existing.extend(
            target_failures.iter().map(|failure| format!("VERUS_TARGET_FAILED {failure}\n")),
        );
    }
}

fn append_forbidden_status(existing: &mut String, forbidden_count: usize) {
    if forbidden_count == 0 {
        existing.push_str("VERUS_FORBIDDEN_TRUST_SCAN_OK\n");
    } else {
        append_count_line(existing, "VERUS_FORBIDDEN_TRUST_FAILURE_COUNT", forbidden_count);
    }
}

fn append_external_status(existing: &mut String, external_marker_count: usize, waived: bool) {
    match (external_marker_count, waived) {
        (0, _) => existing.push_str("VERUS_EXTERNAL_MARKER_SCAN_OK\n"),
        (count, true) => {
            append_count_line(existing, "VERUS_EXTERNAL_MARKER_WAIVED_COUNT", count);
        }
        (count, false) => {
            append_count_line(existing, "VERUS_EXTERNAL_MARKER_FAILURE_COUNT", count);
        }
    }
}

fn append_count_line(existing: &mut String, label: &str, count: usize) {
    existing.push_str(label);
    existing.push(' ');
    existing.push_str(&count.to_string());
    existing.push('\n');
}

const fn registry_ok(status: &SummaryStatus<'_>) -> bool {
    status.target_failures.is_empty()
        && status.forbidden_count == 0
        && (status.external_marker_count == 0 || status.external_markers_waived)
}
