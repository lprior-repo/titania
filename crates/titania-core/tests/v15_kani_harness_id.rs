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
    let result = KaniHarnessId::new("FOO_BAR.X");
    assert!(
        matches!(result, Err(KaniHarnessIdError::NotAscii { byte, offset }) if byte == b'.' && offset == 7),
        "got {result:?}"
    );
}

#[test]
fn rejects_inner_non_ascii() {
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

#[test]
fn serde_serializes_to_exact_wire_form() {
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

#[test]
fn body_alphabet_matches_std_byte_classes_for_all_256_bytes() {
    for byte in 0..=255u8 {
        let expected = byte.is_ascii_alphabetic() || byte.is_ascii_digit() || byte == b'_';
        let bytes = [b'A', byte];
        match core::str::from_utf8(&bytes) {
            Ok(candidate) => match KaniHarnessId::new(candidate) {
                Ok(identifier) => {
                    assert!(expected, "byte {byte} accepted but outside std classes");
                    assert_eq!(
                        identifier.as_str(),
                        candidate,
                        "byte {byte} must be preserved verbatim"
                    );
                }
                Err(error) => {
                    assert!(!expected, "byte {byte} rejected but inside std classes");
                    assert_eq!(
                        error,
                        KaniHarnessIdError::NotAscii { byte, offset: 1 },
                        "body rejection must name the byte and offset"
                    );
                }
            },
            Err(_) => assert!(
                !expected,
                "byte {byte} is not valid UTF-8 and must fall outside the alphabet"
            ),
        }
    }
}

#[test]
fn lengths_around_maximum_are_accepted_or_rejected_exactly() {
    for length in [33, 64, 96] {
        let id = format!("A{}", "a".repeat(length - 1));
        assert_eq!(id.len(), length);
        assert_eq!(KaniHarnessId::new(&id).unwrap().as_str(), id);
    }
    let too_long = "A".repeat(97);
    assert_eq!(KaniHarnessId::new(&too_long).unwrap_err(), KaniHarnessIdError::TooLong(97));
}

#[test]
fn multibyte_body_byte_is_rejected_with_byte_offset() {
    let error = KaniHarnessId::new("Aé").unwrap_err();
    assert_eq!(error, KaniHarnessIdError::NotAscii { byte: 0xC3, offset: 1 });
}

#[test]
fn invalid_second_body_byte_reports_offset_two() {
    let error = KaniHarnessId::new("AB ").unwrap_err();
    assert_eq!(error, KaniHarnessIdError::NotAscii { byte: b' ', offset: 2 });
}
