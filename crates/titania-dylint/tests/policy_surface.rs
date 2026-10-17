//! Contract tests for Titania's Dylint rules that replace raw Rust text scans.

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
};

const FIXTURE_FUNCTIONAL_SURFACE: &str = r#"
pub fn checked_methods(value: Option<i32>, result: Result<i32, i32>) -> i32 {
    let first = value.unwrap();
    let second = result.expect("typed error required");
    let third = value.unwrap_or(1);
    let fourth = value.unwrap_or_else(|| 2);
    let fifth = Option::<i32>::None.unwrap_or_default();
    first + second + third + fourth + fifth
}
"#;

const FIXTURE_BOUNDED_FOR_WITH_QUESTION: &str = r#"
pub fn total(items: &[i32]) -> Result<i32, &'static str> {
    let mut sum = 0;
    for value in items {
        sum += process(*value)?;
    }
    Ok(sum)
}

fn process(value: i32) -> Result<i32, &'static str> {
    if value < 0 {
        return Err("negative input");
    }
    Ok(value)
}
"#;

const FIXTURE_UNBOUNDED_LOOP_SURFACE: &str = r#"
pub fn retry_until_ready(mut attempts: u32) -> u32 {
    loop {
        if attempts == 0 {
            break attempts;
        }
        attempts -= 1;
    }
}
"#;

const FIXTURE_UNBOUNDED_WHILE_SURFACE: &str = r#"
pub fn spin_while_busy(mut total: i32) -> i32 {
    while total < 2 {
        total += 1;
    }
    total
}
"#;

const FIXTURE_WHILE_LET_SURFACE: &str = r#"
pub fn drain(items: &mut Vec<i32>) -> i32 {
    let mut sum = 0;
    while let Some(value) = items.pop() {
        sum += value;
    }
    sum
}
"#;

const FIXTURE_UNBOUNDED_ITERATOR_SURFACE: &str = r#"
pub fn repeat_forever() -> impl Iterator<Item = u32> {
    std::iter::repeat(1u32)
}

pub fn cycle_forever(items: Vec<u32>) -> impl Iterator<Item = u32> {
    items.into_iter().cycle()
}
"#;

const FIXTURE_BOUNDED_ITERATOR_SURFACE: &str = r#"
pub fn bounded_retry(max: u64) -> Vec<u64> {
    (0..max).collect()
}

pub fn bounded_pipeline(items: &[i32]) -> Vec<i32> {
    items.iter().map(|value| value + 1).collect()
}
"#;

const FIXTURE_PANIC_SURFACE: &str = r#"
pub fn panic_surface(flag: bool) {
    assert!(flag);
    assert_eq!(1, 1);
    assert_ne!(1, 2);
    dbg!(flag);
    if flag {
        panic!("typed error required");
    }
    if !flag {
        todo!("finish this branch");
    }
    if flag == !flag {
        unimplemented!("finish this branch");
    }
    if flag && !flag {
        unreachable!("type model required");
    }
}
"#;

const FIXTURE_STRINGS_AND_COMMENTS: &str = r#"
pub fn inert_text() -> &'static str {
    // unwrap() panic! for while loop assert_eq! dbg!
    "expect() unwrap_or_default() todo! unimplemented! unreachable!"
}
"#;

const FIXTURE_CUSTOM_SAME_NAME_METHOD: &str = r#"
pub struct Token(i32);

impl Token {
    pub fn unwrap(self) -> i32 {
        self.0
    }

    pub fn expect(self, _message: &str) -> i32 {
        self.0
    }
}

pub fn redeem(first: Token, second: Token) -> i32 {
    first.unwrap() + second.expect("not a panic path")
}
"#;

const FIXTURE_MACRO_EXPANDED_UNWRAP: &str = r#"
macro_rules! take_it {
    ($value:expr) => {
        $value.unwrap()
    };
}

pub fn redeem_macro(value: Option<i32>) -> i32 {
    take_it!(value)
}
"#;

const FIXTURE_UFCS_UNWRAP: &str = r#"
pub fn redeem_ufcs(value: Option<i32>) -> i32 {
    Option::unwrap(value)
}
"#;

const FIXTURE_ALIAS_UNWRAP: &str = r#"
use std::option::Option as Maybe;

pub fn redeem_alias(value: Maybe<i32>) -> i32 {
    Maybe::unwrap(value)
}
"#;

const FIXTURE_CFG_TEST_MODULE: &str = r#"
#[cfg(test)]
mod tests {
    #[test]
    fn test_policy_surface_is_test_only() {
        assert_eq!(1, 1);
        assert_ne!(1, 2);
        assert!(true);
        dbg!(1);
        for value in 0..1 {
            assert_eq!(value, 0);
        }
        while false {}
        loop {
            break;
        }
    }
}

pub fn production_code() -> i32 {
    1
}
"#;

const FIXTURE_OK_DIRECT: &str = r#"
pub fn erase_direct(result: Result<i32, i32>) -> Option<i32> {
    result.ok()
}
"#;

const FIXTURE_OK_PATHS: &str = r#"
use std::result::Result as Fallible;

pub fn erase_paths(
    a: Result<i32, i32>,
    b: Result<i32, i32>,
    c: Result<i32, i32>,
) -> (Option<i32>, Option<i32>, Option<i32>) {
    (Result::ok(a), <Result<i32, i32>>::ok(b), Fallible::ok(c))
}
"#;

const FIXTURE_OK_VALUES: &str = r#"
pub fn erase_values(values: Vec<Result<i32, i32>>) -> Vec<i32> {
    values.into_iter().filter_map(Result::ok).collect()
}

pub fn erase_letbound(result: Result<i32, i32>) -> Option<i32> {
    let erase = Result::ok;
    erase(result)
}
"#;

const FIXTURE_CUSTOM_OK_AND_OK_OR: &str = r#"
pub struct Token(i32);

impl Token {
    pub fn ok(self) -> i32 {
        self.0
    }
}

pub fn keep(token: Token, value: Option<i32>) -> (i32, Result<i32, i32>, Result<i32, i32>) {
    (token.ok(), value.ok_or(-1), value.ok_or_else(|| -2))
}
"#;

const FIXTURE_EXHAUSTIVE: &str = r#"
pub fn settle(result: Result<i32, i32>) -> i32 {
    match result {
        Ok(v) => v,
        Err(e) => e,
    }
}

pub fn carry(result: Result<i32, i32>) -> Result<i32, i32> {
    result.map_err(|e| e)
}

pub fn pass(result: Result<i32, i32>) -> Result<i32, i32> {
    let value = result?;
    Ok(value)
}
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

fn find_titania_dylint_library(root: &Path) -> Option<PathBuf> {
    std::fs::read_dir(root).ok()?.filter_map(Result::ok).find_map(|entry| {
        let path = entry.path();
        if path.is_dir() {
            return find_titania_dylint_library(&path);
        }
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                name.starts_with(
                    format!("{}titania_dylint@", std::env::consts::DLL_PREFIX).as_str(),
                ) && name.ends_with(std::env::consts::DLL_SUFFIX)
            })
            .then_some(path)
    })
}

fn run_dylint_on_fixture(test_name: &str, fixture_code: &str) -> std::process::Output {
    let tmp_dir = std::env::temp_dir()
        .join(format!("titania_dylint_policy_{test_name}_{}", std::process::id()));
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
fn policy_surface_lints_are_registered_by_cargo_dylint() {
    let output = cargo_dylint_list_output();
    assert!(
        output.status.success(),
        "cargo dylint list --all must load the Titania library:\n{}",
        combined_output(&output)
    );
    [
        "FUNC_UNWRAP_USED",
        "FUNC_EXPECT_USED",
        "FUNC_UNWRAP_OR",
        "BOUNDED_EXECUTION_UNBOUNDED_LOOP",
        "BOUNDED_EXECUTION_UNBOUNDED_ITERATOR",
        "HOLZMAN_PANIC_PANIC",
        "HOLZMAN_PANIC_ASSERT",
        "HOLZMAN_PANIC_ASSERT_EQ",
        "HOLZMAN_PANIC_ASSERT_NE",
        "HOLZMAN_PANIC_TODO",
        "HOLZMAN_PANIC_UNIMPLEMENTED",
        "HOLZMAN_PANIC_UNREACHABLE",
        "HOLZMAN_PANIC_DBG",
        "FUNC_RESULT_OK",
    ]
    .iter()
    .for_each(|rule| {
        assert!(
            output_contains(&output, rule),
            "registered lint list must include {rule}:\n{}",
            combined_output(&output)
        );
    });
}

#[test]
fn functional_surface_detects_unwrap_expect_and_unwrap_or_defaults() {
    let output = run_dylint_on_fixture("functional_surface", FIXTURE_FUNCTIONAL_SURFACE);
    assert_dylint_rejects(&output, "FUNC_UNWRAP_USED", "unwrap method calls");
    assert_dylint_rejects(&output, "FUNC_EXPECT_USED", "expect method calls");
    assert_dylint_rejects(&output, "FUNC_UNWRAP_OR", "unwrap_or default methods");
}

#[test]
fn bounded_execution_permits_finite_for_with_question() {
    let output = run_dylint_on_fixture("bounded_for_question", FIXTURE_BOUNDED_FOR_WITH_QUESTION);
    assert!(
        output.status.success(),
        "bounded for over finite input with ? must pass:\n{}",
        combined_output(&output)
    );
}

#[test]
fn bounded_execution_rejects_unbounded_loop() {
    let output = run_dylint_on_fixture("unbounded_loop", FIXTURE_UNBOUNDED_LOOP_SURFACE);
    assert_dylint_rejects(&output, "BOUNDED_EXECUTION_UNBOUNDED_LOOP", "loop blocks");
}

#[test]
fn bounded_execution_rejects_condition_while() {
    let output = run_dylint_on_fixture("unbounded_while", FIXTURE_UNBOUNDED_WHILE_SURFACE);
    assert_dylint_rejects(&output, "BOUNDED_EXECUTION_UNBOUNDED_LOOP", "condition while loops");
}

#[test]
fn bounded_execution_permits_while_let() {
    let output = run_dylint_on_fixture("while_let", FIXTURE_WHILE_LET_SURFACE);
    assert!(
        output.status.success(),
        "while let (bounded by Option exhaustion) must pass:\n{}",
        combined_output(&output)
    );
}

#[test]
fn bounded_execution_rejects_unbounded_iterators() {
    let output = run_dylint_on_fixture("unbounded_iter", FIXTURE_UNBOUNDED_ITERATOR_SURFACE);
    assert_dylint_rejects(&output, "BOUNDED_EXECUTION_UNBOUNDED_ITERATOR", "repeat/cycle sources");
}

#[test]
fn bounded_execution_permits_finite_iterators() {
    let output = run_dylint_on_fixture("bounded_iter", FIXTURE_BOUNDED_ITERATOR_SURFACE);
    assert!(
        output.status.success(),
        "finite ranges and maps must pass:\n{}",
        combined_output(&output)
    );
}

#[test]
fn panic_surface_detects_panic_macros() {
    let output = run_dylint_on_fixture("panic_surface", FIXTURE_PANIC_SURFACE);
    assert_dylint_rejects(&output, "HOLZMAN_PANIC_PANIC", "panic macros");
    assert_dylint_rejects(&output, "HOLZMAN_PANIC_ASSERT", "assert macros");
    assert_dylint_rejects(&output, "HOLZMAN_PANIC_ASSERT_EQ", "assert_eq macros");
    assert_dylint_rejects(&output, "HOLZMAN_PANIC_ASSERT_NE", "assert_ne macros");
    assert_dylint_rejects(&output, "HOLZMAN_PANIC_TODO", "todo macros");
    assert_dylint_rejects(&output, "HOLZMAN_PANIC_UNIMPLEMENTED", "unimplemented macros");
    assert_dylint_rejects(&output, "HOLZMAN_PANIC_UNREACHABLE", "unreachable macros");
    assert_dylint_rejects(&output, "HOLZMAN_PANIC_DBG", "dbg macros");
}

#[test]
fn policy_surface_ignores_strings_and_comments() {
    let output = run_dylint_on_fixture("strings_and_comments", FIXTURE_STRINGS_AND_COMMENTS);
    assert!(
        output.status.success(),
        "strings and comments must not trigger Dylint policy rules:\n{}",
        combined_output(&output)
    );
}

#[test]
fn functional_surface_ignores_unrelated_custom_methods() {
    let output = run_dylint_on_fixture("custom_same_name", FIXTURE_CUSTOM_SAME_NAME_METHOD);
    assert!(
        output.status.success(),
        "custom same-name methods must not trigger resolved-identity rules:\n{}",
        combined_output(&output)
    );
}

#[test]
fn functional_surface_detects_macro_expanded_unwrap() {
    let output = run_dylint_on_fixture("macro_expanded", FIXTURE_MACRO_EXPANDED_UNWRAP);
    assert_dylint_rejects(&output, "FUNC_UNWRAP_USED", "macro-expanded unwrap calls");
}

#[test]
fn functional_surface_detects_ufcs_unwrap() {
    let output = run_dylint_on_fixture("ufcs", FIXTURE_UFCS_UNWRAP);
    assert_dylint_rejects(&output, "FUNC_UNWRAP_USED", "UFCS unwrap calls");
}

#[test]
fn functional_surface_detects_aliased_unwrap() {
    let output = run_dylint_on_fixture("alias", FIXTURE_ALIAS_UNWRAP);
    assert_dylint_rejects(&output, "FUNC_UNWRAP_USED", "aliased unwrap calls");
}

#[test]
fn policy_surface_ignores_cfg_test_modules() {
    let output = run_dylint_on_fixture("cfg_test_module", FIXTURE_CFG_TEST_MODULE);
    assert!(
        output.status.success(),
        "cfg(test) modules must not trigger production Dylint policy rules:\n{}",
        combined_output(&output)
    );
}

#[test]
fn functional_surface_rejects_direct_ok_erasure() {
    let output = run_dylint_on_fixture("direct_ok_erasure", FIXTURE_OK_DIRECT);
    assert_dylint_rejects(&output, "FUNC_RESULT_OK", "direct .ok() erasure");
}

#[test]
fn functional_surface_rejects_ok_path_forms() {
    let output = run_dylint_on_fixture("ok_path_forms", FIXTURE_OK_PATHS);
    assert_dylint_rejects(&output, "FUNC_RESULT_OK", "UFCS Result::ok path call");
    assert_dylint_rejects(
        &output,
        "FUNC_RESULT_OK",
        "fully-qualified <Result<i32, i32>>::ok path call",
    );
    assert_dylint_rejects(&output, "FUNC_RESULT_OK", "renamed-import Fallible::ok path call");
}

#[test]
fn functional_surface_rejects_ok_value_forms() {
    let output = run_dylint_on_fixture("ok_value_forms", FIXTURE_OK_VALUES);
    assert_dylint_rejects(&output, "FUNC_RESULT_OK", "bare-path ok values");
    assert_eq!(
        combined_output(&output).matches("FUNC_RESULT_OK").count(),
        2,
        "bare-path ok values must emit exactly one FUNC_RESULT_OK per site (2 sites):\n{}",
        combined_output(&output)
    );
}

#[test]
fn functional_surface_accepts_custom_ok_and_ok_or() {
    let output = run_dylint_on_fixture("custom_ok_and_ok_or", FIXTURE_CUSTOM_OK_AND_OK_OR);
    assert!(
        output.status.success(),
        "custom ok and ok_or forms must exit 0:\n{}",
        combined_output(&output)
    );
    assert!(
        !output_contains(&output, "FUNC_RESULT_OK"),
        "custom ok and ok_or forms must not emit FUNC_RESULT_OK:\n{}",
        combined_output(&output)
    );
}

#[test]
fn functional_surface_accepts_exhaustive_disposition() {
    let output = run_dylint_on_fixture("exhaustive_disposition", FIXTURE_EXHAUSTIVE);
    assert!(
        output.status.success(),
        "exhaustive disposition must exit 0:\n{}",
        combined_output(&output)
    );
    assert!(
        !output_contains(&output, "FUNC_RESULT_OK"),
        "exhaustive disposition must not emit FUNC_RESULT_OK:\n{}",
        combined_output(&output)
    );
}
