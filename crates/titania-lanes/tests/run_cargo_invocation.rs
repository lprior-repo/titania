use titania_lanes::policy_scan::invocation::{InvocationError, validate_extra_args};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn args(words: &[&str]) -> Vec<String> {
    words.iter().map(ToString::to_string).collect()
}

#[test]
fn empty_extra_args_are_accepted() -> TestResult {
    assert_eq!(validate_extra_args(&[]), Ok(()));
    Ok(())
}

#[test]
fn bare_cap_lints_flag_is_rejected() -> TestResult {
    let result = validate_extra_args(&args(&["--cap-lints", "allow"]));
    assert_eq!(
        result,
        Err(InvocationError::CapLints { arg: String::from("--cap-lints") }),
        "bare --cap-lints must be rejected with the token named",
    );
    Ok(())
}

#[test]
fn equals_form_is_rejected() -> TestResult {
    let result = validate_extra_args(&args(&["--cap-lints=allow"]));
    assert_eq!(
        result,
        Err(InvocationError::CapLints { arg: String::from("--cap-lints=allow") }),
        "joined --cap-lints=allow must be rejected with the token named",
    );
    Ok(())
}

#[test]
fn cap_lints_after_separator_is_rejected() -> TestResult {
    let result = validate_extra_args(&args(&["--", "--cap-lints", "allow"]));
    assert_eq!(
        result,
        Err(InvocationError::CapLints { arg: String::from("--cap-lints") }),
        "a ceiling after `--` still reaches the compiler and must be rejected",
    );
    Ok(())
}

#[test]
fn warn_level_is_rejected() -> TestResult {
    let result = validate_extra_args(&args(&["--cap-lints=warn"]));
    assert_eq!(
        result,
        Err(InvocationError::CapLints { arg: String::from("--cap-lints=warn") }),
        "any ceiling level weakens mandatory denies, not just `allow`",
    );
    Ok(())
}

#[test]
fn benign_flags_are_accepted() -> TestResult {
    assert_eq!(validate_extra_args(&args(&["--offline"])), Ok(()));
    assert_eq!(validate_extra_args(&args(&["--", "--test-threads=1"])), Ok(()));
    Ok(())
}

#[test]
fn lookalike_flags_are_accepted() -> TestResult {
    assert_eq!(validate_extra_args(&args(&["--cap-lints-allow"])), Ok(()));
    assert_eq!(validate_extra_args(&args(&["cap-lints"])), Ok(()));
    Ok(())
}

#[test]
fn rejection_names_offending_token() -> TestResult {
    let error = validate_extra_args(&args(&["--cap-lints=allow"]))
        .expect_err("expected rejection for --cap-lints=allow");
    let message = error.to_string();
    assert!(message.contains("--cap-lints=allow"), "rejection must name the token: {message}",);
    assert!(message.contains("cap"), "rejection must state the ceiling consequence: {message}",);
    Ok(())
}
