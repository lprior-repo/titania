use std::{
    collections::BTreeSet,
    fs, io,
    path::{Path, PathBuf},
};

use crate::{
    allow_file::load_allow_file,
    model::{COLD_MARKERS, FindingData, HOT_CRATES, SourceRole},
    syntax::{ApiSourceLine, compact, remove_spaces},
};

type ScanOutcome = (Vec<String>, Vec<FindingData>, Vec<FindingData>);

/// Scan hot source files for forbidden APIs.
///
/// # Errors
///
/// Returns an error when allow-file loading or source enumeration fails, or
/// when a hot source file cannot be read.
pub(super) fn scan(root: &Path) -> Result<ScanOutcome, String> {
    let allowed = load_allow_file(root)?;
    let sources = hot_sources(root).map_err(|error| format!("hot source scan failed: {error}"))?;
    let mut state = ScanState::new(allowed);
    sources.iter().try_for_each(|source| scan_source(&mut state, root, source))?;
    Ok(state.finish())
}

struct ScanState {
    allowed: BTreeSet<(String, String)>,
    classified: Vec<String>,
    violations: Vec<FindingData>,
    justified: Vec<FindingData>,
}

impl ScanState {
    const fn new(allowed: BTreeSet<(String, String)>) -> Self {
        Self { allowed, classified: Vec::new(), violations: Vec::new(), justified: Vec::new() }
    }

    fn finish(self) -> ScanOutcome {
        (self.classified, self.violations, self.justified)
    }
}

/// Scan one source file already selected as part of the hot/cold domain.
///
/// # Errors
///
/// Returns an error when a hot production source cannot be read.
fn scan_source(state: &mut ScanState, root: &Path, source: &Path) -> Result<(), String> {
    let rel_path = relative_path(root, source);
    let role = source_role(&rel_path);
    state.classified.push(format!("ClassifiedPath|{role:?}|{rel_path}"));
    if role != SourceRole::HotProduction {
        return Ok(());
    }
    let text = fs::read_to_string(source)
        .map_err(|error| format!("{}: unreadable: {error}", source.display()))?;
    scan_hot_text(state, &rel_path, &text);
    Ok(())
}

fn scan_hot_text(state: &mut ScanState, rel_path: &str, text: &str) {
    let mut line_state = HotLineState::default();
    text.lines().enumerate().for_each(|(index, line)| {
        scan_hot_line(state, rel_path, index.saturating_add(1), line, &mut line_state);
    });
}

fn scan_hot_line(
    state: &mut ScanState,
    rel_path: &str,
    line_no: usize,
    line: &str,
    line_state: &mut HotLineState,
) {
    if skip_test_scope_line(&mut line_state.test_scope, line) {
        return;
    }
    let source_line = ApiSourceLine::parse(line, &mut line_state.block_comment);
    let findings = classify_line(rel_path, line_no, &source_line);
    let (justified, violations): (Vec<FindingData>, Vec<FindingData>) =
        findings.into_iter().partition(|finding| allowed_finding(&state.allowed, finding));
    state.justified.extend(justified);
    state.violations.extend(violations);
}

fn allowed_finding(allowed: &BTreeSet<(String, String)>, finding: &FindingData) -> bool {
    let key = (finding.rel_path.clone(), finding.class_id.to_owned());
    allowed.contains(&key)
}

#[derive(Default)]
struct HotLineState {
    block_comment: bool,
    test_scope: TestScope,
}

#[derive(Default)]
struct TestScope {
    cfg_test_pending: bool,
    depth: i32,
}

fn skip_test_scope_line(scope: &mut TestScope, line: &str) -> bool {
    let trimmed = line.trim();
    if scope.depth > 0_i32 {
        scope.depth = next_depth(scope.depth, line);
        return true;
    }
    if trimmed.starts_with("#[cfg(test)]") {
        scope.cfg_test_pending = true;
        return true;
    }
    if scope.cfg_test_pending && trimmed.contains("mod ") {
        scope.depth = initial_test_depth(line);
        scope.cfg_test_pending = false;
        return true;
    }
    clear_pending_for_code(scope, trimmed);
    false
}

fn clear_pending_for_code(scope: &mut TestScope, trimmed: &str) {
    if !trimmed.is_empty() && !trimmed.starts_with('#') {
        scope.cfg_test_pending = false;
    }
}

fn next_depth(current: i32, line: &str) -> i32 {
    current.saturating_add(char_count_i32(line, '{')).saturating_sub(char_count_i32(line, '}'))
}

fn initial_test_depth(line: &str) -> i32 {
    let depth = char_count_i32(line, '{').saturating_sub(char_count_i32(line, '}'));
    if depth <= 0 { 1 } else { depth }
}

fn char_count_i32(line: &str, needle: char) -> i32 {
    i32::try_from(line.matches(needle).count()).map_or(i32::MAX, core::convert::identity)
}

fn relative_path(root: &Path, source: &Path) -> String {
    match source.strip_prefix(root) {
        Ok(path) => path.display().to_string(),
        Err(_error) => source.display().to_string(),
    }
}

fn source_role(path: &str) -> SourceRole {
    if is_test_path(path) {
        return SourceRole::Test;
    }
    if path.contains("/src/bin/") || path.starts_with("crates/titania-lanes/") {
        return SourceRole::LaneBinary;
    }
    if is_cold_path(path) {
        return SourceRole::ColdSupport;
    }
    if path.starts_with("crates/titania-core/src/") {
        SourceRole::HotProduction
    } else {
        SourceRole::ColdSupport
    }
}

fn is_test_path(path: &str) -> bool {
    path.contains("/tests/")
        || path.ends_with("/tests.rs")
        || path.ends_with("_tests.rs")
        || path.contains("/benches/")
        || path.contains("/kani/")
        || path.ends_with("/kani.rs")
}

fn is_cold_path(path: &str) -> bool {
    path.split(['/', '.', '_', '-']).any(|token| COLD_MARKERS.contains(&token))
}

fn line_has_string_map(line: &str) -> bool {
    let normalized = remove_spaces(line);
    [
        "HashMap<String",
        "HashMap<&str",
        "BTreeMap<String",
        "BTreeMap<&str",
        "IndexMap<String",
        "IndexMap<&str",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn classify_line(rel_path: &str, line_no: usize, source_line: &ApiSourceLine) -> Vec<FindingData> {
    let stripped = source_line.code();
    if stripped.is_empty() || stripped.starts_with('#') || stripped.starts_with("use ") {
        return Vec::new();
    }
    let text = compact(stripped);
    checks_for(stripped)
        .into_iter()
        .filter(|(_class_id, matched)| *matched)
        .map(|(class_id, _matched)| FindingData {
            rel_path: rel_path.to_owned(),
            line_no,
            class_id,
            text: text.clone(),
        })
        .collect()
}

fn checks_for(stripped: &str) -> [(&'static str, bool); 6] {
    [
        ("FORMAT-PRINT-001", stripped.contains("println!(") || stripped.contains("eprintln!(")),
        ("FORMAT-DBG-001", stripped.contains("dbg!(")),
        (
            "FORMAT-JSON-001",
            stripped.contains("serde_json") || stripped.contains("serde_json::Value"),
        ),
        (
            "FORMAT-YAML-001",
            stripped.contains("serde_saphyr")
                || stripped.contains("saphyr::")
                || stripped.contains(" saphyr"),
        ),
        ("MAP-STRING-001", line_has_string_map(stripped)),
        ("CHANNEL-UNBOUNDED-001", has_unbounded_channel(stripped)),
    ]
}

fn has_unbounded_channel(stripped: &str) -> bool {
    stripped.contains("std::sync::mpsc::channel(")
        || stripped.contains("mpsc::channel(")
        || stripped.contains("unbounded_channel(")
        || stripped.contains("crossbeam_channel::unbounded(")
}

/// Recursively collect Rust files under `root`.
///
/// # Errors
///
/// Returns the first directory traversal error from `read_dir`.
fn rust_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    fs::read_dir(root)?.try_fold(Vec::new(), |mut out, entry| {
        let entry = entry?;
        append_rust_entry(&mut out, &entry, root)?;
        Ok(out)
    })
}

/// Append one directory entry's Rust files to the accumulator.
///
/// # Errors
///
/// Returns directory traversal errors from recursive Rust-file collection.
fn append_rust_entry(out: &mut Vec<PathBuf>, entry: &fs::DirEntry, _root: &Path) -> io::Result<()> {
    let path = entry.path();
    if path.is_dir() {
        out.extend(rust_files(&path)?);
    } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
        out.push(path);
    }
    Ok(())
}

/// Collect source files from the configured hot crates.
///
/// # Errors
///
/// Returns directory traversal errors from hot-crate source enumeration.
fn hot_sources(root: &Path) -> io::Result<Vec<PathBuf>> {
    HOT_CRATES.iter().try_fold(Vec::new(), |mut out, crate_name| {
        let src = root.join("crates").join(crate_name).join("src");
        out.extend(rust_files(&src)?);
        Ok(out)
    })
}
