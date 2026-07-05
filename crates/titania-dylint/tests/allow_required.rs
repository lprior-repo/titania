//! Contract tests for Titania's `libtitania_dylint` public-allow and
//! required-lint weakening rules.
//!
//! These tests exercise the real `cargo dylint` CLI against temporary fixture
//! crates. Passing requires a loadable Dylint library, registered lints, and
//! diagnostics emitted by the compiler driver; symbol-table marker exports are
//! not sufficient.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

const FIXTURE_PUB_ALLOW: &str = r#"
#[allow(dead_code)]
pub fn public_api() {}
"#;

const FIXTURE_PUB_ALLOW_WARNINGS: &str = r#"
#[allow(warnings)]
pub fn public_api() {}
"#;

const FIXTURE_PUB_CRATE_ALLOW: &str = r#"
#[allow(dead_code)]
pub(crate) fn crate_api() {}
"#;

const FIXTURE_WEAKEN_FORBID: &str = r#"
#![allow(unsafe_code)]

pub fn kept() {}
"#;

const FIXTURE_WEAKEN_UNWRAP: &str = r#"
#![allow(clippy::unwrap_used)]

pub fn kept() {}
"#;

const FIXTURE_WEAKEN_WARNINGS: &str = r#"
#![allow(warnings)]

pub fn kept() {}
"#;

const FIXTURE_NO_WEAKEN: &str = r#"
#![deny(clippy::unwrap_used)]
#![forbid(unsafe_code)]

pub fn kept() {}
"#;

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
    find_titania_dylint_library(&workspace_root().join("target").join("dylint").join("libraries"))
        .expect("cargo dylint must build libtitania_dylint into target/dylint/libraries")
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

#[test]
fn bypass_pub_allow_is_registered_by_cargo_dylint() {
    let output = cargo_dylint_list_output();
    assert!(
        output.status.success(),
        "cargo dylint list --all must load the Titania library:\n{}",
        combined_output(&output)
    );
    assert!(
        output_contains(&output, "BYPASS_PUB_ALLOW"),
        "registered lint list must include BYPASS_PUB_ALLOW:\n{}",
        combined_output(&output)
    );
}

#[test]
fn bypass_pub_allow_detects_public_allow_attribute() {
    let output = run_dylint_on_fixture("pub_allow", FIXTURE_PUB_ALLOW);
    assert_dylint_rejects(&output, "BYPASS_PUB_ALLOW", "`#[allow]` pub fn");
}

#[test]
fn bypass_pub_allow_detects_public_allow_warnings() {
    let output = run_dylint_on_fixture("pub_allow_warnings", FIXTURE_PUB_ALLOW_WARNINGS);
    assert_dylint_rejects(&output, "BYPASS_PUB_ALLOW", "public API allowing warnings");
}

#[test]
fn bypass_pub_allow_ignores_pub_crate_allow_attribute() {
    let output = run_dylint_on_fixture("pub_crate_allow", FIXTURE_PUB_CRATE_ALLOW);
    assert!(
        output.status.success(),
        "pub(crate) allow fixture should not fail dylint execution:\n{}",
        combined_output(&output)
    );
    assert!(
        !output_contains(&output, "BYPASS_PUB_ALLOW"),
        "BYPASS_PUB_ALLOW must not fire on `pub(crate)` items:\n{}",
        combined_output(&output)
    );
}

#[test]
fn bypass_required_lint_weakening_is_registered_by_cargo_dylint() {
    let output = cargo_dylint_list_output();
    assert!(
        output.status.success(),
        "cargo dylint list --all must load the Titania library:\n{}",
        combined_output(&output)
    );
    assert!(
        output_contains(&output, "BYPASS_REQUIRED_LINT_WEAKENING"),
        "registered lint list must include BYPASS_REQUIRED_LINT_WEAKENING:\n{}",
        combined_output(&output)
    );
}

#[test]
fn bypass_required_lint_weakening_detects_forbid_downgrade() {
    let output = run_dylint_on_fixture("weaken_forbid", FIXTURE_WEAKEN_FORBID);
    assert_dylint_rejects(&output, "BYPASS_REQUIRED_LINT_WEAKENING", "unsafe_code allow");
}

#[test]
fn bypass_required_lint_weakening_detects_deny_unwrap_removal() {
    let output = run_dylint_on_fixture("weaken_unwrap", FIXTURE_WEAKEN_UNWRAP);
    assert_dylint_rejects(&output, "BYPASS_REQUIRED_LINT_WEAKENING", "clippy::unwrap_used allow");
}

#[test]
fn bypass_required_lint_weakening_detects_allow_warnings() {
    let output = run_dylint_on_fixture("weaken_warnings", FIXTURE_WEAKEN_WARNINGS);
    assert_dylint_rejects(&output, "BYPASS_REQUIRED_LINT_WEAKENING", "crate allowing warnings");
}

#[test]
fn bypass_required_lint_weakening_ignores_preserved_lints() {
    let output = run_dylint_on_fixture("no_weaken", FIXTURE_NO_WEAKEN);
    assert!(
        output.status.success(),
        "preserved-lint fixture should pass dylint execution:\n{}",
        combined_output(&output)
    );
    assert!(
        !output_contains(&output, "BYPASS_REQUIRED_LINT_WEAKENING"),
        "BYPASS_REQUIRED_LINT_WEAKENING must not fire when required lints are preserved:\n{}",
        combined_output(&output)
    );
}

// ── clippy::expect_used weakening ──────────────────────────────────────────

const FIXTURE_WEAKEN_EXPECT: &str = r#"
#![allow(clippy::expect_used)]

pub fn kept() {}
"#;

// ── clippy::panic weakening ────────────────────────────────────────────────

const FIXTURE_WEAKEN_PANIC: &str = r#"
#![allow(clippy::panic)]

pub fn kept() {}
"#;

// ── public associated function (struct impl) ───────────────────────────────

const FIXTURE_ASSOC_PUB_ALLOW: &str = r#"
pub struct Api;

impl Api {
    #[allow(dead_code)]
    pub fn method(&self) {}
}
"#;

const FIXTURE_ASSOC_PUB_CRATE_ALLOW: &str = r#"
pub struct Api;

impl Api {
    #[allow(dead_code)]
    pub(crate) fn method(&self) {}
}
"#;
// ── public trait method (direct allow) ────────────────────────────────────

const FIXTURE_TRAIT_PUB_ALLOW: &str = r#"
pub trait PublicTrait {
    #[allow(dead_code)]
    fn trait_method(&self);
}
"#;

// ── macro-generated public associated method ──────────────────────────────

const FIXTURE_MACRO_ASSOC_PUB_ALLOW: &str = r#"
macro_rules! impl_method {
    ($name:ident) => {
        #[allow(dead_code)]
        pub fn $name(&self) {}
    };
}

pub struct Api;

impl Api {
    impl_method!(method);
}
"#;

#[test]
fn bypass_required_lint_weakening_detects_expect_used_removal() {
    let output = run_dylint_on_fixture("weaken_expect", FIXTURE_WEAKEN_EXPECT);
    assert_dylint_rejects(&output, "BYPASS_REQUIRED_LINT_WEAKENING", "clippy::expect_used allow");
}

#[test]
fn bypass_required_lint_weakening_detects_panic_removal() {
    let output = run_dylint_on_fixture("weaken_panic", FIXTURE_WEAKEN_PANIC);
    assert_dylint_rejects(&output, "BYPASS_REQUIRED_LINT_WEAKENING", "clippy::panic allow");
}

#[test]
fn bypass_pub_allow_detects_public_associated_fn() {
    let output = run_dylint_on_fixture("assoc_pub_allow", FIXTURE_ASSOC_PUB_ALLOW);
    assert_dylint_rejects(&output, "BYPASS_PUB_ALLOW", "`#[allow]` on pub associated fn");
}

#[test]
fn bypass_pub_allow_ignores_pub_crate_associated_fn() {
    let output = run_dylint_on_fixture("assoc_pub_crate_allow", FIXTURE_ASSOC_PUB_CRATE_ALLOW);
    assert!(
        output.status.success(),
        "pub(crate) associated fn fixture should not fail dylint execution:\n{}",
        combined_output(&output)
    );
    assert!(
        !output_contains(&output, "BYPASS_PUB_ALLOW"),
        "BYPASS_PUB_ALLOW must not fire on `pub(crate)` associated items:\n{}",
        combined_output(&output)
    );
}
// ── public trait method (direct allow) ────────────────────────────────────

#[test]
fn bypass_pub_allow_detects_public_trait_method() {
    let output = run_dylint_on_fixture("trait_pub_allow", FIXTURE_TRAIT_PUB_ALLOW);
    assert_dylint_rejects(&output, "BYPASS_PUB_ALLOW", "`#[allow]` on pub trait method");
    assert!(
        !output_contains(&output, "BYPASS_ATTR_CONTEXT"),
        "BYPASS_ATTR_CONTEXT must not fire on direct #[allow] on pub trait method:\n{}",
        combined_output(&output)
    );
}

// ── macro-generated public associated method ──────────────────────────────

#[test]
fn bypass_attr_context_detects_macro_generated_associated_method() {
    let output = run_dylint_on_fixture("macro_assoc_pub_allow", FIXTURE_MACRO_ASSOC_PUB_ALLOW);
    assert_dylint_rejects(
        &output,
        "BYPASS_ATTR_CONTEXT",
        "`#[allow]` generated by macro on pub associated fn",
    );
    assert!(
        !output_contains(&output, "BYPASS_PUB_ALLOW"),
        "BYPASS_PUB_ALLOW must not fire when #[allow] comes from macro expansion:\n{}",
        combined_output(&output)
    );
}
