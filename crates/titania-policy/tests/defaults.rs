//! Tests for embedded strict-ai policy defaults.
//!
//! Every test asserts an exact value from the v1 spec. The scaffold in
//! `lib.rs` returns wrong placeholders so these tests fail RED until the
//! production defaults are implemented.
//!
//! See v1-spec.md §9.7 (policy.toml schema) and §9 (the strict-ai policy).

use titania_policy::PolicyDefaults;

// ---------------------------------------------------------------------------
// Contract: v1-spec.md §9 — the strict-ai policy
// ---------------------------------------------------------------------------

#[test]
fn defaults_schema_version_is_1() {
    // v1-spec.md §9.7: schema_version = 1
    let defaults = PolicyDefaults::embedded();
    assert_eq!(defaults.schema_version, 1);
}

#[test]
fn defaults_profile_name_is_strict_ai() {
    // v1-spec.md §9.7: profile name is "strict-ai"
    let defaults = PolicyDefaults::embedded();
    assert_eq!(defaults.profile_name, "strict-ai");
}

#[test]
fn defaults_sources_cite_v1_spec_and_agents() {
    // v1-spec.md §9 + AGENTS.md are the two canonical sources.
    let defaults = PolicyDefaults::embedded();
    let sources: Vec<&str> = defaults.sources().iter().map(|s| s.as_str()).collect();
    assert!(sources.contains(&"v1-spec.md"), "sources must include 'v1-spec.md'");
    assert!(sources.contains(&"AGENTS.md"), "sources must include 'AGENTS.md'");
}

#[test]
fn defaults_core_dirs_include_crate_core_path() {
    // v1-spec.md §9.7: core_dirs = ["src/core", "src/domain", "crates/*-core/src"]
    let defaults = PolicyDefaults::embedded();
    let core_dirs: Vec<&str> = defaults.architecture.core_dirs.iter().map(|s| s.as_str()).collect();
    assert!(core_dirs.contains(&"crates/*-core/src"), "core_dirs must include 'crates/*-core/src'");
}

#[test]
fn defaults_infra_crates_are_tokio_axum_sqlx_reqwest() {
    // v1-spec.md §9.7: infra_crates = ["tokio", "axum", "sqlx", "reqwest"]
    let defaults = PolicyDefaults::embedded();
    let infra: Vec<&str> = defaults.architecture.infra_crates.iter().map(|s| s.as_str()).collect();
    for expected in ["tokio", "axum", "sqlx", "reqwest"] {
        assert!(infra.contains(&expected), "infra_crates must include '{expected}'");
    }
}

#[test]
fn defaults_no_filesystem_access_needed() {
    // The embedded defaults must load without any filesystem I/O.
    // This is enforced by the API: `embedded()` returns compile-time constants.
    let defaults = PolicyDefaults::embedded();
    assert!(defaults.no_fs_access(), "embedded() defaults must not require filesystem access");
}
