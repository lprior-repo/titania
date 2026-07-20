//! v1.5 contract tests for `Lane::name` uniqueness across all 12 variants.

use titania_core::Lane;

#[test]
fn lane_name_uniqueness() {
    let lanes = [
        Lane::Fmt,
        Lane::Compile,
        Lane::Clippy,
        Lane::AstGrep,
        Lane::Dylint,
        Lane::PanicScan,
        Lane::PolicyScan,
        Lane::Test,
        Lane::Deny,
        Lane::Build,
        Lane::Kani,
        Lane::Mutants,
    ];
    let mut seen: Vec<&str> = Vec::with_capacity(lanes.len());
    for lane in &lanes {
        let name = lane.name();
        assert!(!seen.contains(&name), "duplicate lane name: {name}");
        seen.push(name);
    }
}

#[test]
fn file_stem_uniqueness() {
    let lanes = [
        Lane::Fmt,
        Lane::Compile,
        Lane::Clippy,
        Lane::AstGrep,
        Lane::Dylint,
        Lane::PanicScan,
        Lane::PolicyScan,
        Lane::Test,
        Lane::Deny,
        Lane::Build,
        Lane::Kani,
        Lane::Mutants,
    ];
    let mut seen: Vec<&str> = Vec::with_capacity(lanes.len());
    for lane in &lanes {
        let stem = lane.file_stem();
        assert!(!seen.contains(&stem), "duplicate lane file_stem: {stem}");
        seen.push(stem);
    }
}

#[test]
fn name_matches_pascal_case_serde_form() {
    for lane in [Lane::Kani, Lane::Mutants] {
        let name = lane.name();
        let json = serde_json::to_string(&lane).unwrap();
        assert_eq!(json, format!("\"{name}\""));
    }
}
