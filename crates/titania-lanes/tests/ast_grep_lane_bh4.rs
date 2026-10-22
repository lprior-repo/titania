use std::{
    error::Error,
    path::{Path, PathBuf},
};

use titania_core::LaneOutcome;

type TestResult = Result<(), Box<dyn Error>>;

fn fixture_root(dir: &str, name: &str) -> PathBuf {
    let base = env!("CARGO_MANIFEST_DIR");
    Path::new(base).join("tests").join("fixtures").join(dir).join(name)
}

fn run_architecture_fixture(name: &str) -> Vec<titania_core::Finding> {
    let rules_yaml = [include_str!("../rules/architecture.yml")];
    let paths = vec![fixture_root("ast_grep", &format!("crates/example-core/src/{name}"))];
    match titania_lanes::ast_grep_lane::run(&rules_yaml, &paths, &[]) {
        Ok(LaneOutcome::Findings { findings }) => findings.into_vec(),
        Ok(LaneOutcome::Clean { .. }) => Vec::new(),
        Ok(other) => panic!("unexpected lane outcome for {name}: {other:?}"),
        Err(err) => panic!("lane error for {name}: {err}"),
    }
}

fn span_line_col(finding: &titania_core::Finding) -> (u64, u64) {
    let value = serde_json::to_value(finding.location()).expect("location must serialize");
    let span = value.get("Span").expect("ast-grep findings must use Location::Span");
    let line = span.get("line_start").and_then(serde_json::Value::as_u64);
    let col = span.get("col_start").and_then(serde_json::Value::as_u64);
    match (line, col) {
        (Some(line), Some(col)) => (line, col),
        _ => panic!("ast-grep finding span must carry line_start and col_start"),
    }
}

fn fs_spans(name: &str) -> Vec<(u64, u64)> {
    run_architecture_fixture(name)
        .iter()
        .filter(|finding| finding.rule_id().as_str() == "ARCHITECTURE_IMPORT_CORE_FS")
        .map(span_line_col)
        .collect()
}

#[test]
fn bh4_whitespace_newline_use_path_fires_at_the_fs_segment() -> TestResult {
    let spans = fs_spans("architecture_import_core_fs_whitespace_newline_violation.rs");
    assert_eq!(
        spans,
        [(2, 9)],
        "use\\n std::fs; must emit exactly one FS finding at the fs segment"
    );
    Ok(())
}

#[test]
fn bh4_whitespace_around_colons_use_path_fires_at_the_fs_segment() -> TestResult {
    let spans = fs_spans("architecture_import_core_fs_whitespace_colons_violation.rs");
    assert_eq!(
        spans,
        [(1, 11)],
        "use std :: fs; must emit exactly one FS finding at the fs segment"
    );
    Ok(())
}

#[test]
fn bh4_whitespace_grouped_use_path_fires_per_member() -> TestResult {
    let spans = fs_spans("architecture_import_core_fs_whitespace_grouped_violation.rs");
    assert_eq!(
        spans,
        [(1, 13), (1, 18)],
        "use std :: {{ env, net }}; must emit one FS finding per member at its own span"
    );
    Ok(())
}

#[test]
fn bh4_user_module_named_std_stays_clean() -> TestResult {
    let findings = run_architecture_fixture("allowed_core_user_std_module.rs");
    assert!(
        findings.is_empty(),
        "foo::std::fs::read() through a user module named std must stay clean: {findings:?}"
    );
    Ok(())
}
