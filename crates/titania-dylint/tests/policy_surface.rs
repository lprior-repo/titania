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

const FIXTURE_LOOP_SURFACE: &str = r#"
pub fn loops() -> i32 {
    let mut total = 0;
    for value in 0..1 {
        total += value;
    }
    while total < 2 {
        total += 1;
    }
    loop {
        break total;
    }
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
            .is_some_and(|name| name.starts_with("libtitania_dylint@") && name.ends_with(".so"))
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
        "FUNC_LOOPS_FOR",
        "FUNC_LOOPS_WHILE",
        "FUNC_LOOPS_LOOP",
        "HOLZMAN_PANIC_PANIC",
        "HOLZMAN_PANIC_ASSERT",
        "HOLZMAN_PANIC_ASSERT_EQ",
        "HOLZMAN_PANIC_ASSERT_NE",
        "HOLZMAN_PANIC_TODO",
        "HOLZMAN_PANIC_UNIMPLEMENTED",
        "HOLZMAN_PANIC_UNREACHABLE",
        "HOLZMAN_PANIC_DBG",
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
fn functional_surface_detects_imperative_loops() {
    let output = run_dylint_on_fixture("loop_surface", FIXTURE_LOOP_SURFACE);
    assert_dylint_rejects(&output, "FUNC_LOOPS_FOR", "for loops");
    assert_dylint_rejects(&output, "FUNC_LOOPS_WHILE", "while loops");
    assert_dylint_rejects(&output, "FUNC_LOOPS_LOOP", "loop blocks");
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
fn policy_surface_ignores_cfg_test_modules() {
    let output = run_dylint_on_fixture("cfg_test_module", FIXTURE_CFG_TEST_MODULE);
    assert!(
        output.status.success(),
        "cfg(test) modules must not trigger production Dylint policy rules:\n{}",
        combined_output(&output)
    );
}
