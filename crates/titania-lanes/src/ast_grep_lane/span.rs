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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MatchSite {
    pub(super) start_byte: usize,
    pub(super) end_byte: usize,
}

impl MatchSite {
    #[must_use]
    pub(super) const fn from_range(range: std::ops::Range<usize>) -> Self {
        Self { start_byte: range.start, end_byte: range.end }
    }
}

pub(super) fn line_offsets(source: &str) -> Vec<usize> {
    let mut offsets: Vec<usize> = vec![0];
    offsets.extend(source.match_indices('\n').map(|(i, _)| i.saturating_add(1)));
    offsets
}

pub(super) fn line_at_byte(offsets: &[usize], byte: usize) -> usize {
    offsets.partition_point(|&offset| offset <= byte).saturating_sub(1)
}

fn column_at(source: &str, line_start_byte: usize, byte: usize) -> usize {
    let Some(tail) = source.get(line_start_byte..) else {
        return 0;
    };
    let within_line = byte.saturating_sub(line_start_byte);
    tail.char_indices().take_while(|(pos, _)| *pos < within_line).count()
}

pub(super) fn line_col(source: &str, offsets: &[usize], byte: usize) -> (usize, usize) {
    let line = line_at_byte(offsets, byte);
    let Some(&line_start_byte) = offsets.get(line) else {
        return (line, 0);
    };
    (line, column_at(source, line_start_byte, byte))
}

#[cfg(test)]
mod tests {
    use super::{MatchSite, line_at_byte, line_col, line_offsets};

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

    #[test]
    fn ascii_indented_token_column_counts_spaces() {
        let source = "    for v in values {\n}\n";
        let offsets = line_offsets(source);
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

    #[test]
    fn two_byte_leading_rune_counts_as_one_column() {
        let source = "é   for";
        let offsets = line_offsets(source);
        let for_byte = 5;
        let (line, col) = line_col(source, &offsets, for_byte);
        assert_eq!((line, col), (0, 4), "é (2 bytes) + 3 spaces ⇒ column 4, not 6");
    }

    #[test]
    fn three_byte_leading_rune_counts_as_one_column() {
        let source = "→   for";
        let offsets = line_offsets(source);
        let for_byte = source.len() - "for".len();
        let (line, col) = line_col(source, &offsets, for_byte);
        assert_eq!((line, col), (0, 4), "→ (3 bytes) + 3 spaces ⇒ column 4, not {}", for_byte);
    }

    #[test]
    fn four_byte_leading_emoji_counts_as_one_column() {
        let source = "🚀   for";
        let offsets = line_offsets(source);
        let for_byte = source.len() - "for".len();
        let (line, col) = line_col(source, &offsets, for_byte);
        assert_eq!((line, col), (0, 4), "🚀 (4 bytes) + 3 spaces ⇒ column 4, not {}", for_byte);
    }

    #[test]
    fn mixed_multibyte_prefix_each_rune_one_column() {
        let source = "→é🚀  for";
        let offsets = line_offsets(source);
        let for_byte = source.len() - "for".len();
        let (line, col) = line_col(source, &offsets, for_byte);
        assert_eq!((line, col), (0, 5), "3 runes + 2 spaces ⇒ column 5");
    }

    #[test]
    fn cjk_identifier_column_and_byte_range_are_consistent() {
        let source = "    関数() { 本体 }";
        let offsets = line_offsets(source);
        let kanji_start = 4;
        let (line, col) = line_col(source, &offsets, kanji_start);
        assert_eq!((line, col), (0, 4), "CJK identifier sits at codepoint column 4");

        let kanji_end = kanji_start + "関数".len();
        let slice = source.get(kanji_start..kanji_end);
        assert_eq!(slice, Some("関数"), "byte range must slice back to the CJK token");
    }

    #[test]
    fn column_resets_at_each_line() {
        let source = "x\n  for";
        let offsets = line_offsets(source);
        let for_byte = source.len() - "for".len();
        let (line, col) = line_col(source, &offsets, for_byte);
        assert_eq!((line, col), (1, 2));
    }

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
            let offsets = line_offsets(source);
            let (_, col) = line_col(source, &offsets, start);
            let prefix_chars = source.get(..start).map(str::chars).map(Iterator::count);
            assert_eq!(Some(col), prefix_chars, "column must equal prefix codepoint count");
        }
    }

    #[test]
    fn from_range_preserves_byte_offsets() {
        let site = MatchSite::from_range(4..7);
        assert_eq!(site.start_byte, 4);
        assert_eq!(site.end_byte, 7);
    }
}
