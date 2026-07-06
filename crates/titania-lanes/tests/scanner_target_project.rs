//! Scanner lane behavior tests against a synthetic target project.

use std::{
    error::Error,
    fs,
    path::Path,
    process::{Command, Output},
};

use tempfile::TempDir;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

#[test]
fn hotpath_scan_from_member_scans_workspace_root() -> TestResult {
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/vb_core/src/lib.rs"),
        "use std::collections::HashMap;\n",
    )?;

    let output = run_from_member(env!("CARGO_BIN_EXE_hotpath-scan"), &fixture)?;

    assert!(!output.status.success(), "scanner missed root hotpath token");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("crates/vb_core/src/lib.rs"), "stderr was: {stderr}");
    assert!(stderr.contains("token HashMap on hot path"), "stderr was: {stderr}");
    Ok(())
}

#[test]
fn panic_surface_from_member_scans_workspace_root() -> TestResult {
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/src/lib.rs"),
        "pub fn guard(value: bool) {\n    assert!(value);\n}\n",
    )?;

    let output = run_from_member(env!("CARGO_BIN_EXE_check-panic-surface"), &fixture)?;

    assert!(!output.status.success(), "scanner missed root panic macro");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("crates/example/src/lib.rs"), "stderr was: {stderr}");
    assert!(stderr.contains("PANIC_SURFACE_001"), "stderr was: {stderr}");
    Ok(())
}

#[test]
fn production_inner_drift_from_member_scans_workspace_root() -> TestResult {
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/src/lib.rs"),
        "pub struct MissingIdentifier;\n",
    )?;
    write_file(
        fixture.path().join("verification/verus/production_inner/example.rs"),
        concat!(
            "// DRIFT POLICY: `crates/example/src/lib.rs`\n",
            "// Production source: `crates/example/src/lib.rs:1-1`\n",
            "pub struct PresentIdentifier;\n",
        ),
    )?;

    let output = run_from_member(env!("CARGO_BIN_EXE_check-production-inner-drift"), &fixture)?;

    assert!(!output.status.success(), "scanner missed root production-inner drift");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("verification/verus/production_inner/example.rs"),
        "stderr was: {stderr}"
    );
    assert!(stderr.contains("missing identifiers"), "stderr was: {stderr}");
    assert!(stderr.contains("MissingIdentifier"), "stderr was: {stderr}");
    Ok(())
}

fn workspace_fixture() -> TestResult<TempDir> {
    let fixture = TempDir::new()?;
    write_file(
        fixture.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"member\"]\nresolver = \"2\"\n",
    )?;
    write_file(
        fixture.path().join("member/Cargo.toml"),
        "[package]\nname = \"member\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
    )?;
    write_file(fixture.path().join("member/src/lib.rs"), "pub fn member() {}\n")?;
    Ok(fixture)
}

fn run_from_member(binary: &str, fixture: &TempDir) -> TestResult<Output> {
    Ok(Command::new(binary).current_dir(fixture.path().join("member")).output()?)
}

fn write_file(path: impl AsRef<Path>, text: &str) -> std::io::Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)
}
#[test]
fn panic_surface_flags_assert_after_cfg_test_mod_inside_fn() -> TestResult {
    // Regression: the cfg(test) scope tracker used to leave the cfg
    // scope open past its closing `}` when the `#[cfg(test)] mod` was
    // nested inside a function body, so an `assert!(false)` placed
    // after the block slipped through silently.
    let fixture = workspace_fixture()?;
    let lib = "pub fn a() {\n\
               \x20\x20\x20\x20#[cfg(test)]\n\
               \x20\x20\x20\x20mod tests {\n\
               \x20\x20\x20\x20\x20\x20\x20\x20#[test]\n\
               \x20\x20\x20\x20\x20\x20\x20\x20fn x() {\n\
               \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20assert!(true);\n\
               \x20\x20\x20\x20\x20\x20\x20\x20}\n\
               \x20\x20\x20\x20}\n\
               \x20\x20\x20\x20assert!(false);\n\
               }\n";
    write_file(fixture.path().join("crates/example/src/lib.rs"), lib)?;

    let output = run_from_member(env!("CARGO_BIN_EXE_check-panic-surface"), &fixture)?;

    assert!(!output.status.success(), "scanner missed assert after cfg(test) mod");
    let stderr = String::from_utf8_lossy(&output.stderr);
    // The assert OUTSIDE the cfg(test) block (line 9) must be reported
    // as a violation, with the macro name `assert!` in the message.
    assert!(stderr.contains("lib.rs:9"), "expected finding at lib.rs:9; stderr was: {stderr}");
    assert!(stderr.contains("PANIC_SURFACE_001"), "stderr was: {stderr}");
    // The assert INSIDE the cfg(test) block (line 5) must NOT appear.
    assert!(!stderr.contains("lib.rs:5"), "cfg(test) internals leaked: {stderr}");
    Ok(())
}

#[test]
fn panic_surface_does_not_flag_assert_inside_top_level_cfg_test_mod() -> TestResult {
    // Counterpart: a top-level (not nested) `#[cfg(test)] mod` must
    // suppress the inner `assert!` and still flag the outer one.
    let fixture = workspace_fixture()?;
    let lib = "#[cfg(test)]\n\
               mod tests {\n\
               \x20\x20\x20\x20#[test]\n\
               \x20\x20\x20\x20fn x() {\n\
               \x20\x20\x20\x20\x20\x20\x20\x20assert!(true);\n\
               \x20\x20\x20\x20}\n\
               }\n\
               assert!(false);\n";
    write_file(fixture.path().join("crates/example/src/lib.rs"), lib)?;

    let output = run_from_member(env!("CARGO_BIN_EXE_check-panic-surface"), &fixture)?;

    assert!(!output.status.success(), "scanner missed outer assert");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("lib.rs:8"), "expected finding at lib.rs:8; stderr was: {stderr}");
    // Inner assert at line 5 must NOT be flagged.
    assert!(!stderr.contains("lib.rs:5"), "cfg(test) internals leaked: {stderr}");
    Ok(())
}

#[test]
fn nightly_features_single_line_allowed_feature_is_not_flagged() -> TestResult {
    // Regression: `push_closed_feature` used to slice up to the `)` of
    // the `)]` close, leaving the `]` out of the slice. The downstream
    // `trim_end_matches(")]")` then failed to strip the suffix, so a
    // single-line `#![feature(try_blocks)]` was reported as the
    // literal feature name `try_blocks)` and treated as disallowed.
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/src/lib.rs"),
        "#![feature(try_blocks)]\npub fn a() {}\n",
    )?;

    // The nightly lane walks the current directory; the existing
    // helper `run_from_member` cd's into `member/`, so for this test
    // we run directly from the workspace root.
    let output = Command::new(env!("CARGO_BIN_EXE_check-nightly-features"))
        .current_dir(fixture.path())
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("try_blocks)"),
        "feature name carries stray ')'; stderr was: {stderr}"
    );
    assert!(stderr.contains("no disallowed feature attributes"), "stderr was: {stderr}");
    Ok(())
}

#[test]
fn nightly_features_multi_line_attribute_extracts_every_name() -> TestResult {
    // The two perf-only features span multiple lines. Each must be
    // extracted as its own feature (no leading whitespace, no `,`).
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/src/lib.rs"),
        "#![feature(\n    allocator_api,\n    generic_const_exprs\n)]\npub fn a() {}\n",
    )?;
    let output = Command::new(env!("CARGO_BIN_EXE_check-nightly-features"))
        .current_dir(fixture.path())
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Both names must appear in their clean form.
    assert!(stderr.contains("`allocator_api`"), "stderr was: {stderr}");
    assert!(stderr.contains("`generic_const_exprs`"), "stderr was: {stderr}");
    Ok(())
}

#[test]
fn forbidden_scan_flags_expect_with_message() -> TestResult {
    // Regression: `expect()` (with both parens) never appears as a
    // literal substring in real Rust code (`.expect("msg")` only has
    // `expect(`). The token set now stores `expect` as a `Method` and
    // matches when followed by `(`.
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/src/lib.rs"),
        "use std::fs;\npub fn a() -> String {\n    fs::read_to_string(\"/tmp/x\").expect(\"boom\")\n}\n",
    )?;
    let output =
        Command::new(env!("CARGO_BIN_EXE_forbidden-scan")).current_dir(fixture.path()).output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "scanner missed .expect()");
    assert!(stderr.contains("`expect`"), "stderr was: {stderr}");
    Ok(())
}

#[test]
fn forbidden_scan_does_not_flag_user_identifiers_named_like_tokens() -> TestResult {
    // The Method matcher requires `.` or `::` immediately before the
    // name and `(` immediately after, so user identifiers like
    // `myexpect()` are not flagged.
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/src/lib.rs"),
        "fn myexpect() -> i32 { 1 }\nfn myunwrap() -> i32 { 2 }\npub fn a() -> i32 { myexpect() + myunwrap() }\n",
    )?;
    let output =
        Command::new(env!("CARGO_BIN_EXE_forbidden-scan")).current_dir(fixture.path()).output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "false positive: {stderr}");
    Ok(())
}

#[test]
fn forbidden_scan_flags_qualified_path_unwrap() -> TestResult {
    // `Result::unwrap(...)` is a forbidden method call. The Method
    // matcher accepts `::` as a receiver.
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/src/lib.rs"),
        "use std::fs;\npub fn a() -> String {\n    let r: Result<String, _> = fs::read_to_string(\"/tmp/x\");\n    Result::unwrap(r)\n}\n",
    )?;
    let output =
        Command::new(env!("CARGO_BIN_EXE_forbidden-scan")).current_dir(fixture.path()).output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "scanner missed Result::unwrap");
    assert!(stderr.contains("`unwrap`"), "stderr was: {stderr}");
    Ok(())
}

#[test]
fn panic_surface_skips_block_comments() -> TestResult {
    // Regression: the old `is_comment` only checked `//`, so an
    // `assert!` inside a `/* ... */` block comment was flagged. The
    // panic-surface lane now uses the shared `SourceLine` parser,
    // which strips line/block comments and blanks string contents.
    let fixture = workspace_fixture()?;
    let lib = "pub fn a() {\n    /* assert!(true); */\n    let _x = 1;\n}\n";
    write_file(fixture.path().join("crates/example/src/lib.rs"), lib)?;
    let output = run_from_member(env!("CARGO_BIN_EXE_check-panic-surface"), &fixture)?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "block comment leaked: {stderr}");
    Ok(())
}

#[test]
fn panic_surface_skips_assert_inside_string_literal() -> TestResult {
    // An `assert!` that is purely a string literal is not real code
    // and must not be flagged. The `SourceLine` parser blanks string
    // contents so the panic-macro check never sees them.
    let fixture = workspace_fixture()?;
    let lib = "pub const DOC: &str = \"do not call assert!(true) here\";\npub fn a() {}\n";
    write_file(fixture.path().join("crates/example/src/lib.rs"), lib)?;
    let output = run_from_member(env!("CARGO_BIN_EXE_check-panic-surface"), &fixture)?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "string literal leaked: {stderr}");
    Ok(())
}

#[test]
fn nightly_features_dylint_rustc_private_is_allowed() -> TestResult {
    // Regression gate: `crates/titania-dylint/src/lib.rs` carries
    // `#![feature(rustc_private)]` because Dylint is a rustc compiler
    // plugin. The nightly-feature lane must NOT flag this feature.
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/titania-dylint/src/lib.rs"),
        "#![feature(rustc_private)]\n\nextern crate rustc_ast;\n",
    )?;
    let output = Command::new(env!("CARGO_BIN_EXE_check-nightly-features"))
        .current_dir(fixture.path())
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "dylint rustc_private was flagged; stderr was: {stderr}");
    assert!(
        stderr.contains("no disallowed feature attributes"),
        "expected clean pass; stderr was: {stderr}"
    );
    Ok(())
}

#[test]
fn nightly_features_normal_crate_rustc_private_is_rejected() -> TestResult {
    // `rustc_private` in an ordinary production crate is disallowed.
    // Only the Dylint plugin path is exempted.
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/src/lib.rs"),
        "#![feature(rustc_private)]\npub fn a() {}\n",
    )?;
    let output = Command::new(env!("CARGO_BIN_EXE_check-nightly-features"))
        .current_dir(fixture.path())
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "rustc_private in normal crate should be rejected; stderr was: {stderr}"
    );
    assert!(stderr.contains("`rustc_private`"), "stderr was: {stderr}");
    assert!(stderr.contains("NIGHTLY_FEATURE_001"), "stderr was: {stderr}");
    Ok(())
}

#[test]
fn nightly_features_allow_internal_unstable_in_dylint_fixture_passes() -> TestResult {
    // `allow_internal_unstable` is allowed only in the Dylint test
    // fixture path `crates/titania-dylint/tests/internal_escape.rs`.
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/titania-dylint/tests/internal_escape.rs"),
        "#![feature(allow_internal_unstable)]\n",
    )?;
    let output = Command::new(env!("CARGO_BIN_EXE_check-nightly-features"))
        .current_dir(fixture.path())
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "dylint fixture allow_internal_unstable was flagged; stderr was: {stderr}"
    );
    assert!(
        stderr.contains("no disallowed feature attributes"),
        "expected clean pass; stderr was: {stderr}"
    );
    Ok(())
}

#[test]
fn nightly_features_allow_internal_unstable_in_normal_crate_is_rejected() -> TestResult {
    // `allow_internal_unstable` in an ordinary production crate is disallowed.
    // Only the Dylint test-fixture path is exempted.
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/src/lib.rs"),
        "#![feature(allow_internal_unstable)]\npub fn a() {}\n",
    )?;
    let output = Command::new(env!("CARGO_BIN_EXE_check-nightly-features"))
        .current_dir(fixture.path())
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "allow_internal_unstable in normal crate should be rejected; stderr was: {stderr}"
    );
    assert!(stderr.contains("`allow_internal_unstable`"), "stderr was: {stderr}");
    assert!(stderr.contains("NIGHTLY_FEATURE_001"), "stderr was: {stderr}");
    Ok(())
}
#[test]
fn forbidden_scan_skips_unwrap_inside_raw_string() -> TestResult {
    // `r#"x.unwrap()"#` is a string literal — forbidden-scan must not
    // flag the `unwrap()` that lives inside raw-string content.
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/src/lib.rs"),
        r##"pub fn demo() {
    let s = r#"x.unwrap()"#;
    let t = "also safe";
}
"##,
    )?;
    let output =
        Command::new(env!("CARGO_BIN_EXE_forbidden-scan")).current_dir(fixture.path()).output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "raw string should not trigger forbidden scan; stderr was: {stderr}"
    );
    assert!(stderr.contains("NoViolationFound"), "expected clean pass; stderr was: {stderr}");
    Ok(())
}

#[test]
fn forbidden_scan_skips_unwrap_inside_byte_raw_string() -> TestResult {
    // `br#"..."#` raw string must also suppress forbidden tokens.
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/src/lib.rs"),
        r##"pub fn demo() {
    let b = br#"Result::unwrap()"#;
}
"##,
    )?;
    let output =
        Command::new(env!("CARGO_BIN_EXE_forbidden-scan")).current_dir(fixture.path()).output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "byte raw string should not trigger forbidden scan; stderr was: {stderr}"
    );
    Ok(())
}

#[test]
fn panic_surface_skips_assert_inside_raw_string() -> TestResult {
    // `assert!(false)` inside a raw string literal must not be flagged
    // by the panic-surface lane.
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/src/lib.rs"),
        r##"pub fn demo() {
    let s = r#"assert!(false)"#;
}
"##,
    )?;
    let output = run_from_member(env!("CARGO_BIN_EXE_check-panic-surface"), &fixture)?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "raw string assert! should not trigger panic surface; stderr was: {stderr}"
    );
    Ok(())
}

#[test]
fn panic_surface_skips_assert_inside_byte_raw_string() -> TestResult {
    // `assert!(true)` inside a `br#"..."#` must not be flagged.
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/src/lib.rs"),
        r##"pub fn demo() {
    let b = br#"assert_eq!(1, 2)"#;
}
"##,
    )?;
    let output = run_from_member(env!("CARGO_BIN_EXE_check-panic-surface"), &fixture)?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "byte raw string assert_eq! should not trigger panic surface; stderr was: {stderr}"
    );
    Ok(())
}

#[test]
fn panic_surface_flags_assert_outside_raw_string() -> TestResult {
    // Sanity: assert! OUTSIDE any raw string must still be flagged.
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/src/lib.rs"),
        r##"pub fn demo() {
    let s = r#"safe string"#;
    assert!(false);
}
"##,
    )?;
    let output = run_from_member(env!("CARGO_BIN_EXE_check-panic-surface"), &fixture)?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "real assert! should still be flagged; stderr was: {stderr}");
    assert!(stderr.contains("lib.rs:3"), "expected finding at lib.rs:3; stderr was: {stderr}");
    Ok(())
}

#[test]
fn check_ignored_fallible_results_skips_drop_inside_raw_string() -> TestResult {
    // `drop(x)` inside a raw string must not trigger the discard-001 rule.
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/src/lib.rs"),
        r##"pub fn demo() {
    let s = r#"drop(some_result)"#;
}
"##,
    )?;
    let output = run_from_member(env!("CARGO_BIN_EXE_check-ignored-fallible-results"), &fixture)?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "raw string drop() should not trigger discard rule; stderr was: {stderr}"
    );
    Ok(())
}

#[test]
fn check_hot_cold_forbidden_apis_skips_println_inside_raw_string() -> TestResult {
    // `println!("hello")` inside a raw string must not trigger
    // the hot-cold forbidden-apis lane.
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/titania-core/src/lib.rs"),
        r##"pub fn demo() {
    let s = r#"println!("hello")"#;
}
"##,
    )?;
    let output = run_from_member(env!("CARGO_BIN_EXE_check-hot-cold-forbidden-apis"), &fixture)?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "raw string println! should not trigger hot-cold scan; stderr was: {stderr}"
    );
    Ok(())
}

#[test]
fn check_hot_cold_forbidden_apis_skips_dbg_inside_raw_string() -> TestResult {
    // `dbg!(x)` inside a raw string must not trigger.
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/titania-core/src/lib.rs"),
        r##"pub fn demo() {
    let s = r#"dbg!(value)"#;
}
"##,
    )?;
    let output = run_from_member(env!("CARGO_BIN_EXE_check-hot-cold-forbidden-apis"), &fixture)?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "raw string dbg! should not trigger hot-cold scan; stderr was: {stderr}"
    );
    Ok(())
}

#[test]
fn forbidden_scan_flags_real_unwrap_outside_raw_string() -> TestResult {
    // Sanity: real .unwrap() calls outside raw strings must still be flagged.
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/src/lib.rs"),
        r##"pub fn demo() {
    let s = r#"safe string"#;
    let x = some_result.unwrap();
}
"##,
    )?;
    let output =
        Command::new(env!("CARGO_BIN_EXE_forbidden-scan")).current_dir(fixture.path()).output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "real unwrap() should still be flagged; stderr was: {stderr}"
    );
    assert!(stderr.contains("`unwrap`"), "expected forbidden token; stderr was: {stderr}");
    Ok(())
}

#[test]
fn raw_string_with_multi_hash_delimiters_not_flagged() -> TestResult {
    // `r##"..."##` with multiple `#` must also blank correctly.
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/src/lib.rs"),
        r###"pub fn demo() {
    let s = r##"unwrap(); panic!()"##;
}
"###,
    )?;
    let output =
        Command::new(env!("CARGO_BIN_EXE_forbidden-scan")).current_dir(fixture.path()).output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "multi-hash raw string should not trigger forbidden scan; stderr was: {stderr}"
    );
    Ok(())
}

#[test]
fn multi_line_raw_string_forbidden_scan_skips_content() -> TestResult {
    // A raw string that opens on one line and closes on the next must blank
    // content on BOTH lines. The shared SourceLine parser discards
    // RawString state in `finish()`, leaking the body of multi-line raw
    // strings as code. Before the fix, `unwrap()` on the second line
    // appears as real code and gets flagged.
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/src/lib.rs"),
        "pub fn demo() {\n\
         let s = r#\"\n\
         some unwrap()\n\
         \x22#;\n\
         }\n",
    )?;
    let output =
        Command::new(env!("CARGO_BIN_EXE_forbidden-scan")).current_dir(fixture.path()).output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "multi-line raw string should not trigger forbidden scan; stderr was: {stderr}"
    );
    assert!(
        stderr.contains("NoViolationFound"),
        "expected clean pass for multi-line raw string; stderr was: {stderr}"
    );
    Ok(())
}

#[test]
fn multi_line_raw_string_panic_surface_skips_assert() -> TestResult {
    // check-panic-surface uses the shared SourceLine parser.
    // Multi-line raw string content must not trigger panic! findings.
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/src/lib.rs"),
        "pub fn demo() {\n\
         let s = r#\"\n\
         panic!(\x22inside raw\x22)\n\
         \x22#;\n\
         }\n",
    )?;
    let output = run_from_member(env!("CARGO_BIN_EXE_check-panic-surface"), &fixture)?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "multi-line raw string panic! should not trigger; stderr was: {stderr}"
    );
    Ok(())
}

#[test]
fn multi_line_raw_string_ignored_fallible_results_skips_drop() -> TestResult {
    // The shared SourceLine parser blanks raw-string content across lines.
    // `drop(fallible_result)` inside `r#"...\n..."#` must not be flagged.
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/src/lib.rs"),
        "pub fn demo() {\n\
         let s = r#\"\n\
         drop(fallible_result)\n\
         \x22#;\n\
         }\n",
    )?;
    let output = run_from_member(env!("CARGO_BIN_EXE_check-ignored-fallible-results"), &fixture)?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "multi-line raw string drop() should not trigger discard rule; stderr was: {stderr}"
    );
    Ok(())
}

#[test]
fn multi_line_raw_string_hot_cold_skips_println() -> TestResult {
    // The shared SourceLine parser blanks raw-string content across lines.
    // `println!(...)` inside `r#"...\n..."#` must not be flagged.
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/titania-core/src/lib.rs"),
        "pub fn demo() {\n\
         let s = r#\"\n\
         println!(\x22hot code\x22)\n\
         \x22#;\n\
         }\n",
    )?;
    let output = run_from_member(env!("CARGO_BIN_EXE_check-hot-cold-forbidden-apis"), &fixture)?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "multi-line raw string println! should not trigger hot-cold scan; stderr was: {stderr}"
    );
    Ok(())
}

#[test]
fn raw_string_close_delimiter_not_flagged_in_code_after() -> TestResult {
    // Code immediately after a raw-string close delimiter must be scanned.
    // `r##"safe"##; real.unwrap();` — the unwrap() is outside the raw string
    // and should be flagged. This verifies the close-delimiter matching works.
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/src/lib.rs"),
        r####"pub fn demo() {
    let s = r##"safe string"##;
    let x = some_result.unwrap();
}
"####,
    )?;
    let output =
        Command::new(env!("CARGO_BIN_EXE_forbidden-scan")).current_dir(fixture.path()).output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "unwrap() after multi-hash raw string close should be flagged; stderr was: {stderr}"
    );
    assert!(
        stderr.contains("`unwrap`"),
        "expected forbidden token unwrap in findings; stderr was: {stderr}"
    );
    Ok(())
}

#[test]
fn ignored_fallible_results_skips_drop_inside_single_line_raw() -> TestResult {
    // Single-line raw strings containing forbidden-looking calls must not
    // trigger ignored-fallible-results. This is a desired-behavior guard;
    // multi-line and interior-quote tests below catch the current duplicate
    // parser's raw-string state bugs.
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/src/lib.rs"),
        r##"pub fn demo() {
    let s = r#"drop(fallible_result)"#;
}
"##,
    )?;
    let output = run_from_member(env!("CARGO_BIN_EXE_check-ignored-fallible-results"), &fixture)?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "duplicate parser correctly consumes raw string content on single line; stderr was: {stderr}"
    );
    Ok(())
}

#[test]
fn hot_cold_forbidden_apis_skips_println_inside_single_line_raw() -> TestResult {
    // Single-line raw strings containing forbidden-looking calls must not
    // trigger hot-cold forbidden API detection. This is a desired-behavior
    // guard; multi-line and interior-quote tests below catch the current
    // duplicate parser's raw-string state bugs.
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/titania-core/src/lib.rs"),
        r##"pub fn demo() {
    let s = r#"println!("hello")"#;
}
"##,
    )?;
    let output = run_from_member(env!("CARGO_BIN_EXE_check-hot-cold-forbidden-apis"), &fixture)?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Desired behavior: forbidden-looking text inside the raw string is ignored.
    assert!(
        output.status.success(),
        "single-line raw string println! should not be flagged (content consumed); stderr was: {stderr}"
    );
    Ok(())
}
#[test]
fn ignored_fallible_results_with_interior_quote_raw_string() -> TestResult {
    // Raw string with interior quotes: `r#"quoted " drop(write_result)"#`.
    // The interior `"` must not break raw-string detection.
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/example/src/lib.rs"),
        r##"pub fn demo() {
    let s = r#"quoted " drop(write_result)"#;
}
"##,
    )?;
    let output = run_from_member(env!("CARGO_BIN_EXE_check-ignored-fallible-results"), &fixture)?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "raw string with interior quotes should not trigger discard rule; stderr was: {stderr}"
    );
    Ok(())
}

#[test]
fn hot_cold_forbidden_apis_with_interior_quote_raw_string() -> TestResult {
    // Raw string with an interior quote before the forbidden call:
    // `r#"quoted " println!("hello")"#`.
    let fixture = workspace_fixture()?;
    write_file(
        fixture.path().join("crates/titania-core/src/lib.rs"),
        r##"pub fn demo() {
    let s = r#"quoted " println!("hello")"#;
}
"##,
    )?;
    let output = run_from_member(env!("CARGO_BIN_EXE_check-hot-cold-forbidden-apis"), &fixture)?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "raw string with interior quotes should not trigger hot-cold scan; stderr was: {stderr}"
    );
    Ok(())
}
