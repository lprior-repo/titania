use super::ForbiddenToken;

fn macro_token(name: &str) -> Option<ForbiddenToken> {
    ForbiddenToken::parse(name)
}

fn must_some<T>(opt: Option<T>, msg: &str) -> Result<T, Box<dyn std::error::Error>> {
    opt.ok_or_else(|| msg.to_string()).map_err(|e| e.into())
}

#[test]
fn macro_token_matches_panic_bang() -> Result<(), Box<dyn std::error::Error>> {
    let token = must_some(macro_token("panic!"), "parse")?;
    assert!(token.is_present_in("panic!(\"boom\")"));
    assert!(token.is_present_in("let _ = panic!();"));
    Ok(())
}

#[test]
fn macro_token_rejects_identifier_prefixed_match() -> Result<(), Box<dyn std::error::Error>> {
    // `mypanic!` must not be flagged as `panic!`.
    let token = must_some(macro_token("panic!"), "parse")?;
    assert!(!token.is_present_in("mypanic!()"));
    Ok(())
}

#[test]
fn method_token_matches_dot_receiver() -> Result<(), Box<dyn std::error::Error>> {
    let token = must_some(macro_token("unwrap"), "parse")?;
    assert!(token.is_present_in("x.unwrap()"));
    // `unwrap_or_default` is a different method, not `unwrap`.
    assert!(!token.is_present_in("x.unwrap_or_default()"));
    Ok(())
}

#[test]
fn method_token_matches_double_colon_receiver() -> Result<(), Box<dyn std::error::Error>> {
    let token = must_some(macro_token("unwrap"), "parse")?;
    assert!(token.is_present_in("Result::unwrap(r)"));
    Ok(())
}

#[test]
fn method_token_matches_expect_with_message() -> Result<(), Box<dyn std::error::Error>> {
    let token = must_some(macro_token("expect"), "parse")?;
    // because `.expect("msg")` never contains the literal `expect()` token.
    assert!(token.is_present_in("fs::read_to_string(\"/tmp/x\").expect(\"boom\")"));
    Ok(())
}

#[test]
fn method_token_rejects_identifier_prefixed_match() -> Result<(), Box<dyn std::error::Error>> {
    let token = must_some(macro_token("expect"), "parse")?;
    assert!(!token.is_present_in("myexpect()"));
    assert!(!token.is_present_in("myexpect"));
    Ok(())
}

#[test]
fn method_token_requires_open_paren() -> Result<(), Box<dyn std::error::Error>> {
    let token = must_some(macro_token("unwrap"), "parse")?;
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
fn dbg_macro_token_matches_bang_form() -> Result<(), Box<dyn std::error::Error>> {
    let token = must_some(macro_token("dbg!"), "parse")?;
    assert!(token.is_present_in("dbg!(x)"));
    Ok(())
}
