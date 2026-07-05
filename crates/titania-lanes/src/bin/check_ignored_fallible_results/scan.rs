use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use titania_lanes::{Finding, LaneReport, RuleId, RuleIdError, helpers::line_no_from_idx};

use crate::source::SourceLine;

/// Rule identifiers used for ignored fallible-result findings.
#[derive(Debug)]
pub struct DiscardRules {
    bare_call: RuleId,
    assignment: RuleId,
    ok_err: RuleId,
    match_arm: RuleId,
    drop_call: RuleId,
}

impl DiscardRules {
    /// Build rule identifiers for discard findings.
    ///
    /// # Errors
    ///
    /// Returns the invalid rule-id error if one of the configured discard
    /// rule ids violates the shared rule-id format.
    pub fn new() -> Result<Self, RuleIdError> {
        Ok(Self {
            bare_call: RuleId::new("DISCARD_001")?,
            assignment: RuleId::new("DISCARD_002")?,
            ok_err: RuleId::new("DISCARD_003")?,
            match_arm: RuleId::new("DISCARD_004")?,
            drop_call: RuleId::new("DISCARD_005")?,
        })
    }
}

/// Scan source roots for ignored fallible-result patterns.
pub fn scan(
    root: &Path,
    allow: &BTreeMap<String, String>,
    rules: &DiscardRules,
    report: &mut LaneReport,
) {
    scan_roots(root).into_iter().fold((), |(), file_root| {
        scan_dir(&file_root, root, allow, rules, report);
    });
}

fn scan_roots(root: &Path) -> Vec<PathBuf> {
    let crates = crate_src_roots(root);
    let xtask = xtask_root(root);
    crates.into_iter().chain(xtask).collect()
}

fn crate_src_roots(root: &Path) -> Vec<PathBuf> {
    let crates_dir = root.join("crates");
    let Ok(read) = std::fs::read_dir(&crates_dir) else {
        return Vec::new();
    };
    read.flatten().filter_map(|entry| crate_src_root(&entry)).collect()
}

fn crate_src_root(entry: &std::fs::DirEntry) -> Option<PathBuf> {
    let path = entry.path();
    let src = path.join("src");
    (path.is_dir() && src.is_dir()).then_some(src)
}

fn xtask_root(root: &Path) -> Option<PathBuf> {
    let xtask = root.join("xtask/src");
    xtask.is_dir().then_some(xtask)
}

fn should_skip(rel: &str) -> bool {
    rel.contains("kani_")
        || rel.contains("workspace_tests")
        || rel.ends_with("/tests.rs")
        || rel.ends_with("_tests.rs")
        || rel.contains("/test_harness.rs")
        || rel.contains("/tests/")
        || rel.contains("/impl_tests/")
        || rel.contains("/lifecycle_tests/")
}

fn scan_dir(
    dir: &Path,
    root: &Path,
    allow: &BTreeMap<String, String>,
    rules: &DiscardRules,
    report: &mut LaneReport,
) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    read.flatten().fold((), |(), entry| scan_entry(&entry.path(), root, allow, rules, report));
}

fn scan_entry(
    path: &Path,
    root: &Path,
    allow: &BTreeMap<String, String>,
    rules: &DiscardRules,
    report: &mut LaneReport,
) {
    if path.is_dir() {
        scan_dir(path, root, allow, rules, report);
    } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
        scan_rust_file(path, root, allow, rules, report);
    }
}

fn scan_rust_file(
    file: &Path,
    root: &Path,
    allow: &BTreeMap<String, String>,
    rules: &DiscardRules,
    report: &mut LaneReport,
) {
    let rel = rel_str(root, file);
    if !should_skip(&rel) {
        scan_file(file, &rel, allow, rules, report);
    }
}

fn rel_str(root: &Path, path: &Path) -> String {
    match path.strip_prefix(root) {
        Ok(relative) => relative.to_string_lossy().replace('\\', "/"),
        Err(_error) => path.to_string_lossy().into_owned(),
    }
}

fn scan_file(
    file: &Path,
    rel: &str,
    allow: &BTreeMap<String, String>,
    rules: &DiscardRules,
    report: &mut LaneReport,
) {
    let Ok(text) = std::fs::read_to_string(file) else {
        return;
    };
    let mut block_comment = false;
    let mut context = ScanContext { rel, allow, rules, report };
    text.lines().enumerate().fold((), |(), (idx, line)| {
        scan_line(&mut context, line_no_from_idx(idx), line, &mut block_comment);
    });
}

struct ScanContext<'a> {
    rel: &'a str,
    allow: &'a BTreeMap<String, String>,
    rules: &'a DiscardRules,
    report: &'a mut LaneReport,
}

fn scan_line(context: &mut ScanContext<'_>, line_no: u32, raw: &str, block_comment: &mut bool) {
    let source_line = SourceLine::parse(raw, block_comment);
    if let Some(class_id) = classify_line(&source_line, context.rules) {
        push_unless_allowed(context, line_no, raw, class_id);
    }
}

fn push_unless_allowed(context: &mut ScanContext<'_>, line_no: u32, raw: &str, class_id: &RuleId) {
    let key = format!("{}|{class_id}", context.rel);
    if !context.allow.contains_key(&key) {
        context.report.push(Finding::new(
            class_id.clone(),
            context.rel,
            line_no,
            format!("discarded fallible: {}", raw.trim()),
        ));
    }
}

fn classify_line<'rules>(line: &SourceLine, rules: &'rules DiscardRules) -> Option<&'rules RuleId> {
    let trimmed = line.code();
    if is_ignored_line(line, trimmed) {
        return None;
    }
    discard_patterns(trimmed, rules)
        .into_iter()
        .find_map(|(class_id, matched)| matched.then_some(class_id))
}

fn is_ignored_line(line: &SourceLine, trimmed: &str) -> bool {
    trimmed.is_empty()
        || line.is_signature()
        || trimmed.starts_with("use ")
        || trimmed.starts_with("return ")
        || !line.is_code_expression()
}

fn discard_patterns<'rules>(
    trimmed: &str,
    rules: &'rules DiscardRules,
) -> [(&'rules RuleId, bool); 5] {
    [
        (&rules.assignment, discarded_assignment(trimmed)),
        (&rules.ok_err, discarded_ok_err(trimmed)),
        (&rules.match_arm, discarded_match_arm(trimmed)),
        (&rules.drop_call, discarded_drop(trimmed)),
        (&rules.bare_call, discarded_bare_call(trimmed)),
    ]
}

fn discarded_assignment(trimmed: &str) -> bool {
    (trimmed.starts_with("let _ =") || trimmed.starts_with("let _="))
        && contains_fallible_signal(trimmed)
}

fn discarded_ok_err(trimmed: &str) -> bool {
    (trimmed.ends_with(".ok();") || trimmed.ends_with(".err();"))
        && contains_fallible_signal(trimmed)
}

fn discarded_match_arm(trimmed: &str) -> bool {
    trimmed.contains("Ok(()) | Err(_) => {}")
        || trimmed.contains("Ok(())|Err(_)=>{}")
        || trimmed.contains("Err(_) => {}")
}

fn discarded_drop(trimmed: &str) -> bool {
    trimmed.contains("drop(") && contains_fallible_signal(trimmed)
}

fn discarded_bare_call(trimmed: &str) -> bool {
    trimmed.ends_with(';')
        && !trimmed.contains('=')
        && !trimmed.contains('?')
        && !trimmed.contains('|')
        && !trimmed.contains("assert")
        && !trimmed.contains("expect(")
        && !trimmed.contains("unwrap")
        && !trimmed.contains(".push(")
        && !trimmed.contains(".pop(")
        && contains_fallible_signal(trimmed)
}

fn contains_fallible_signal(trimmed: &str) -> bool {
    [
        "fallible",
        "try_",
        "write_",
        "send(",
        "recv(",
        "cancel",
        "persist",
        "commit",
        "remove_",
        "create_",
        "open_",
        "save_",
        "read_to_",
        "from_bytes",
        "to_allocvec",
        "try_from_parts",
    ]
    .iter()
    .any(|needle| trimmed.contains(needle))
}
