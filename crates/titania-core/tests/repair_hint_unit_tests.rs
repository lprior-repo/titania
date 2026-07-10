//! Unit tests for [`RepairHint`] zero-width patch semantics.
//!
//! These tests live in `tests/` so source-target lint and Dylint gates
//! stay strict while test assertions remain clear. They exercise the
//! `pub` API surface end-to-end.
//!
//! Contract reference: v1 §10 — a [`RepairHint::Patch`] range is a
//! half-open `[start_byte, end_byte)` span. Zero-width ranges are
//! *insertion patches*: they insert `replacement` at `start_byte`
//! without consuming any bytes of the source file. They are valid
//! and MUST NOT be rejected by [`RepairHint::patch`].
//!
//! Range *bounds* (`start_byte <= end_byte`) are still enforced by
//! [`TextRange::new`] itself; this file verifies that the
//! `RepairHint` layer does not add a redundant empty-range rejection
//! on top of that.

use titania_core::{RepairHint, TextRange};

// ============================================================================
// Zero-width patches are valid (insertion semantics).
// ============================================================================

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
    // start == end == 0, empty replacement is still a valid (no-op insertion).
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

// ============================================================================
// Non-zero-width patches still work (regression coverage).
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

// ============================================================================
// Range-bound enforcement is preserved (TextRange is the only gate).
// ============================================================================

#[test]
fn text_range_rejects_inverted_range() {
    // Range bounds are preserved by TextRange::new itself; this is
    // the only gate. RepairHint does not weaken or duplicate it.
    let inverted = TextRange::new(10, 5);
    assert!(inverted.is_err(), "inverted range must be rejected by TextRange::new");
    let equal_zero_width = TextRange::new(0, 0);
    assert!(equal_zero_width.is_ok(), "zero-width range must be accepted by TextRange::new");
}

#[test]
fn patch_accepts_zero_width_at_every_boundary_in_u32() {
    // Spot-check several zero-width offsets to confirm the constructor
    // does not introduce a hidden non-empty precondition.
    for offset in [0u32, 1, 16, 256, 65_535, 1_000_000] {
        let range = TextRange::new(offset, offset).unwrap();
        let _hint = RepairHint::patch("f.rs".to_owned(), range, "x".to_owned());
    }
}

// ============================================================================
// Wire deserialization accepts zero-width patches too.
// ============================================================================

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

// ============================================================================
// Non-patch variants are unaffected by the zero-width change.

#[test]
fn non_patch_variants_still_construct() {
    drop(RepairHint::use_iterator_pipeline("use iter()".to_owned()));
    drop(RepairHint::flatten_nesting("reduce nesting".to_owned()));
    drop(RepairHint::use_checked_arithmetic("add".to_owned()));
    drop(RepairHint::remove_allow_attribute("clippy::unwrap_used".to_owned()));
    drop(RepairHint::replace_dependency("a".to_owned(), "b".to_owned()));
    drop(RepairHint::requires_human_review("manual".to_owned()));
}
