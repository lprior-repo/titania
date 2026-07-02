#![allow(clippy::excessive_nesting, reason = "Diff parsing logic requires conditional nesting for +++/--- line handling in update_current.")]

use std::collections::BTreeSet;

fn is_rust_ext(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("rs"))
}

pub fn is_test_path(path: &str) -> bool {
    if !is_rust_ext(path) {
        return false;
    }
    let is_in_tests = path.split('/').any(|segment| matches!(segment, "tests" | "benches" | "examples" | "fuzz"));
    is_in_tests || path.contains("workspace_tests") || is_module_test_path(path)
}

fn is_behavior_test_path(path: &str) -> bool {
    if !is_rust_ext(path) {
        return false;
    }
    let is_in_tests = path.split('/').any(|segment| segment == "tests");
    is_in_tests || path.contains("workspace_tests") || is_module_test_path(path)
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
    child.is_some_and(|value| !value.is_empty() && is_rust_ext(value) && !value.contains('/'))
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

pub fn scan_diff(diff: &str) -> Vec<(String, String, String)> {
    let mut state = DiffState::default();
    diff.lines().for_each(|line| state.scan_line(line));
    state.finish()
}

#[derive(Default)]
struct DiffState {
    current: String,
    removed_test_decl: Vec<(String, String)>,
    added_test_decl: Vec<(String, String)>,
    removed_exact: Vec<String>,
    added_exact: Vec<String>,
    added_weak: Vec<String>,
    findings: Vec<(String, String, String)>,
}

impl DiffState {
    fn scan_line(&mut self, line: &str) {
        if self.update_current(line) || !is_test_path(&self.current) {
            return;
        }
        if let Some(payload) = removed_payload(line) {
            self.scan_removed_if_not_fixture(payload);
        } else if let Some(payload) = added_payload(line) {
            self.scan_added_if_not_fixture(payload);
        }
    }

    fn scan_removed_if_not_fixture(&mut self, payload: &str) {
        if is_fixture_literal_line(payload) {
            return;
        }
        self.scan_removed(payload);
    }

    fn scan_added_if_not_fixture(&mut self, payload: &str) {
        if is_fixture_literal_line(payload) {
            return;
        }
        self.scan_added(payload);
    }

    fn update_current(&mut self, line: &str) -> bool {
        if let Some(path) = line.strip_prefix("+++ b/") {
            path.clone_into(&mut self.current);
            return true;
        }
        if let Some(path) = line.strip_prefix("--- a/") {
            if self.current.is_empty() {
                path.clone_into(&mut self.current);
            }
            return true;
        }
        false
    }

    fn scan_removed(&mut self, payload: &str) {
        if has_test_decl(payload) {
            self.removed_test_decl.push((self.current.clone(), payload.trim().to_owned()));
        }
        if has_exact_assertion(payload) {
            self.removed_exact.push(self.current.clone());
        }
    }

    fn scan_added(&mut self, payload: &str) {
        if has_test_decl(payload) {
            self.added_test_decl.push((self.current.clone(), payload.trim().to_owned()));
        }
        self.scan_added_behavior_flags(payload);
        if has_exact_assertion(payload) {
            self.added_exact.push(self.current.clone());
        }
        if has_weak_assertion(payload) {
            self.added_weak.push(self.current.clone());
        }
    }

    fn scan_added_behavior_flags(&mut self, payload: &str) {
        if !is_behavior_test_path(&self.current) {
            return;
        }
        if has_ignore_or_skip(payload) {
            self.findings.push((
                "IgnoredOrSkippedTest".to_owned(),
                self.current.clone(),
                payload.trim().to_owned(),
            ));
        }
        if has_compile_only(payload) {
            self.findings.push((
                "CompileOnlyReplacement".to_owned(),
                self.current.clone(),
                payload.trim().to_owned(),
            ));
        }
    }

    fn finish(mut self) -> Vec<(String, String, String)> {
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

fn removed_payload(line: &str) -> Option<&str> {
    line.strip_prefix('-').filter(|_| !line.starts_with("---"))
}

fn added_payload(line: &str) -> Option<&str> {
    line.strip_prefix('+').filter(|_| !line.starts_with("+++"))
}

fn deleted_test_declarations(
    removed: &[(String, String)],
    added: &[(String, String)],
) -> Vec<(String, String, String)> {
    let removed_count = removed.len();
    let added_count = added.len();
    if added_count >= removed_count {
        Vec::new()
    } else {
        let paths = removed.iter().map(|(path, _)| path.clone()).collect::<BTreeSet<_>>();
        paths
            .into_iter()
            .map(|path| {
                (
                    "DeletedTestDeclaration".to_owned(),
                    path,
                    format!(
                        "removed_declarations={removed_count} added_declarations={added_count}"
                    ),
                )
            })
            .collect()
    }
}

fn weakened_assertions(
    removed_exact: &[String],
    added_exact: &[String],
    added_weak: &[String],
) -> Vec<(String, String, String)> {
    let removed_count = removed_exact.len();
    let added_exact_count = added_exact.len();
    let added_weak_count = added_weak.len();
    if added_exact_count >= removed_count {
        Vec::new()
    } else {
        let paths = removed_exact.iter().cloned().collect::<BTreeSet<_>>();
        paths
            .into_iter()
            .map(|path| {
                (
                    "WeakenedAssertion".to_owned(),
                    path,
                    format!(
                        "removed_exact={removed_count} added_exact={added_exact_count} added_weak={added_weak_count}"
                    ),
                )
            })
            .collect()
    }
}
