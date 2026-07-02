use super::{collector::collect_features, scope::is_perf_scoped_path};

#[test]
fn push_closed_feature_extracts_single_line_attribute_without_stray_paren() {
    // Regression: the slice used to end at the `)` of `)]`, leaving the
    // `]` out. `extract_names` then failed to strip the `)]` suffix and
    // the last feature name carried a stray `)`.
    let uses = collect_features("#![feature(try_blocks)]\n");
    assert_eq!(uses.len(), 1);
    let (line_no, names, _) = &uses[0];
    assert_eq!(*line_no, 1);
    assert_eq!(names, &vec!["try_blocks".to_owned()]);
}

#[test]
fn push_closed_feature_extracts_multi_feature_attribute() {
    let uses = collect_features("#![feature(allocator_api, generic_const_exprs)]\n");
    let names = &uses[0].1;
    assert_eq!(names, &vec!["allocator_api".to_owned(), "generic_const_exprs".to_owned()]);
}

#[test]
fn push_closed_feature_returns_false_when_no_close() {
    let uses = collect_features("#![feature(try_blocks\n");
    assert!(uses.is_empty());
}

#[test]
fn collect_features_handles_multi_line_attribute() {
    let content = "#![feature(\n    allocator_api,\n    generic_const_exprs\n)]\n";
    let uses = collect_features(content);
    assert_eq!(uses.len(), 1);
    let (_line, names, _report) = &uses[0];
    assert_eq!(names, &vec!["allocator_api".to_owned(), "generic_const_exprs".to_owned()]);
}

#[test]
fn collect_features_handles_single_line_attribute() {
    let content = "#![feature(try_blocks)]\n";
    let uses = collect_features(content);
    assert_eq!(uses.len(), 1);
    let (_line, names, _report) = &uses[0];
    assert_eq!(names, &vec!["try_blocks".to_owned()]);
}

#[test]
fn collect_features_ignores_non_attribute_text() {
    // The `#![feature(` prefix is required; this line just mentions
    // `feature` in a comment.
    let content = "// #![feature(specialization)]\nlet x = 1;\n";
    let uses = collect_features(content);
    assert!(uses.is_empty());
}

#[test]
fn perf_scoped_path_recognises_crate_perf_and_generated() {
    assert!(is_perf_scoped_path("crates/foo/src/perf/widget.rs"));
    assert!(is_perf_scoped_path("crates/foo/src/generated/widget.rs"));
    assert!(is_perf_scoped_path("benches/bench.rs"));
    // Outside any perf scope.
    assert!(!is_perf_scoped_path("crates/foo/src/lib.rs"));
}
