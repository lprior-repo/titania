use titania_core::{RepairHint, TextRange};

#[test]
fn patch_accepts_zero_width_range_at_file_start() {
    let range = TextRange::new(0, 0).unwrap();
    let hint = RepairHint::patch("f.rs".to_owned(), range, "header".to_owned());
    assert!(hint.is_auto_applicable(), "Patch hints must remain auto-applicable");
}

#[test]
fn patch_accepts_zero_width_range_in_middle() {
    let range = TextRange::new(5, 5).unwrap();
    let hint = RepairHint::patch("f.rs".to_owned(), range, "inserted".to_owned());
    assert!(hint.is_auto_applicable());
}

#[test]
fn patch_accepts_zero_width_range_at_known_offset() {
    let range = TextRange::new(1_024, 1_024).unwrap();
    let hint = RepairHint::patch("src/lib.rs".to_owned(), range, "x".to_owned());
    assert!(hint.is_auto_applicable());
}

#[test]
fn patch_accepts_zero_width_range_with_empty_replacement() {
    let range = TextRange::new(0, 0).unwrap();
    let hint = RepairHint::patch("f.rs".to_owned(), range, String::new());
    assert!(hint.is_auto_applicable());
}

#[test]
fn patch_accepts_zero_width_range_with_unicode_replacement() {
    let range = TextRange::new(7, 7).unwrap();
    let hint = RepairHint::patch("f.rs".to_owned(), range, "—✓—".to_owned());
    assert!(hint.is_auto_applicable());
}

#[test]
fn patch_accepts_non_zero_width_range() {
    let range = TextRange::new(10, 20).unwrap();
    let hint = RepairHint::patch("f.rs".to_owned(), range, "replacement".to_owned());
    assert!(hint.is_auto_applicable());
}

#[test]
fn patch_accepts_single_byte_range() {
    let range = TextRange::new(3, 4).unwrap();
    let hint = RepairHint::patch("f.rs".to_owned(), range, "X".to_owned());
    assert!(hint.is_auto_applicable());
}

#[test]
fn text_range_rejects_inverted_range() {
    let inverted = TextRange::new(10, 5);
    assert!(inverted.is_err(), "inverted range must be rejected by TextRange::new");
    let equal_zero_width = TextRange::new(0, 0);
    assert!(equal_zero_width.is_ok(), "zero-width range must be accepted by TextRange::new");
}

#[test]
fn patch_accepts_zero_width_at_every_boundary_in_u32() {
    for offset in [0u32, 1, 16, 256, 65_535, 1_000_000] {
        let range = TextRange::new(offset, offset).unwrap();
        let _hint = RepairHint::patch("f.rs".to_owned(), range, "x".to_owned());
    }
}

#[test]
fn patch_wire_deserialize_accepts_zero_width() {
    let json =
        r#"{"Patch":{"file":"f.rs","range":{"start_byte":3,"end_byte":3},"replacement":"INS"}}"#;
    let parsed: RepairHint = serde_json::from_str(json)
        .expect("wire deserialization must accept zero-width insertion patches");
    assert!(parsed.is_auto_applicable());
}

#[test]
fn patch_wire_serialize_round_trip_preserves_zero_width() {
    let range = TextRange::new(8, 8).unwrap();
    let original = RepairHint::patch("f.rs".to_owned(), range, "INS".to_owned());
    let json = serde_json::to_string(&original).expect("serialize");
    let parsed: RepairHint = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(original, parsed, "zero-width patch must round-trip through wire format");
}

#[test]
fn non_patch_variants_still_construct() {
    drop(RepairHint::use_iterator_pipeline("use iter()".to_owned()));
    drop(RepairHint::flatten_nesting("reduce nesting".to_owned()));
    drop(RepairHint::use_checked_arithmetic("add".to_owned()));
    drop(RepairHint::remove_allow_attribute("clippy::unwrap_used".to_owned()));
    drop(RepairHint::replace_dependency("a".to_owned(), "b".to_owned()));
    drop(RepairHint::requires_human_review("manual".to_owned()));
}
