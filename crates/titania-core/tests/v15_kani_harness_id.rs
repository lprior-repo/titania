//! v1.5 contract tests for `KaniHarnessId` validation.
//!
//! Mirrors the spec rule `^[a-zA-Z][a-zA-Z0-9_]*$` (max length 96). Mixed
//! case and underscore-less ids are both accepted; an underscore is no
//! longer required. Each test exercises one rejection or acceptance axis.

use titania_core::{KANI_HARNESS_ID_MAX_LEN, KaniHarnessId, KaniHarnessIdError};

#[test]
fn accepts_uppercase_with_underscore() {
    let id = KaniHarnessId::new("FOO_BAR");
    assert!(matches!(id, Ok(ref v) if v.as_str() == "FOO_BAR"));
}

#[test]
fn accepts_mixed_case() {
    let id = KaniHarnessId::new("Foo_Bar");
    assert!(matches!(id, Ok(ref v) if v.as_str() == "Foo_Bar"));
}

#[test]
fn accepts_lowercase() {
    let id = KaniHarnessId::new("foo_bar");
    assert!(matches!(id, Ok(ref v) if v.as_str() == "foo_bar"));
}

#[test]
fn accepts_no_underscore() {
    let id = KaniHarnessId::new("FooBar");
    assert!(matches!(id, Ok(ref v) if v.as_str() == "FooBar"));
}

#[test]
fn rejects_empty() {
    let result = KaniHarnessId::new("");
    assert_eq!(result.unwrap_err(), KaniHarnessIdError::Empty);
}

#[test]
fn rejects_leading_digit() {
    let result = KaniHarnessId::new("1FOO_BAR");
    assert_eq!(result.unwrap_err(), KaniHarnessIdError::LeadingNonLetter { byte: b'1' });
}

#[test]
fn rejects_leading_underscore() {
    // A literal underscore is a legal body byte but illegal as the first
    // byte — the spec requires an ASCII letter first.
    let result = KaniHarnessId::new("_FOO_BAR");
    assert_eq!(result.unwrap_err(), KaniHarnessIdError::LeadingNonLetter { byte: b'_' });
}

#[test]
fn rejects_leading_only_underscore() {
    let result = KaniHarnessId::new("_");
    assert_eq!(result.unwrap_err(), KaniHarnessIdError::LeadingNonLetter { byte: b'_' });
}

#[test]
fn rejects_leading_non_ascii_letter() {
    // U+00C0 (`À`) starts a UTF-8 sequence with byte 0xC3; the Kani id
    // contract requires an ASCII letter first, so the lead byte 0xC3 must
    // surface as `LeadingNonLetter` — never as `NotAscii` with offset 0.
    let result = KaniHarnessId::new("ÀFOO");
    assert_eq!(result.unwrap_err(), KaniHarnessIdError::LeadingNonLetter { byte: 0xC3 });
}

#[test]
fn rejects_leading_symbol() {
    let result = KaniHarnessId::new("@foo");
    assert_eq!(result.unwrap_err(), KaniHarnessIdError::LeadingNonLetter { byte: b'@' });
}

#[test]
fn rejects_dot_after_first_letter() {
    // A `.` is non-letter, non-digit, non-underscore and lands at offset ≥ 1,
    // so it surfaces as `NotAscii` (not `LeadingNonLetter`).
    let result = KaniHarnessId::new("FOO_BAR.X");
    assert!(
        matches!(result, Err(KaniHarnessIdError::NotAscii { byte, offset }) if byte == b'.' && offset == 7),
        "got {result:?}"
    );
}

#[test]
fn rejects_inner_non_ascii() {
    // FOO_B is 5 ASCII bytes; the UTF-8 encoding of U+00C4 (`Ä`) starts at
    // byte offset 5 with the lead byte 0xC3.
    let result = KaniHarnessId::new("FOO_BÄR");
    assert!(
        matches!(result, Err(KaniHarnessIdError::NotAscii { byte, offset }) if byte == 0xC3 && offset == 5),
        "got {result:?}"
    );
}

#[test]
fn rejects_over_max_len() {
    let too_long = "A".repeat(KANI_HARNESS_ID_MAX_LEN + 1);
    let result = KaniHarnessId::new(&too_long);
    assert!(matches!(result, Err(KaniHarnessIdError::TooLong(_))));
}

#[test]
fn accepts_max_len_boundary() {
    let mut max_len = "A".repeat(KANI_HARNESS_ID_MAX_LEN - 1);
    max_len.push('Z');
    let id = KaniHarnessId::new(&max_len);
    assert!(matches!(id, Ok(ref v) if v.as_str() == max_len));
}

#[test]
fn from_str_parses_canonical_id() {
    let parsed: KaniHarnessId = "FooBar".parse().unwrap();
    assert_eq!(parsed.as_str(), "FooBar");
}

#[test]
fn from_str_rejects_lowercase_dash() {
    let err = "Foo-Bar".parse::<KaniHarnessId>().unwrap_err();
    assert!(matches!(err, KaniHarnessIdError::NotAscii { .. }));
}

#[test]
fn display_emits_inner_string() {
    let id = KaniHarnessId::new("Foo").unwrap();
    assert_eq!(id.to_string(), "Foo");
}

// ---- Serde exact wire forms -------------------------------------------

#[test]
fn serde_serializes_to_exact_wire_form() {
    // The wire form is the inner string verbatim; no transformation,
    // escaping, or quoting beyond JSON's normal string rules.
    let id = KaniHarnessId::new("FooBar").unwrap();
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, "\"FooBar\"");

    let id = KaniHarnessId::new("FOO_BAR").unwrap();
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, "\"FOO_BAR\"");

    let id = KaniHarnessId::new("foo_bar").unwrap();
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, "\"foo_bar\"");
}

#[test]
fn serde_round_trips_max_len_identifier() {
    let mut id_str = "A".repeat(KANI_HARNESS_ID_MAX_LEN - 1);
    id_str.push('Z');
    assert_eq!(id_str.len(), KANI_HARNESS_ID_MAX_LEN);
    let id = KaniHarnessId::new(&id_str).unwrap();
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, format!("\"{id_str}\""));
    let parsed: KaniHarnessId = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.as_str(), id_str);
}

#[test]
fn serde_round_trips_underscore_only_body() {
    let id_str = "A_";
    let id = KaniHarnessId::new(id_str).unwrap();
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, "\"A_\"");
    let parsed: KaniHarnessId = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.as_str(), id_str);
}
