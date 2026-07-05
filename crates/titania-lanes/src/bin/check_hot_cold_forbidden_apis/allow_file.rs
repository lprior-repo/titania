use std::{collections::BTreeSet, fs, path::Path};

use thiserror::Error;

type AllowKey = (String, String);
type AllowSet = BTreeSet<AllowKey>;

/// Errors from allow-file loading and parsing.
#[derive(Debug, Error)]
pub(super) enum AllowFileError {
    /// The allow file is unreadable.
    #[error("scripts/hot-cold-forbidden-apis.allow: unreadable: {source}")]
    ReadFailed { source: std::io::Error },
    /// The absence notice could not be written.
    #[error("stderr write failed: {source}")]
    Stderr { source: std::io::Error },
    /// The row is malformed or has wrong field count.
    #[error("{0}")]
    Malformed(String),
    /// The row is overbroad or missing required metadata.
    #[error("{0}")]
    Overbroad(String),
}
/// Load and validate the hot/cold allow file.
///
/// # Errors
///
/// Returns an error when the allow file is unreadable or contains malformed
/// or overbroad rows.
pub(super) fn load_allow_file(root: &Path) -> Result<AllowSet, AllowFileError> {
    let allow_path = root.join("scripts/hot-cold-forbidden-apis.allow");
    if !allow_path.exists() {
        crate::write_stderr_line(format_args!(
            "NotApplicable: hot/cold forbidden API allow file absent"
        ))
        .map_err(|error| AllowFileError::Stderr { source: error })?;
        return Ok(AllowSet::new());
    }
    let text = fs::read_to_string(&allow_path)
        .map_err(|error| AllowFileError::ReadFailed { source: error })?;
    text.lines().enumerate().try_fold(BTreeSet::new(), collect_allow_entry)
}

/// Fold one allow-file line into the accumulated allow set.
///
/// # Errors
///
/// Returns row parsing or validation errors for malformed allow entries.
fn collect_allow_entry(
    mut acc: AllowSet,
    (index, line): (usize, &str),
) -> Result<AllowSet, AllowFileError> {
    if let Some(entry) = parse_allow_entry(index, line)? {
        let _ = acc.insert(entry);
    }
    Ok(acc)
}

/// Parse one allow-file row.
///
/// # Errors
///
/// Returns an error when the row is malformed or fails allow-entry
/// validation.
fn parse_allow_entry(index: usize, line: &str) -> Result<Option<AllowKey>, AllowFileError> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return Ok(None);
    }
    let parts: Vec<&str> = trimmed.split('|').collect();
    let [path, class, owner, reviewed_by, test, reason] = parts.as_slice() else {
        return Err(AllowFileError::Malformed(format!(
            "MalformedException: scripts/hot-cold-forbidden-apis.allow:{} expected path|class|owner=...|reviewed_by=...|test=...|reason=...",
            index.saturating_add(1)
        )));
    };
    let entry = AllowEntry { index, path, class, owner, reviewed_by, test, reason };
    validate_allow_entry(&entry)?;
    Ok(Some((entry.path.to_owned(), entry.class.to_owned())))
}

struct AllowEntry<'a> {
    index: usize,
    path: &'a str,
    class: &'a str,
    owner: &'a str,
    reviewed_by: &'a str,
    test: &'a str,
    reason: &'a str,
}

/// Validate a parsed allow-file row.
///
/// # Errors
///
/// Returns an error when the row is overbroad or missing required metadata.
fn validate_allow_entry(entry: &AllowEntry<'_>) -> Result<(), AllowFileError> {
    if entry.path.contains('*')
        || !entry.path.starts_with("crates/")
        || !std::path::Path::new(&entry.path)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("rs"))
    {
        return Err(AllowFileError::Overbroad(format!(
            "OverbroadException: scripts/hot-cold-forbidden-apis.allow:{} path must be exact crates/*/src/*.rs",
            entry.index.saturating_add(1)
        )));
    }
    if entry.class == "ALL" || entry.class.contains('*') {
        return Err(AllowFileError::Overbroad(format!(
            "OverbroadException: scripts/hot-cold-forbidden-apis.allow:{} class must be exact",
            entry.index.saturating_add(1)
        )));
    }
    if !entry.owner.starts_with("owner=")
        || !entry.reviewed_by.starts_with("reviewed_by=")
        || !entry.test.starts_with("test=")
        || !entry.reason.starts_with("reason=")
    {
        return Err(AllowFileError::Malformed(format!(
            "MalformedException: scripts/hot-cold-forbidden-apis.allow:{} missing owner/reviewed_by/test/reason",
            entry.index.saturating_add(1)
        )));
    }
    Ok(())
}
