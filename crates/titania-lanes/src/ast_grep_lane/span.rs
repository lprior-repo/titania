//! Pure byte → (line, codepoint-column) calculus for ast-grep matches.
//!
//! This module is the Calculation layer for span reporting (v1-spec §10):
//! it translates ast-grep byte offsets — which are UTF-8 byte positions
//! (see `ast_grep_core::Node::range`: "byte offsets of start and end") —
//! into the spec coordinate system:
//!
//! - Lines are 1-based at the `Location` boundary (`line_col` itself returns
//!   the 0-based line index; the lane adds 1).
//! - Columns are 0-based Unicode scalar values (Rust `char`s), counted from
//!   the first character of the line. A multi-byte leading rune (e.g. `→`
//!   U+2192 or an emoji) contributes exactly ONE column, not its UTF-8 byte
//!   length.
//!
//! Everything here is a deterministic Calculation: no I/O, no globals, no
//! panics, no `unsafe`, no `as` casts, no string slicing (the only source
//! access is `str::get`, which returns `Option`).

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::unreachable)]
#![deny(clippy::indexing_slicing)]
#![deny(clippy::string_slice)]
#![deny(clippy::get_unwrap)]
#![deny(clippy::arithmetic_side_effects)]
#![deny(clippy::as_conversions)]
#![forbid(unsafe_code)]

/// Byte range of a detected ast-grep match (half-open `[start_byte, end_byte)`).
///
/// Both fields are UTF-8 byte offsets into the source string, matching the
/// contract of `ast_grep_core::Node::range`. The lane converts these to a
/// `Location::Span` (codepoint columns) and, where a patch is appropriate,
/// a `titania_core::TextRange` (byte offsets) — see `ast_grep_lane::finding`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MatchSite {
    /// Inclusive start byte offset of the matched node.
    pub(super) start_byte: usize,
    /// Exclusive end byte offset of the matched node.
    pub(super) end_byte: usize,
}

impl MatchSite {
    /// Build a match site from an ast-grep byte range.
    ///
    /// `range.end >= range.start` is guaranteed by ast-grep's `Node::range`.
    #[must_use]
    pub(super) const fn from_range(range: std::ops::Range<usize>) -> Self {
        Self { start_byte: range.start, end_byte: range.end }
    }
}

/// Anchoring site for file-level findings produced by the legacy string
/// detectors (inline-suppression comment, architecture import rules).
///
/// These detectors do not identify a precise AST token — they only report
/// that a pattern is present somewhere in the file. The byte range `[0, 1)`
/// reproduces the pre-H3 wire output (`line 1, col 0..1`) for any non-empty
/// source: byte 0 and byte 1 both fall on line 0, giving a one-rune span at
/// the file head. It is never the site of a real ast-grep match.
pub(super) const FILE_LEVEL_SITE: MatchSite = MatchSite { start_byte: 0, end_byte: 1 };

/// Compute the byte offset of the start of every source line.
///
/// Line 0 starts at byte 0; each subsequent entry is the byte offset
/// immediately following a `\n`. The returned vector always has at least one
/// element so `line_at_byte` can never index out of bounds.
pub(super) fn line_offsets(source: &str) -> Vec<usize> {
    let mut offsets: Vec<usize> = vec![0];
    offsets.extend(source.match_indices('\n').map(|(i, _)| i.saturating_add(1)));
    offsets
}

/// 0-based line index containing `byte`, via a binary search over line-start
/// `offsets`.
///
/// `offsets` must be non-empty and sorted ascending; [`line_offsets`] is the
/// canonical producer. Returns `0` for a `byte` at or before the first line
/// start.
pub(super) fn line_at_byte(offsets: &[usize], byte: usize) -> usize {
    offsets.partition_point(|&offset| offset <= byte).saturating_sub(1)
}

/// 0-based Unicode-scalar column of `byte` within the line that starts at
/// `line_start_byte`.
///
/// Counts Rust `char`s (Unicode scalar values) in `[line_start_byte, byte)`.
/// Each multi-byte rune contributes exactly one column, per v1-spec §10. The
/// source is accessed only through `str::get`, which returns `None` for a
/// non-char-boundary argument; in that case the function returns `0` rather
/// than panicking (this never happens for offsets produced by [`line_offsets`]
/// or ast-grep, which are always on char boundaries).
fn column_at(source: &str, line_start_byte: usize, byte: usize) -> usize {
    let Some(tail) = source.get(line_start_byte..) else {
        return 0;
    };
    let within_line = byte.saturating_sub(line_start_byte);
    tail.char_indices().take_while(|(pos, _)| *pos < within_line).count()
}

/// `(0-based line index, 0-based codepoint column)` of `byte`.
///
/// Combines [`line_at_byte`] and [`column_at`]. Returns `(line, 0)` if the
/// line-start offset is somehow missing from `offsets` (defensive; cannot
/// happen with a table from [`line_offsets`]).
pub(super) fn line_col(source: &str, offsets: &[usize], byte: usize) -> (usize, usize) {
    let line = line_at_byte(offsets, byte);
    let Some(&line_start_byte) = offsets.get(line) else {
        return (line, 0);
    };
    (line, column_at(source, line_start_byte, byte))
}

#[cfg(test)]
mod tests {
    use super::{FILE_LEVEL_SITE, MatchSite, line_at_byte, line_col, line_offsets};

    // ── line_at_byte / line_offsets ────────────────────────────────────────

    #[test]
    fn empty_source_has_single_line_offset() {
        let offsets = line_offsets("");
        assert_eq!(offsets, vec![0]);
        assert_eq!(line_at_byte(&offsets, 0), 0);
    }

    #[test]
    fn first_byte_is_line_zero() {
        let offsets = line_offsets("abc\ndef\n");
        assert_eq!(offsets, vec![0, 4, 8]);
        assert_eq!(line_at_byte(&offsets, 0), 0);
        assert_eq!(line_at_byte(&offsets, 3), 0, "byte before first \\n is line 0");
        assert_eq!(line_at_byte(&offsets, 4), 1, "byte at line-1 start is line 1");
        assert_eq!(line_at_byte(&offsets, 7), 1);
        assert_eq!(line_at_byte(&offsets, 8), 2, "byte after final \\n is line 2");
    }

    // ── ASCII column accuracy ──────────────────────────────────────────────

    #[test]
    fn ascii_indented_token_column_counts_spaces() {
        // Four spaces then `for`.
        let source = "    for v in values {\n}\n";
        let offsets = line_offsets(source);
        // `for` starts at byte 4.
        let (line, col) = line_col(source, &offsets, 4);
        assert_eq!(line, 0);
        assert_eq!(col, 4, "four ASCII spaces ⇒ codepoint column 4");
    }

    #[test]
    fn ascii_column_zero_for_first_rune() {
        let source = "for x in y {}";
        let offsets = line_offsets(source);
        let (line, col) = line_col(source, &offsets, 0);
        assert_eq!((line, col), (0, 0));
    }

    // ── Multi-byte leading UTF-8 ───────────────────────────────────────────
    //
    // The H3 guarantee: a multi-byte rune before the token contributes ONE
    // column, not its byte length.

    #[test]
    fn two_byte_leading_rune_counts_as_one_column() {
        // U+00E9 `é` is 2 UTF-8 bytes (C3 A9), then 3 spaces, then `for`.
        let source = "é   for";
        let offsets = line_offsets(source);
        // `é` occupies bytes [0,2); spaces [2,5); `for` starts at byte 5.
        let for_byte = 5;
        let (line, col) = line_col(source, &offsets, for_byte);
        assert_eq!((line, col), (0, 4), "é (2 bytes) + 3 spaces ⇒ column 4, not 6");
    }

    #[test]
    fn three_byte_leading_rune_counts_as_one_column() {
        // U+2192 `→` is 3 UTF-8 bytes (E2 86 92), then 3 spaces, then `for`.
        // This is exactly the spec's `→   for` example.
        let source = "→   for";
        let offsets = line_offsets(source);
        let for_byte = source.len() - "for".len();
        let (line, col) = line_col(source, &offsets, for_byte);
        assert_eq!((line, col), (0, 4), "→ (3 bytes) + 3 spaces ⇒ column 4, not {}", for_byte);
    }

    #[test]
    fn four_byte_leading_emoji_counts_as_one_column() {
        // U+1F680 `🚀` is 4 UTF-8 bytes, then 3 spaces, then `for`.
        let source = "🚀   for";
        let offsets = line_offsets(source);
        let for_byte = source.len() - "for".len();
        let (line, col) = line_col(source, &offsets, for_byte);
        assert_eq!((line, col), (0, 4), "🚀 (4 bytes) + 3 spaces ⇒ column 4, not {}", for_byte);
    }

    #[test]
    fn mixed_multibyte_prefix_each_rune_one_column() {
        // `→` (3B) + `é` (2B) + `🚀` (4B) + 2 spaces + `for`.
        let source = "→é🚀  for";
        let offsets = line_offsets(source);
        let for_byte = source.len() - "for".len();
        let (line, col) = line_col(source, &offsets, for_byte);
        assert_eq!((line, col), (0, 5), "3 runes + 2 spaces ⇒ column 5");
    }

    // ── CJK matched token: the token itself is multi-byte ──────────────────

    #[test]
    fn cjk_identifier_column_and_byte_range_are_consistent() {
        // A line where a CJK identifier `関数` (3 bytes per rune, 6 bytes
        // total) sits at a known column, then a second CJK token `本体`.
        let source = "    関数() { 本体 }";
        let offsets = line_offsets(source);
        // 4 spaces ⇒ `関数` at byte 4. `関` is U+95A2 (3 bytes).
        let kanji_start = 4;
        let (line, col) = line_col(source, &offsets, kanji_start);
        assert_eq!((line, col), (0, 4), "CJK identifier sits at codepoint column 4");

        // Byte-patching sanity: the byte range of `関数` slices back to the
        // exact token. This is the deterministic-patching guarantee for a
        // multi-byte matched token.
        let kanji_end = kanji_start + "関数".len();
        let slice = source.get(kanji_start..kanji_end);
        assert_eq!(slice, Some("関数"), "byte range must slice back to the CJK token");
    }

    // ── Cross-line column independence ─────────────────────────────────────

    #[test]
    fn column_resets_at_each_line() {
        // `for` on line 1 indented 2 spaces; column must be 2, computed
        // from line 1's start — not carried over from line 0.
        let source = "x\n  for";
        let offsets = line_offsets(source);
        let for_byte = source.len() - "for".len();
        let (line, col) = line_col(source, &offsets, for_byte);
        assert_eq!((line, col), (1, 2));
    }

    // ── Byte-patching round-trip (TextRange start_byte..end_byte) ──────────
    //
    // `titania_core::TextRange` is byte-based by construction (§10: fields
    // `start_byte`, `end_byte`). `RepairHint::Patch` is not emitted by the
    // ast-grep lane today (it emits advisory hints), but the byte offsets
    // produced here are the patch substrate. This test proves they are
    // byte-accurate: slicing `[start, end)` recovers the matched token, so a
    // hypothetical `String::replace_range(start..end, …)` would target the
    // right bytes.

    #[test]
    fn byte_range_slices_back_to_the_token_ascii_and_multibyte() {
        let cases = [
            ("    for v in x {}", "for", 4),
            ("→   for", "for", 6),
            ("🚀   for", "for", 7),
            ("    関数() {}", "関数", 4),
        ];
        for (source, token, start) in cases {
            let end = start + token.len();
            assert_eq!(
                source.get(start..end),
                Some(token),
                "byte range [{start}..{end}) must slice back to `{token}` in `{source}`",
            );
            // And the column computed from that start byte matches the
            // codepoint count of the prefix.
            let offsets = line_offsets(source);
            let (_, col) = line_col(source, &offsets, start);
            let prefix_chars = source.get(..start).map(str::chars).map(Iterator::count);
            assert_eq!(Some(col), prefix_chars, "column must equal prefix codepoint count");
        }
    }

    // ── MatchSite / FILE_LEVEL_SITE ────────────────────────────────────────

    #[test]
    fn from_range_preserves_byte_offsets() {
        let site = MatchSite::from_range(4..7);
        assert_eq!(site.start_byte, 4);
        assert_eq!(site.end_byte, 7);
    }

    #[test]
    fn file_level_site_anchors_at_first_byte_pair() {
        assert_eq!(FILE_LEVEL_SITE.start_byte, 0);
        assert_eq!(FILE_LEVEL_SITE.end_byte, 1);
    }
}
