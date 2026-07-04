//! Panic-scan lane behavior tests — failing-first.
//!
//! These tests exercise `check-panic-surface` against synthetic Cargo projects
//! in temp directories and assert:
//!
//! 1. **Exact HOLZMAN rule IDs per macro** — each panic/assert macro must emit its
//!    own distinct rule id (HOLZMAN_PANIC_ASSERT, HOLZMAN_PANIC_ASSERT_EQ,
//!    HOLZMAN_PANIC_ASSERT_NE, HOLZMAN_PANIC_UNREACHABLE).
//! 2. **Path exclusions** — tests/, benches/, examples/, build.rs, target/ must not
//!    produce findings.
//! 3. **Missing rg InfraFailure** — when rg is absent from PATH, the lane records
//!    an InfraFailure with tool "rg" (not a clean pass and not a silent error).
//!
//! Bead: tn-i7q.1

use std::{
    error::Error,
    fs,
    path::Path,
    process::{Command, Output},
};

use tempfile::TempDir;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn binary_path() -> String {
    env!("CARGO_BIN_EXE_check-panic-surface").to_owned()
}

fn write_file(path: impl AsRef<Path>, text: &str) -> std::io::Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)
}

/// Build a minimal workspace fixture at a temp dir with a `crates/` layout:
/// ```text
/// <tmp>/
///   Cargo.toml          (workspace root)
///   crates/
///     example/
///       Cargo.toml      (member crate)
///       src/
///         lib.rs        (source file)
/// ```
/// This matches how `check-panic-surface` discovers `crates/*/src`.
fn workspace_fixture() -> TestResult<TempDir> {
    let tmp = TempDir::new()?;
    write_file(
        tmp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/example\"]\nresolver = \"2\"\n",
    )?;
    write_file(
        tmp.path().join("crates/example/Cargo.toml"),
        "[package]\nname = \"example\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
    )?;
    write_file(tmp.path().join("crates/example/src/lib.rs"), "pub fn example() {}\n")?;
    Ok(tmp)
}

fn run_panic_scan(cwd: &Path) -> Result<Output, std::io::Error> {
    Command::new(binary_path()).current_dir(cwd).output()
}

fn run_panic_scan_no_rg(cwd: &Path) -> Result<Output, std::io::Error> {
    let original_path = std::env::var("PATH").unwrap_or_default();
    let filtered = original_path
        .split(':')
        .filter(|dir| {
            !dir.ends_with("/.cargo/bin")
                && !dir.ends_with("/cargo/bin")
                && !dir.ends_with("/rustup/toolchains")
                && !dir.ends_with("/bin")
                && !dir.ends_with("/usr/bin")
                && !dir.ends_with("/usr/local/bin")
        })
        .collect::<Vec<_>>()
        .join(":");

    Command::new(binary_path()).current_dir(cwd).env("PATH", filtered).output()
}

fn stderr_text(output: &Output) -> Result<String, std::string::FromUtf8Error> {
    String::from_utf8(output.stderr.clone())
}

// ---------------------------------------------------------------------------
// 1. Exact HOLZMAN rule IDs per macro
// ---------------------------------------------------------------------------

/// A file with `assert_eq!` in production code must emit
/// `HOLZMAN_PANIC_ASSERT_EQ` — NOT the generic `PANIC_SURFACE_001`.
#[test]
fn panic_scan_assert_eq_emits_holzman_rule_id() -> TestResult {
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/src/lib.rs"),
        "pub fn check(x: i32, y: i32) {\n    assert_eq!(x, y, \"match\");\n}\n",
    )?;

    let output = run_panic_scan(fixture.path())?;
    assert!(!output.status.success(), "assert_eq! should produce a finding");

    let stderr = stderr_text(&output)?;
    assert!(
        stderr.contains("HOLZMAN_PANIC_ASSERT_EQ"),
        "expected rule HOLZMAN_PANIC_ASSERT_EQ, got:\n{stderr}"
    );
    // Production MUST NOT use the generic rule id for assert_eq!
    assert!(
        !stderr.contains("PANIC_SURFACE_001") || stderr.contains("HOLZMAN_PANIC_ASSERT_EQ"),
        "assert_eq! should use HOLZMAN_PANIC_ASSERT_EQ, not generic PANIC_SURFACE_001"
    );
    Ok(())
}

/// A file with `assert_ne!` in production code must emit
/// `HOLZMAN_PANIC_ASSERT_NE`.
#[test]
fn panic_scan_assert_ne_emits_holzman_rule_id() -> TestResult {
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/src/lib.rs"),
        "pub fn check(x: i32, y: i32) {\n    assert_ne!(x, y, \"diff\");\n}\n",
    )?;

    let output = run_panic_scan(fixture.path())?;
    assert!(!output.status.success(), "assert_ne! should produce a finding");

    let stderr = stderr_text(&output)?;
    assert!(
        stderr.contains("HOLZMAN_PANIC_ASSERT_NE"),
        "expected rule HOLZMAN_PANIC_ASSERT_NE, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("PANIC_SURFACE_001") || stderr.contains("HOLZMAN_PANIC_ASSERT_NE"),
        "assert_ne! should use HOLZMAN_PANIC_ASSERT_NE, not generic PANIC_SURFACE_001"
    );
    Ok(())
}

/// A file with `assert!` in production code must emit
/// `HOLZMAN_PANIC_ASSERT`.
#[test]
fn panic_scan_assert_emits_holzman_rule_id() -> TestResult {
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/src/lib.rs"),
        "pub fn guard(ok: bool) {\n    assert!(ok, \"must be true\");\n}\n",
    )?;

    let output = run_panic_scan(fixture.path())?;
    assert!(!output.status.success(), "assert! should produce a finding");

    let stderr = stderr_text(&output)?;
    assert!(
        stderr.contains("HOLZMAN_PANIC_ASSERT"),
        "expected rule HOLZMAN_PANIC_ASSERT, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("PANIC_SURFACE_001") || stderr.contains("HOLZMAN_PANIC_ASSERT"),
        "assert! should use HOLZMAN_PANIC_ASSERT, not generic PANIC_SURFACE_001"
    );
    Ok(())
}

/// A file with `unreachable!` in production code must emit
/// `HOLZMAN_PANIC_UNREACHABLE`.
#[test]
fn panic_scan_unreachable_emits_holzman_rule_id() -> TestResult {
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/src/lib.rs"),
        "pub fn impossible() {\n    unreachable!(\"should not reach\");\n}\n",
    )?;

    let output = run_panic_scan(fixture.path())?;
    assert!(!output.status.success(), "unreachable! should produce a finding");

    let stderr = stderr_text(&output)?;
    assert!(
        stderr.contains("HOLZMAN_PANIC_UNREACHABLE"),
        "expected rule HOLZMAN_PANIC_UNREACHABLE, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("PANIC_SURFACE_001") || stderr.contains("HOLZMAN_PANIC_UNREACHABLE"),
        "unreachable! should use HOLZMAN_PANIC_UNREACHABLE, not generic PANIC_SURFACE_001"
    );
    Ok(())
}

/// Clean file (no panic macros) must exit 0.
#[test]
fn panic_scan_clean_source_exits_zero() -> TestResult {
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/src/lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 {\n    a.checked_add(b).expect(\"overflow\")\n}\n",
    )?;

    let output = run_panic_scan(fixture.path())?;
    assert!(
        output.status.success(),
        "clean source should exit zero, got status {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 2. Path exclusions — no findings from non-production paths
// ---------------------------------------------------------------------------

/// assert! inside tests/ must be excluded.
#[test]
fn panic_scan_excluded_tests_dir() -> TestResult {
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/tests/unit/mod.rs"),
        "#[test]\nfn example() {\n    assert_eq!(1, 1);\n}\n",
    )?;

    let output = run_panic_scan(fixture.path())?;
    assert!(
        output.status.success(),
        "tests/ should be excluded, got status {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

/// assert! inside benches/ must be excluded.
#[test]
fn panic_scan_excluded_benches_dir() -> TestResult {
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/benches/benchmarks/mod.rs"),
        "#[bench]\nfn example(_b: &mut test::Bencher) {\n    assert!(true);\n}\n",
    )?;

    let output = run_panic_scan(fixture.path())?;
    assert!(
        output.status.success(),
        "benches/ should be excluded, got status {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

/// assert! inside examples/ must be excluded.
#[test]
fn panic_scan_excluded_examples_dir() -> TestResult {
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/examples/sample/main.rs"),
        "fn main() {\n    assert!(true, \"example\");\n}\n",
    )?;

    let output = run_panic_scan(fixture.path())?;
    assert!(
        output.status.success(),
        "examples/ should be excluded, got status {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

/// assert! inside build.rs must be excluded.
#[test]
fn panic_scan_excluded_build_rs() -> TestResult {
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/build.rs"),
        "fn main() {\n    assert!(true, \"build script check\");\n}\n",
    )?;

    let output = run_panic_scan(fixture.path())?;
    assert!(
        output.status.success(),
        "build.rs should be excluded, got status {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

/// assert! inside target/ must be excluded.
#[test]
fn panic_scan_excluded_target_dir() -> TestResult {
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/target/debug/build/out.rs"),
        "pub fn foo() {\n    assert!(true);\n}\n",
    )?;

    let output = run_panic_scan(fixture.path())?;
    assert!(
        output.status.success(),
        "target/ should be excluded, got status {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 3. Missing rg / PATH — InfraFailure with tool rg
// ---------------------------------------------------------------------------

/// When rg is absent from PATH, the lane MUST record an InfraFailure with
/// tool "rg" — never a clean pass and never a silent crash.
#[test]
fn panic_scan_missing_rg_records_infra_failure() -> TestResult {
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/src/lib.rs"),
        "pub fn check(x: i32) {\n    assert_eq!(x, 0);\n}\n",
    )?;

    let output = run_panic_scan_no_rg(fixture.path())?;

    // The lane must record an InfraFailure with tool "rg" when rg is absent from PATH.
    let stderr = stderr_text(&output)?;

    assert!(
        !output.status.success(),
        "missing rg must NOT result in a clean pass; got status {:?}\nstderr: {}",
        output.status.code(),
        stderr
    );

    assert!(
        stderr.contains("InfraFailure") || stderr.contains("infra_failure"),
        "missing rg must record InfraFailure with tool rg, got:\n{stderr}"
    );

    assert!(stderr.contains("rg"), "InfraFailure message must name tool 'rg', got:\n{stderr}");
    Ok(())
}
