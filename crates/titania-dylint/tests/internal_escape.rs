//! Contract tests for Titania's Dylint rules that detect internal-escape
//! bypass patterns: `BYPASS_ATTR_CONTEXT`, `BYPASS_INTERNAL_UNSTABLE`, and
//! `BYPASS_INTERNAL_UNSAFE`.
//!
//! These tests exercise the real `cargo dylint` CLI against temporary fixture
//! crates. Passing requires a loadable Dylint library, registered lints, and
//! diagnostics emitted by the compiler driver; symbol-table marker exports are
//! not sufficient.

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
};

// ── Fixture: macro generates #[allow(...)] on a pub item ──────────────
//
// This is the canonical `attr_context` trigger: a macro_rules! macro
// produces `#[allow(dead_code)]` on a `pub fn`, so the allow is not
// written directly in user code but "escapes" from the macro expansion.
const FIXTURE_ATTR_CONTEXT: &str = r#"
macro_rules! impl_public {
    ($name:ident) => {
        #[allow(dead_code)]
        pub fn $name() {}
    };
}

impl_public!(public_api);
"#;

const FIXTURE_ATTR_CONTEXT_SELF_SUPPRESSED: &str = r#"
#![allow(bypass_attr_context)]

macro_rules! impl_public {
    ($name:ident) => {
        #[allow(dead_code)]
        pub fn $name() {}
    };
}

impl_public!(public_api);
"#;

// ── Fixture: plain pub fn with direct #[allow(...)] — no macro escape ─
//
// This should NOT trigger BYPASS_ATTR_CONTEXT because the allow is on
// the same source line as the pub fn (no macro mediation).
const FIXTURE_ATTR_CONTEXT_CLEAN: &str = r#"
#[allow(dead_code)]
pub fn public_api() {}
"#;

// ── Fixture: macro with #[allow_internal_unstable(...)] ───────────────
//
// The compiler attribute #[allow_internal_unstable(...)] is applied to a
// macro definition to suppress unstable-lint warnings that originate
// from macro expansion.  This is the canonical trigger for
// BYPASS_INTERNAL_UNSTABLE.
//
// Requires the unstable feature gate because allow_internal_unstable
// itself is an unstable attribute.
const FIXTURE_INTERNAL_UNSTABLE: &str = r#"
#![feature(allow_internal_unstable)]

#[allow_internal_unstable(stability_warning)]
macro_rules! unstable_macro {
    () => {
        pub fn _internal_unstable_item() {}
    };
}

unstable_macro!();
"#;

const FIXTURE_INTERNAL_UNSTABLE_SELF_SUPPRESSED: &str = r#"
#![feature(allow_internal_unstable)]
#![allow(bypass_internal_unstable)]

#[allow_internal_unstable(stability_warning)]
macro_rules! unstable_macro {
    () => {
        pub fn _internal_unstable_item() {}
    };
}

unstable_macro!();
"#;

// ── Fixture: macro without allow_internal_unstable — no trigger ───────
const FIXTURE_INTERNAL_UNSTABLE_CLEAN: &str = r#"
macro_rules! plain_macro {
    () => {
        pub fn _plain_item() {}
    };
}

plain_macro!();
"#;

// ── Fixture: #[allow_internal_unsafe] on a macro ──────────────────────
//
// The compiler attribute #[allow_internal_unsafe] is applied to a macro
// definition to allow unsafe blocks inside the macro even when the
// surrounding crate forbids unsafe code.  This is the canonical trigger
// for BYPASS_INTERNAL_UNSAFE.
//
// Requires the unstable feature gate because allow_internal_unsafe
// itself is an unstable attribute.
const FIXTURE_INTERNAL_UNSAFE: &str = r#"
#![feature(allow_internal_unsafe)]

#[allow_internal_unsafe]
macro_rules! unsafe_macro {
    () => {
        pub fn _internal_unsafe_item() {}
    };
}

unsafe_macro!();
"#;

const FIXTURE_INTERNAL_UNSAFE_SELF_SUPPRESSED: &str = r#"
#![feature(allow_internal_unsafe)]
#![allow(bypass_internal_unsafe)]

#[allow_internal_unsafe]
macro_rules! unsafe_macro {
    () => {
        pub fn _internal_unsafe_item() {}
    };
}

unsafe_macro!();
"#;

// ── Fixture: macro without allow_internal_unsafe — no trigger ─────────
const FIXTURE_INTERNAL_UNSAFE_CLEAN: &str = r#"
macro_rules! plain_macro {
    () => {
        pub fn _plain_item() {}
    };
}

plain_macro!();
"#;

// ── Helpers (mirrored from allow_required.rs) ─────────────────────────

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .map(std::path::Path::to_path_buf)
        .expect("titania-dylint crate must live under <workspace>/crates/titania-dylint")
}

fn cargo_dylint_list_output() -> std::process::Output {
    Command::new("cargo")
        .current_dir(workspace_root())
        .args(["dylint", "list", "--all"])
        .output()
        .expect("cargo-dylint must be installed and runnable")
}

fn dylint_library_path() -> PathBuf {
    let output = cargo_dylint_list_output();
    assert!(
        output.status.success(),
        "cargo dylint list --all must build and load libtitania_dylint before fixture runs:\n{}",
        combined_output(&output)
    );
    let workspace = workspace_root();
    let target_env = std::env::var_os("CARGO_TARGET_DIR");
    let target_dir = resolve_cargo_target_dir(&workspace, target_env.as_deref());
    find_titania_dylint_library(&target_dir.join("dylint").join("libraries"))
        .expect("cargo dylint must build libtitania_dylint into target/dylint/libraries")
}

fn resolve_cargo_target_dir(workspace: &Path, cargo_target_dir: Option<&OsStr>) -> PathBuf {
    let Some(value) = cargo_target_dir else {
        return workspace.join("target");
    };
    let target_dir = PathBuf::from(value);
    if target_dir.is_absolute() { target_dir } else { workspace.join(target_dir) }
}

#[test]
fn resolve_cargo_target_dir_defaults_to_workspace_target_when_env_absent() {
    let workspace = Path::new("/workspace/project");

    let resolved = resolve_cargo_target_dir(workspace, None);

    assert_eq!(resolved, PathBuf::from("/workspace/project/target"));
}

#[test]
fn resolve_cargo_target_dir_joins_relative_env_to_workspace() {
    let workspace = Path::new("/workspace/project");

    let resolved = resolve_cargo_target_dir(workspace, Some(OsStr::new(".titania/cache/dylint")));

    assert_eq!(resolved, PathBuf::from("/workspace/project/.titania/cache/dylint"));
}

#[test]
fn resolve_cargo_target_dir_preserves_absolute_env() {
    let workspace = Path::new("/workspace/project");

    let resolved = resolve_cargo_target_dir(workspace, Some(OsStr::new("/tmp/shared-target")));

    assert_eq!(resolved, PathBuf::from("/tmp/shared-target"));
}

fn find_titania_dylint_library(root: &Path) -> Option<PathBuf> {
    std::fs::read_dir(root).ok()?.filter_map(Result::ok).find_map(|entry| {
        let path = entry.path();
        if path.is_dir() {
            return find_titania_dylint_library(&path);
        }
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("libtitania_dylint@") && name.ends_with(".so"))
            .then_some(path)
    })
}

fn run_dylint_on_fixture(test_name: &str, fixture_code: &str) -> std::process::Output {
    let tmp_dir =
        std::env::temp_dir().join(format!("titania_dylint_{test_name}_{}", std::process::id()));
    let src_dir = tmp_dir.join("src");
    std::fs::create_dir_all(&src_dir).expect("create fixture src directory");
    std::fs::write(src_dir.join("lib.rs"), fixture_code).expect("write fixture source");
    let manifest = tmp_dir.join("Cargo.toml");
    std::fs::write(
        &manifest,
        r#"[package]
name = "fixture"
version = "0.0.0"
edition = "2021"

[lib]
path = "src/lib.rs"
"#,
    )
    .expect("write fixture manifest");

    let output = Command::new("cargo")
        .current_dir(workspace_root())
        .arg("dylint")
        .arg("--lib-path")
        .arg(dylint_library_path())
        .arg("--manifest-path")
        .arg(&manifest)
        .output()
        .expect("cargo-dylint must run against fixture manifest");

    drop(std::fs::remove_dir_all(&tmp_dir));
    output
}

fn combined_output(output: &std::process::Output) -> String {
    format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn output_contains(output: &std::process::Output, needle: &str) -> bool {
    String::from_utf8_lossy(&output.stdout).contains(needle)
        || String::from_utf8_lossy(&output.stderr).contains(needle)
}

fn assert_dylint_rejects(output: &std::process::Output, rule: &str, context: &str) {
    assert!(
        !output.status.success(),
        "cargo dylint must reject {context}:\n{}",
        combined_output(output)
    );
    assert!(
        output_contains(output, rule),
        "cargo dylint must emit {rule} for {context}:\n{}",
        combined_output(output)
    );
}

fn assert_lint_name_is_recognized(output: &std::process::Output, context: &str) {
    assert!(
        !output_contains(output, "unknown lint") && !output_contains(output, "unknown tool name"),
        "self-suppression fixture must use a recognized lint name for {context}:\n{}",
        combined_output(output)
    );
}

fn assert_self_suppression_rejected(
    output: &std::process::Output,
    lint_name: &str,
    rule: &str,
    context: &str,
) {
    assert!(
        !output.status.success(),
        "cargo dylint must reject self-suppression for {context}:\n{}",
        combined_output(output)
    );
    assert_lint_name_is_recognized(output, context);
    assert!(
        output_contains(output, rule)
            || (output_contains(output, "incompatible with previous forbid")
                && output_contains(output, lint_name)),
        "self-suppression must either emit {rule} or be blocked by default forbid for {context}:\n{}",
        combined_output(output)
    );
}

// ── Registration tests ────────────────────────────────────────────────

#[test]
fn bypass_attr_context_is_registered_by_cargo_dylint() {
    let output = cargo_dylint_list_output();
    assert!(
        output.status.success(),
        "cargo dylint list --all must load the Titania library:\n{}",
        combined_output(&output)
    );
    assert!(
        output_contains(&output, "BYPASS_ATTR_CONTEXT"),
        "registered lint list must include BYPASS_ATTR_CONTEXT:\n{}",
        combined_output(&output)
    );
}

#[test]
fn bypass_internal_unstable_is_registered_by_cargo_dylint() {
    let output = cargo_dylint_list_output();
    assert!(
        output.status.success(),
        "cargo dylint list --all must load the Titania library:\n{}",
        combined_output(&output)
    );
    assert!(
        output_contains(&output, "BYPASS_INTERNAL_UNSTABLE"),
        "registered lint list must include BYPASS_INTERNAL_UNSTABLE:\n{}",
        combined_output(&output)
    );
}

#[test]
fn bypass_internal_unsafe_is_registered_by_cargo_dylint() {
    let output = cargo_dylint_list_output();
    assert!(
        output.status.success(),
        "cargo dylint list --all must load the Titania library:\n{}",
        combined_output(&output)
    );

    assert!(
        output_contains(&output, "BYPASS_INTERNAL_UNSAFE"),
        "registered lint list must include BYPASS_INTERNAL_UNSAFE:\n{}",
        combined_output(&output)
    );
}

// ── Diagnostic-emission tests ─────────────────────────────────────────

#[test]
fn bypass_attr_context_detects_macro_generated_allow() {
    let output = run_dylint_on_fixture("attr_context", FIXTURE_ATTR_CONTEXT);
    assert_dylint_rejects(
        &output,
        "BYPASS_ATTR_CONTEXT",
        "`#[allow(...)]` generated inside a macro on a pub fn",
    );
    assert!(
        !output_contains(&output, "BYPASS_PUB_ALLOW"),
        "BYPASS_PUB_ALLOW must not fire on #[allow(...)] generated inside a macro:\n{}",
        combined_output(&output)
    );
}

#[test]
fn bypass_attr_context_rejects_self_suppression() {
    let output =
        run_dylint_on_fixture("attr_context_self_suppressed", FIXTURE_ATTR_CONTEXT_SELF_SUPPRESSED);
    assert_self_suppression_rejected(
        &output,
        "bypass_attr_context",
        "BYPASS_ATTR_CONTEXT",
        "self-suppressed macro-generated allow",
    );
}

#[test]
fn bypass_attr_context_leaves_direct_allow_to_pub_allow() {
    let output = run_dylint_on_fixture("attr_context_clean", FIXTURE_ATTR_CONTEXT_CLEAN);
    assert_dylint_rejects(&output, "BYPASS_PUB_ALLOW", "direct `#[allow(...)]` on pub fn");
    assert!(
        !output_contains(&output, "BYPASS_ATTR_CONTEXT"),
        "BYPASS_ATTR_CONTEXT must not fire when #[allow(...)] is written directly on the pub fn:\n{}",
        combined_output(&output)
    );
}

#[test]
fn bypass_internal_unstable_detects_allow_internal_unstable_attr() {
    let output = run_dylint_on_fixture("internal_unstable", FIXTURE_INTERNAL_UNSTABLE);
    assert_dylint_rejects(&output, "BYPASS_INTERNAL_UNSTABLE", "`#[allow_internal_unstable(...)]`");
}

#[test]
fn bypass_internal_unstable_rejects_self_suppression() {
    let output = run_dylint_on_fixture(
        "internal_unstable_self_suppressed",
        FIXTURE_INTERNAL_UNSTABLE_SELF_SUPPRESSED,
    );
    assert_self_suppression_rejected(
        &output,
        "bypass_internal_unstable",
        "BYPASS_INTERNAL_UNSTABLE",
        "self-suppressed allow_internal_unstable",
    );
}

#[test]
fn bypass_internal_unstable_ignores_plain_function() {
    let output = run_dylint_on_fixture("internal_unstable_clean", FIXTURE_INTERNAL_UNSTABLE_CLEAN);
    assert!(
        output.status.success(),
        "plain-function fixture should not fail dylint execution:\n{}",
        combined_output(&output)
    );
    assert!(
        !output_contains(&output, "BYPASS_INTERNAL_UNSTABLE"),
        "BYPASS_INTERNAL_UNSTABLE must not fire on a plain function:\n{}",
        combined_output(&output)
    );
}

#[test]
fn bypass_internal_unsafe_detects_allow_internal_unsafe_attr() {
    let output = run_dylint_on_fixture("internal_unsafe", FIXTURE_INTERNAL_UNSAFE);
    assert_dylint_rejects(&output, "BYPASS_INTERNAL_UNSAFE", "`#[allow_internal_unsafe]`");
}

#[test]
fn bypass_internal_unsafe_rejects_self_suppression() {
    let output = run_dylint_on_fixture(
        "internal_unsafe_self_suppressed",
        FIXTURE_INTERNAL_UNSAFE_SELF_SUPPRESSED,
    );
    assert_self_suppression_rejected(
        &output,
        "bypass_internal_unsafe",
        "BYPASS_INTERNAL_UNSAFE",
        "self-suppressed allow_internal_unsafe",
    );
}

#[test]
fn bypass_internal_unsafe_ignores_plain_function() {
    let output = run_dylint_on_fixture("internal_unsafe_clean", FIXTURE_INTERNAL_UNSAFE_CLEAN);
    assert!(
        output.status.success(),
        "plain-function fixture should not fail dylint execution:\n{}",
        combined_output(&output)
    );
    assert!(
        !output_contains(&output, "BYPASS_INTERNAL_UNSAFE"),
        "BYPASS_INTERNAL_UNSAFE must not fire on a plain function:\n{}",
        combined_output(&output)
    );
}
