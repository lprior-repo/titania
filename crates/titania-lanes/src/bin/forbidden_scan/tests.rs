use std::{error::Error, fs};

use titania_lanes::RuleId;

use super::{ForbiddenToken, collect_source_files, default_forbidden_set, scan_file};

fn macro_token(name: &str) -> Result<ForbiddenToken, String> {
    ForbiddenToken::parse(name).ok_or_else(|| format!("token parse failed for {name}"))
}

#[test]
fn macro_token_matches_panic_bang() -> Result<(), String> {
    let token = macro_token("panic!")?;
    assert!(token.is_present_in("panic!(\"boom\")"));
    assert!(token.is_present_in("let _ = panic!();"));
    Ok(())
}

#[test]
fn macro_token_rejects_identifier_prefixed_match() -> Result<(), String> {
    // `mypanic!` must not be flagged as `panic!`.
    let token = macro_token("panic!")?;
    assert!(!token.is_present_in("mypanic!()"));
    Ok(())
}

#[test]
fn method_token_matches_dot_receiver() -> Result<(), String> {
    let token = macro_token("unwrap")?;
    assert!(token.is_present_in("x.unwrap()"));
    // `unwrap_or_default` is a different method, not `unwrap`.
    assert!(!token.is_present_in("x.unwrap_or_default()"));
    Ok(())
}

#[test]
fn method_token_matches_double_colon_receiver() -> Result<(), String> {
    let token = macro_token("unwrap")?;
    assert!(token.is_present_in("Result::unwrap(r)"));
    Ok(())
}

#[test]
fn method_token_matches_expect_with_message() -> Result<(), String> {
    // Regression: the old plain-substring matcher missed this because
    // `.expect("msg")` never contains the literal `expect()` token.
    let token = macro_token("expect")?;
    assert!(token.is_present_in("fs::read_to_string(\"/tmp/x\").expect(\"boom\")"));
    Ok(())
}

#[test]
fn method_token_rejects_identifier_prefixed_match() -> Result<(), String> {
    // `myexpect()` must not be flagged as the `expect` method.
    let token = macro_token("expect")?;
    assert!(!token.is_present_in("myexpect()"));
    assert!(!token.is_present_in("myexpect"));
    Ok(())
}

#[test]
fn method_token_requires_open_paren() -> Result<(), String> {
    let token = macro_token("unwrap")?;
    // No `(` after the name means it's just an identifier in scope.
    assert!(!token.is_present_in("let unwrap = 1;"));
    assert!(!token.is_present_in("x.unwrap"));
    Ok(())
}

#[test]
fn empty_token_string_is_rejected() {
    assert!(ForbiddenToken::parse("").is_none());
    assert!(ForbiddenToken::parse("   ").is_none());
}

#[test]
fn dbg_macro_token_matches_bang_form() -> Result<(), String> {
    let token = macro_token("dbg!")?;
    assert!(token.is_present_in("dbg!(x)"));
    Ok(())
}

#[test]
fn production_tests_rs_is_scanned_under_tests_checkout_path() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("tests").join("checkout");
    let file = root.join("crates/example/src/tests.rs");
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&file, "pub fn bad() { panic!(\"boom\"); }\n")?;

    let forbidden = default_forbidden_set();
    let rule = RuleId::new("FORBIDDEN_001")?;
    let findings = collect_source_files(&root)
        .iter()
        .flat_map(|source| scan_file(&root, source, &forbidden, &rule))
        .collect::<Vec<_>>();

    assert!(findings.iter().any(|finding| {
        finding.path() == "crates/example/src/tests.rs"
            && finding.rule().as_str() == "FORBIDDEN_001"
    }));
    Ok(())
}
