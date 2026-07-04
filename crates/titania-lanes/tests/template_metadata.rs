//! Integration tests for `cargo generate titania/template` metadata and policy-Rust config copy fidelity.
//!
//! Two acceptance tests:
//! - `template_metadata` — reads `cargo-generate.toml`, `README.md`, and `Cargo.toml` from the
//!   template directory, parses TOML with `toml_edit`, and asserts meaningful metadata, placeholders,
//!   and workspace fields exist.
//! - `template_policy_rust_configs` — compares every policy/Rust config file in the template against
//!   the root source configs where exact copy is expected, and asserts template `Cargo.toml` contains
//!   the required strict-ai workspace lint keys and metadata.
//!
//! These tests are intentionally written against the template *as it should exist*; on branches
//! where `titania/template/` has not yet been created they produce RED (file-not-found) evidence.

use anyhow::{Context, Result};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use toml_edit::{DocumentMut, Item, Table};

// ── helpers ──────────────────────────────────────────────────────────────────

/// Resolve the repository root from the crate's manifest directory.
fn workspace_root() -> Result<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    // CARGO_MANIFEST_DIR = …/crates/titania-lanes
    // .parent()        → …/crates/
    // .parent()        → …/e0r1-receipt-schema/  (workspace root)
    let root = manifest
        .parent() // crates/
        .and_then(Path::parent) // workspace root (e.g. e0r1-receipt-schema/)
        .map(Path::to_path_buf)
        .context("cannot derive workspace root from CARGO_MANIFEST_DIR")?;
    Ok(root)
}

/// Read the template directory path under the workspace root.
fn template_root() -> Result<PathBuf> {
    workspace_root().map(|root| root.join("titania").join("template"))
}

/// Read a file as a UTF-8 string.
fn read_file(path: &Path, label: &str) -> Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("read {label}: {}", path.display()))
}

/// Parse TOML text into a `toml_edit::DocumentMut`.
fn parse_toml(text: &str, label: &str) -> Result<DocumentMut> {
    text.parse::<DocumentMut>().with_context(|| format!("parse {label} as TOML"))
}
/// Look up a nested string scalar, e.g. `[workspace.package].edition`.
fn nested_str<'a>(doc: &'a DocumentMut, section: &str, key: &str) -> Result<Option<&'a str>> {
    Ok(nested_table(doc, section)?.get(key).and_then(Item::as_str))
}

fn nested_table<'a>(doc: &'a DocumentMut, section: &str) -> Result<&'a Table> {
    let mut parts = section.split('.');
    let first = parts.next().context("section path must not be empty")?;
    let mut item = doc.get(first).with_context(|| format!("expected [{section}] table"))?;
    for part in parts {
        item = item
            .as_table()
            .and_then(|table| table.get(part))
            .with_context(|| format!("expected [{section}] table"))?;
    }
    item.as_table().with_context(|| format!("expected [{section}] table"))
}

/// Compare two file contents byte-for-byte (ignoring leading/trailing whitespace).
fn files_match(template_path: &Path, root_path: &Path, label: &str) -> Result<()> {
    let template_text = read_file(template_path, &format!("{label} (template)"))?;
    let root_text = read_file(root_path, &format!("{label} (root)"))?;
    let trimmed_t = template_text.trim();
    let trimmed_r = root_text.trim();
    if trimmed_t != trimmed_r {
        anyhow::bail!(
            "{label} content mismatch: template != root\n\
             template ({len_t} bytes)\n  first 80: {sample_t}\nroot     ({len_r} bytes)\n  first 80: {sample_r}",
            len_t = trimmed_t.len(),
            len_r = trimmed_r.len(),
            sample_t = &trimmed_t.chars().take(80).collect::<String>(),
            sample_r = &trimmed_r.chars().take(80).collect::<String>(),
        );
    }
    Ok(())
}

/// Assert that a set of keys is present in a BTreeMap, reporting the missing ones.
fn assert_keys_present(map: &BTreeMap<&str, &str>, required: &[&str], section: &str) -> Result<()> {
    let mut missing = Vec::new();
    for key in required {
        if !map.contains_key(*key) {
            missing.push(*key);
        }
    }
    if !missing.is_empty() {
        anyhow::bail!("[{section}] missing required keys: {missing:?}");
    }
    Ok(())
}

// ── test 1: template_metadata ───────────────────────────────────────────────

#[test]
fn template_metadata() -> Result<()> {
    let tmpl = template_root()?;

    // 1. cargo-generate.toml — must exist, parse as TOML, have [package] with name and description
    let cg_path = tmpl.join("cargo-generate.toml");
    let cg_text = read_file(&cg_path, "cargo-generate.toml")?;
    let cg_doc = parse_toml(&cg_text, "cargo-generate.toml")?;

    // Top-level metadata keys that cargo-generate expects
    let required_cg_keys = ["name", "description", "authors", "license", "keywords"];
    let cg_top_keys: BTreeMap<&str, &str> =
        cg_doc.iter().map(|(k, v)| (k, v.as_str().unwrap_or_default())).collect();
    assert_keys_present(&cg_top_keys, &required_cg_keys, "cargo-generate.toml")?;

    // [package].name inside cargo-generate.toml is also used as template dir name
    let pkg_name = cg_doc
        .get("package")
        .and_then(|i| i.as_table())
        .and_then(|t| t.get("name"))
        .and_then(Item::as_str);
    assert!(
        pkg_name.is_some(),
        "cargo-generate.toml [package] must contain name (the template directory name)"
    );

    // 2. README.md — must exist and be non-empty
    let readme_path = tmpl.join("README.md");
    let readme_text = read_file(&readme_path, "README.md")?;
    let readme_trimmed = readme_text.trim();
    assert!(!readme_trimmed.is_empty(), "README.md must not be empty");
    // Should mention the template identity
    assert!(
        readme_text.contains("titania/template"),
        "README.md should reference titania/template"
    );

    // 3. Cargo.toml — must exist, parse as TOML, have [workspace] section
    let cargo_path = tmpl.join("Cargo.toml");
    let cargo_text = read_file(&cargo_path, "template Cargo.toml")?;
    let cargo_doc = parse_toml(&cargo_text, "template Cargo.toml")?;

    assert!(
        cargo_doc.get("workspace").is_some(),
        "template Cargo.toml must contain [workspace] section"
    );

    // [workspace.package] should have at least edition and rust-version
    let pkg_edition = nested_str(&cargo_doc, "workspace.package", "edition")?;
    let pkg_rust_version = nested_str(&cargo_doc, "workspace.package", "rust-version")?;
    assert!(pkg_edition.is_some(), "[workspace.package] must contain edition");
    assert!(pkg_rust_version.is_some(), "[workspace.package] must contain rust-version");
    if let Some(ed) = pkg_edition {
        assert_eq!(ed, "2024", "workspace.package.edition must be 2024");
    }

    // ── 4. v1-spec §14 required files: exist in template and listed in [template].include ──
    let required_files = [
        ".moon/workspace.yml",
        ".moon/toolchains.yml",
        ".moon/tasks/all.yml",
        ".titania/profiles/strict-ai/policy.toml",
        ".titania/profiles/strict-ai/exceptions.toml",
        ".cargo/config.toml",
        "clippy.toml",
        "rustfmt.toml",
        "deny.toml",
        "rust-toolchain.toml",
        "Cargo.toml",
    ];

    // Check each §14 file exists under the template directory
    for rel in &required_files {
        let fpath = tmpl.join(rel);
        drop(read_file(&fpath, &format!("§14 file {rel}"))?);
    }

    // Parse the [template].include array and assert every §14 file is listed
    let template_section = cg_doc
        .get("template")
        .and_then(|i| i.as_table())
        .context("[template] section missing in cargo-generate.toml")?;
    let include_array = template_section
        .get("include")
        .and_then(Item::as_array)
        .context("[template].include must be an array")?;
    let included: Vec<&str> = include_array.iter().filter_map(|v| v.as_str()).collect();

    for rel in &required_files {
        assert!(
            included.contains(&rel),
            "§14 file {rel} exists in template but is missing from [template].include in cargo-generate.toml"
        );
    }

    Ok(())
}

// ── test 2: template_policy_rust_configs ─────────────────────────────────────

#[test]
fn template_policy_rust_configs() -> Result<()> {
    let tmpl = template_root()?;

    // ── 2a. Exact-copy configs ──────────────────────────────────────────────
    // These files should be byte-for-byte identical between the template and the root workspace.
    let copy_pairs: [(&str, &str); 6] = [
        (".titania/profiles/strict-ai/policy.toml", ".titania/profiles/strict-ai/policy.toml"),
        (
            ".titania/profiles/strict-ai/exceptions.toml",
            ".titania/profiles/strict-ai/exceptions.toml",
        ),
        ("clippy.toml", "clippy.toml"),
        ("rustfmt.toml", "rustfmt.toml"),
        ("deny.toml", "deny.toml"),
        ("rust-toolchain.toml", "rust-toolchain.toml"),
    ];

    for (template_rel, root_rel) in &copy_pairs {
        let template_path = tmpl.join(template_rel);
        let root_path = workspace_root()?.join(root_rel);
        files_match(&template_path, &root_path, root_rel)?;
    }

    let cargo_config_text =
        read_file(&tmpl.join(".cargo").join("config.toml"), ".cargo/config.toml (template)")?;
    let cargo_config_doc = parse_toml(&cargo_config_text, ".cargo/config.toml")?;
    let rustc_wrapper = nested_str(&cargo_config_doc, "build", "rustc-wrapper")?;
    assert_eq!(
        rustc_wrapper,
        Some("sccache"),
        "template .cargo/config.toml must configure the sccache rustc wrapper"
    );
    assert!(
        cargo_config_text.contains("non_exhaustive_omitted_patterns"),
        "template .cargo/config.toml must document why the unstable §9.1 Rust lint is toolchain-blocked"
    );

    // ── 2b. Template Cargo.toml must carry strict-ai workspace lint keys ────
    let template_cargo_text = read_file(&tmpl.join("Cargo.toml"), "template Cargo.toml")?;
    let template_cargo_doc = parse_toml(&template_cargo_text, "template Cargo.toml")?;

    // [workspace.lints.rust] must exist and contain these non-negotiable lints
    let required_rust_lints = [
        "unsafe_code",
        "unused_must_use",
        "unreachable_pub",
        "missing_docs",
        "unsafe_op_in_unsafe_fn",
        "rust_2018_idioms",
    ];

    // [workspace.lints.clippy] must exist and contain these non-negotiable lints
    let required_clippy_lints = [
        "all",
        "cargo",
        "pedantic",
        "nursery",
        "unwrap_used",
        "expect_used",
        "panic",
        "todo",
        "unimplemented",
        "indexing_slicing",
        "string_slice",
        "disallowed_methods",
        "disallowed_macros",
        "disallowed_types",
        "missing_errors_doc",
        "multiple_crate_versions",
    ];

    // [workspace.metadata.titania] must have strict_ai = true
    let metadata_section = template_cargo_doc
        .get("workspace")
        .and_then(|i| i.as_table())
        .context("[workspace] section missing in template Cargo.toml")?;

    let titania_metadata = metadata_section
        .get("metadata")
        .and_then(Item::as_table)
        .and_then(|metadata| metadata.get("titania"))
        .and_then(Item::as_table)
        .context("[workspace.metadata.titania] section missing")?;

    let strict_ai = titania_metadata
        .get("strict_ai")
        .context("[workspace.metadata.titania.strict_ai] key missing")?;
    let strict_ai_bool = strict_ai
        .as_bool()
        .context("[workspace.metadata.titania.strict_ai] must be boolean true")?;
    assert!(strict_ai_bool, "[workspace.metadata.titania.strict_ai] must be true");

    // Parse [workspace.lints.rust] and [workspace.lints.clippy] as tables and check keys exist
    let lints_table = metadata_section
        .get("lints")
        .and_then(|i| i.as_table())
        .context("[workspace.lints] section missing")?;

    let rust_lints = lints_table
        .get("rust")
        .and_then(|i| i.as_table())
        .context("[workspace.lints.rust] section missing")?;
    let rust_lint_keys: BTreeMap<&str, &Item> = rust_lints.iter().collect();
    for key in &required_rust_lints {
        assert!(
            rust_lint_keys.contains_key(*key),
            "[workspace.lints.rust] missing required lint key: {key}"
        );
    }

    let clippy_lints = lints_table
        .get("clippy")
        .and_then(|i| i.as_table())
        .context("[workspace.lints.clippy] section missing")?;
    let clippy_lint_keys: BTreeMap<&str, &Item> = clippy_lints.iter().collect();
    for key in &required_clippy_lints {
        assert!(
            clippy_lint_keys.contains_key(*key),
            "[workspace.lints.clippy] missing required lint key: {key}"
        );
    }

    // ── 2c. v1-spec §9.1 stable keys layered into the strict lint table ──────
    // The unstable Rust `non_exhaustive_omitted_patterns` key is intentionally
    // documented in .cargo/config.toml instead of enabled because the pinned
    // nightly rejects it through workspace lints.
    assert!(
        clippy_lint_keys.contains_key("unwrap_or_default"),
        "[workspace.lints.clippy] missing §9.1 key: unwrap_or_default"
    );
    assert!(
        clippy_lint_keys.contains_key("exit"),
        "[workspace.lints.clippy] missing §9.1 key: exit"
    );
    assert!(
        clippy_lint_keys.contains_key("default_numeric_fallback"),
        "[workspace.lints.clippy] missing §9.1 key: default_numeric_fallback"
    );

    // ── 2d. [workspace.package] must match root ─────────────────────────────
    let ws_pkg = metadata_section
        .get("package")
        .and_then(|i| i.as_table())
        .context("[workspace.package] section missing")?;

    let expected_pkg_keys = ["edition", "rust-version", "license", "authors"];
    for key in &expected_pkg_keys {
        assert!(ws_pkg.contains_key(*key), "[workspace.package] must contain {key}");
    }

    Ok(())
}

// ── test 3: template_moon_configs — §13 task DAG ─────────────────────────────

/// Validates that `titania/template/.moon/tasks/all.yml` implements the
/// v1-spec §13 task DAG: required task names, commands, selected deps,
/// CARGO_TARGET_DIR env values, release output, and absence of repo-specific
/// legacy crate paths.
///
/// The YAML parser is not in dev-dependencies; assertions are text-based but
/// each line is checked at the semantic level that matters for the DAG contract.
#[test]
fn template_moon_configs() -> Result<()> {
    let tmpl = template_root()?;
    let tasks_path = tmpl.join(".moon").join("tasks").join("all.yml");
    let text = read_file(&tasks_path, "tasks/all.yml")?;

    // ── 3a. File groups and required task names under the Moon `tasks:` map ──
    assert!(
        text.starts_with("fileGroups:\n"),
        "tasks/all.yml must start with top-level Moon fileGroups used by @globs(...)"
    );
    for group in &["sources", "tests"] {
        assert!(
            text.contains(&format!("  {group}:\n")),
            "tasks/all.yml fileGroups must define {group}"
        );
    }
    assert!(text.contains("\ntasks:\n"), "tasks/all.yml must define the top-level Moon tasks map");
    let required_tasks = [
        "titania-fmt",
        "titania-compile",
        "titania-clippy",
        "titania-ast-grep",
        "titania-dylint",
        "titania-panic-scan",
        "titania-policy-scan",
        "titania-test",
        "titania-deny",
        "titania-build",
        "gate-edit",
        "gate-prepush",
        "gate-release",
    ];
    for task in &required_tasks {
        assert!(
            text.contains(&format!("  {task}:")),
            "task {task} must be an entry in the top-level Moon tasks map"
        );
    }

    // ── 3b. Lane commands — titania-check run-lane <lane> ────────────────────
    let lanes: &[&str] = &[
        "fmt",
        "compile",
        "clippy",
        "ast-grep",
        "dylint",
        "panic-scan",
        "policy-scan",
        "test",
        "deny",
        "build",
    ];
    for lane in lanes {
        let expected = format!("command: 'titania-check run-lane {lane}'");
        assert!(
            text.contains(&expected),
            "lane '{lane}' must have command 'titania-check run-lane {lane}' (found: {expected})"
        );
    }

    // ── 3c. Gate commands — titania-check aggregate --scope <scope> ──────────
    let gates: &[(&str, &str)] =
        &[("gate-edit", "edit"), ("gate-prepush", "prepush"), ("gate-release", "release")];
    for (gate, scope) in gates {
        let expected = format!("command: 'titania-check aggregate --scope {scope}'");
        assert!(
            text.contains(&expected),
            "gate '{gate}' must have command 'titania-check aggregate --scope {scope}'"
        );
    }

    // ── 3d. Selected deps: lane → compile ────────────────────────────────────
    // Moon's task-file form uses `~:` dependencies for sibling tasks.
    let compile_dep = "deps: ['~:titania-compile']";
    let count = text.matches(compile_dep).count();
    assert_eq!(
        count, 3,
        "expected exactly 3 lane tasks (clippy/test/build) to depend on ~:titania-compile, found {count} occurrence(s)"
    );
    // Also assert each task name exists to guard against the file being empty of those tasks.
    for task in &["titania-clippy", "titania-test", "titania-build"] {
        assert!(
            text.contains(&format!("{task}:")),
            "task '{task}' must be a top-level key in tasks/all.yml"
        );
    }

    // ── 3e. Gate deps ────────────────────────────────────────────────────────
    // gate-edit depends on the seven edit lanes
    let edit_lane_tasks = [
        "titania-fmt",
        "titania-compile",
        "titania-clippy",
        "titania-ast-grep",
        "titania-dylint",
        "titania-panic-scan",
        "titania-policy-scan",
    ];
    let gate_edit_deps_line =
        edit_lane_tasks.iter().map(|t| format!("'~:{t}'")).collect::<Vec<_>>().join(", ");
    assert!(
        text.contains(&format!("deps: [{gate_edit_deps_line}]")),
        "gate-edit deps must list all seven edit lanes"
    );

    // gate-prepush depends on ~:gate-edit, ~:titania-test, ~:titania-deny
    assert!(
        text.contains("deps: ['~:gate-edit', '~:titania-test', '~:titania-deny']"),
        "gate-prepush deps must be ['~:gate-edit', '~:titania-test', '~:titania-deny']"
    );

    // gate-release depends on ~:gate-prepush, ~:titania-build
    assert!(
        text.contains("deps: ['~:gate-prepush', '~:titania-build']"),
        "gate-release deps must be ['~:gate-prepush', '~:titania-build']"
    );

    // ── 3f. CARGO_TARGET_DIR env values ──────────────────────────────────────
    // compile, clippy, test, build must set CARGO_TARGET_DIR
    for (task, suffix) in &[
        ("titania-compile", "compile"),
        ("titania-clippy", "clippy"),
        ("titania-test", "test"),
        ("titania-build", "release"),
    ] {
        let expected_env =
            format!("CARGO_TARGET_DIR: '${{workspace.root}}/.titania/cache/{suffix}'");
        assert!(
            text.contains(&expected_env),
            "task '{task}' must set CARGO_TARGET_DIR to ${{workspace.root}}/.titania/cache/{suffix}"
        );
    }

    // ── 3g. Release output artifact ──────────────────────────────────────────
    assert!(
        text.contains("outputs: ['target/release/titania-check']"),
        "titania-build must declare outputs: ['target/release/titania-check']"
    );

    // ── 3h. Absence of repo-specific legacy crate paths ──────────────────────
    // The template must NOT contain hardcoded paths to specific workspace crates.
    let legacy_paths = [
        "crates/titania-core",
        "crates/titania-lanes",
        "crates/titania-policy",
        "crates/titania-check",
    ];
    for path in &legacy_paths {
        assert!(
            !text.contains(path),
            "tasks/all.yml must not contain repo-specific path '{path}' — template tasks must be workspace-agnostic"
        );
    }

    Ok(())
}
