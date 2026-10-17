use titania_policy::PolicyDefaults;

#[test]
fn defaults_schema_version_is_1() {
    let defaults = PolicyDefaults::embedded();
    assert_eq!(defaults.schema_version, 1);
}

#[test]
fn defaults_profile_name_is_strict_ai() {
    let defaults = PolicyDefaults::embedded();
    assert_eq!(defaults.profile_name, "strict-ai");
}

#[test]
fn defaults_sources_cite_v1_spec_and_agents() {
    let defaults = PolicyDefaults::embedded();
    let sources: Vec<&str> = defaults.sources().iter().map(|s| s.as_str()).collect();
    assert!(sources.contains(&"v1-spec.md"), "sources must include 'v1-spec.md'");
    assert!(sources.contains(&"AGENTS.md"), "sources must include 'AGENTS.md'");
}

#[test]
fn defaults_core_dirs_include_crate_core_path() {
    let defaults = PolicyDefaults::embedded();
    let core_dirs: Vec<&str> = defaults.architecture.core_dirs.iter().map(|s| s.as_str()).collect();
    assert!(core_dirs.contains(&"crates/*-core/src"), "core_dirs must include 'crates/*-core/src'");
}

#[test]
fn defaults_infra_crates_are_tokio_axum_sqlx_reqwest() {
    let defaults = PolicyDefaults::embedded();
    let infra: Vec<&str> = defaults.architecture.infra_crates.iter().map(|s| s.as_str()).collect();
    for expected in ["tokio", "axum", "sqlx", "reqwest"] {
        assert!(infra.contains(&expected), "infra_crates must include '{expected}'");
    }
}

#[test]
fn defaults_no_filesystem_access_needed() {
    let defaults = PolicyDefaults::embedded();
    assert!(defaults.no_fs_access(), "embedded() defaults must not require filesystem access");
}
