//! Coverage for v1 run-lane source discovery.

use std::path::PathBuf;

use titania_lanes::collect_rust_sources;

#[test]
fn titania_cache_sources_are_skipped() {
    let root = tempfile::tempdir().expect("create temp root");
    let src_dir = root.path().join("src");
    let cache_dir = root.path().join(".titania/cache/test/debug/build/generated/out");

    std::fs::create_dir_all(&src_dir).expect("create src dir");
    std::fs::create_dir_all(&cache_dir).expect("create cache dir");
    std::fs::write(src_dir.join("lib.rs"), "pub fn real() {}\n").expect("write source");
    std::fs::write(cache_dir.join("private.rs"), "use super::*;\n").expect("write cache source");

    let sources = collect_rust_sources(root.path()).expect("collect sources");

    assert_eq!(sources, vec![PathBuf::from("src/lib.rs")]);
}
