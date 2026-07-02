use std::collections::BTreeSet;

use super::{IntegrityFinding, TestDeclaration};

pub(super) fn is_test_path(path: &str) -> bool {
    if !is_rust_ext(path) {
        return false;
    }
    let segments: Vec<&str> = path.split('/').collect();
    let is_in_tests = segments
        .iter()
        .any(|segment| matches!(*segment, "tests" | "benches" | "examples" | "fuzz"));
    is_in_tests || path.contains("workspace_tests") || is_module_test_path(path)
}

fn is_behavior_test_path(path: &str) -> bool {
    if !is_rust_ext(path) {
        return false;
    }
    let is_in_tests = path.split('/').any(|segment| matches!(segment, "tests"));
    is_in_tests || path.contains("workspace_tests") || is_module_test_path(path)
}

fn is_rust_ext(path: &str) -> bool {
    std::path::Path::new(path).extension().is_some_and(|ext| ext.eq_ignore_ascii_case("rs"))
}

fn is_module_test_path(path: &str) -> bool {
    let Some(after_src) = path_after_src(path) else {
        return false;
    };
    is_src_tests_rs_path(after_src) || is_src_tests_child_path(after_src)
}

fn path_after_src(path: &str) -> Option<&str> {
    path.strip_prefix("src/")
        .or_else(|| path.split_once("/src/").map(|(_prefix, after_src)| after_src))
}

fn is_src_tests_rs_path(after_src: &str) -> bool {
    after_src == "tests.rs" || after_src.ends_with("/tests.rs")
}

fn is_src_tests_child_path(after_src: &str) -> bool {
    let child = after_src
        .strip_prefix("tests/")
        .or_else(|| after_src.rsplit_once("/tests/").map(|(_before, child)| child));
    child.is_some_and(|value| {
        !value.is_empty()
            && std::path::Path::new(value)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("rs"))
            && !value.contains('/')
    })
}

fn has_exact_assertion(text: &str) -> bool {
    [
        "assert_eq!(",
        "assert_ne!(",
        "assert_matches!(",
        "assert_json_",
        "insta::assert_",
        "snapshot!(",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

fn has_weak_assertion(text: &str) -> bool {
    text.contains("assert!(")
        && [".is_ok(", ".is_err(", ".is_some(", ".is_none(", ".is_empty("]
            .iter()
            .any(|needle| text.contains(needle))
}

fn has_test_decl(text: &str) -> bool {
    text.contains("#[test")
        || text.contains("#[tokio::test")
        || text.contains("fn test_")
        || text.contains("_test(")
}

fn has_ignore_or_skip(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("#[ignore")
        || (lower.contains("cfg_attr") && lower.contains("ignore"))
        || lower.contains("return;")
        || lower.contains(" skipped")
        || lower.contains(" skip")
        || lower.contains("ignored")
}

fn is_fixture_literal_line(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with('"') || trimmed.starts_with("r#")
}

fn has_compile_only(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    ["no_run", "compile_only", "compile-only", "smoke only", "compile smoke"]
        .iter()
        .any(|needle| lower.contains(needle))
}

pub(super) fn scan_diff(diff: &str) -> Vec<IntegrityFinding> {
    let state = diff.lines().fold(DiffState::default(), |mut state, line| {
        state.scan_line(line);
        state
    });
    state.finish()
}

#[derive(Default)]
struct DiffState {
    current: String,
    removed_test_decl: Vec<TestDeclaration>,
    added_test_decl: Vec<TestDeclaration>,
    removed_exact: Vec<String>,
    added_exact: Vec<String>,
    added_weak: Vec<String>,
    findings: Vec<IntegrityFinding>,
}

impl DiffState {
    fn scan_line(&mut self, line: &str) {
        scan_diff_line(self, line);
    }

    fn finish(mut self) -> Vec<IntegrityFinding> {
        self.findings
            .extend(deleted_test_declarations(&self.removed_test_decl, &self.added_test_decl));
        self.findings.extend(weakened_assertions(
            &self.removed_exact,
            &self.added_exact,
            &self.added_weak,
        ));
        self.findings
    }
}

fn scan_diff_line(state: &mut DiffState, line: &str) {
    if update_current(state, line) || !is_test_path(&state.current) {
        return;
    }
    if let Some(payload) = removed_payload(line).filter(|payload| !is_fixture_literal_line(payload))
    {
        scan_removed(state, payload);
        return;
    }
    if let Some(payload) = added_payload(line).filter(|payload| !is_fixture_literal_line(payload)) {
        scan_added(state, payload);
    }
}

fn update_current(state: &mut DiffState, line: &str) -> bool {
    if let Some(path) = line.strip_prefix("+++ b/") {
        path.clone_into(&mut state.current);
        return true;
    }
    if let Some(path) = line.strip_prefix("--- a/") {
        capture_old_path(state, path);
        return true;
    }
    false
}

fn capture_old_path(state: &mut DiffState, path: &str) {
    if state.current.is_empty() {
        path.clone_into(&mut state.current);
    }
}

fn scan_removed(state: &mut DiffState, payload: &str) {
    if has_test_decl(payload) {
        state.removed_test_decl.push((state.current.clone(), payload.trim().to_owned()));
    }
    if has_exact_assertion(payload) {
        state.removed_exact.push(state.current.clone());
    }
}

fn scan_added(state: &mut DiffState, payload: &str) {
    if has_test_decl(payload) {
        state.added_test_decl.push((state.current.clone(), payload.trim().to_owned()));
    }
    scan_added_behavior_flags(state, payload);
    if has_exact_assertion(payload) {
        state.added_exact.push(state.current.clone());
    }
    if has_weak_assertion(payload) {
        state.added_weak.push(state.current.clone());
    }
}

fn scan_added_behavior_flags(state: &mut DiffState, payload: &str) {
    if !is_behavior_test_path(&state.current) {
        return;
    }
    if has_ignore_or_skip(payload) {
        state.findings.push((
            "IgnoredOrSkippedTest".to_owned(),
            state.current.clone(),
            payload.trim().to_owned(),
        ));
    }
    if has_compile_only(payload) {
        state.findings.push((
            "CompileOnlyReplacement".to_owned(),
            state.current.clone(),
            payload.trim().to_owned(),
        ));
    }
}

fn removed_payload(line: &str) -> Option<&str> {
    line.strip_prefix('-').filter(|_| !line.starts_with("---"))
}

fn added_payload(line: &str) -> Option<&str> {
    line.strip_prefix('+').filter(|_| !line.starts_with("+++"))
}

fn deleted_test_declarations(
    removed: &[TestDeclaration],
    added: &[TestDeclaration],
) -> Vec<IntegrityFinding> {
    let removed_count = removed.len();
    let added_count = added.len();
    if added_count >= removed_count {
        Vec::new()
    } else {
        let paths = removed.iter().map(|(path, _)| path.clone()).collect::<BTreeSet<_>>();
        paths
            .into_iter()
            .map(|path| deleted_test_declaration_finding(path, removed_count, added_count))
            .collect()
    }
}

fn deleted_test_declaration_finding(
    path: String,
    removed_count: usize,
    added_count: usize,
) -> IntegrityFinding {
    (
        "DeletedTestDeclaration".to_owned(),
        path,
        format!("removed_declarations={removed_count} added_declarations={added_count}"),
    )
}

fn weakened_assertions(
    removed_exact: &[String],
    added_exact: &[String],
    added_weak: &[String],
) -> Vec<IntegrityFinding> {
    let counts = AssertionCounts {
        removed: removed_exact.len(),
        added_exact: added_exact.len(),
        added_weak: added_weak.len(),
    };
    if counts.added_exact >= counts.removed {
        Vec::new()
    } else {
        let paths = removed_exact.iter().cloned().collect::<BTreeSet<_>>();
        paths.into_iter().map(|path| weakened_assertion_finding(path, counts)).collect()
    }
}

#[derive(Clone, Copy)]
struct AssertionCounts {
    removed: usize,
    added_exact: usize,
    added_weak: usize,
}

fn weakened_assertion_finding(path: String, counts: AssertionCounts) -> IntegrityFinding {
    (
        "WeakenedAssertion".to_owned(),
        path,
        format!(
            "removed_exact={} added_exact={} added_weak={}",
            counts.removed, counts.added_exact, counts.added_weak
        ),
    )
}
