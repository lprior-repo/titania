use std::collections::BTreeSet;

fn quoted_values_in_line(line: &str) -> Vec<String> {
    line.split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .filter(|value| !value.is_empty())
        .collect()
}

/// Locate the `[` that opens a TOML array for `key`. Tolerates arbitrary
/// whitespace, including compact arrays such as `members=["crates/foo"]`.
fn array_open_after<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let bytes = text.as_bytes();
    let key_bytes = key.as_bytes();
    if key_bytes.is_empty() || bytes.len() < key_bytes.len() {
        return None;
    }
    bytes.windows(key_bytes.len()).enumerate().find_map(|(match_start, window)| {
        array_slice_for_match(text, bytes, key_bytes, match_start, window)
    })
}

fn array_slice_for_match<'a>(
    text: &'a str,
    bytes: &[u8],
    key_bytes: &[u8],
    match_start: usize,
    window: &[u8],
) -> Option<&'a str> {
    if window != key_bytes {
        return None;
    }
    let key_end = match_start.saturating_add(key_bytes.len());
    bracket_after_line_key(bytes, match_start, key_end).and_then(|after| text.get(after..))
}

fn bracket_after_line_key(bytes: &[u8], match_start: usize, key_end: usize) -> Option<usize> {
    is_line_key_start(bytes, match_start).then_some(())?;
    bracket_after_key(bytes, key_end)
}

fn is_line_key_start(bytes: &[u8], match_start: usize) -> bool {
    bytes
        .get(..match_start)
        .and_then(|prefix| prefix.iter().rev().find(|byte| **byte != b' ' && **byte != b'\t'))
        .is_none_or(|byte| *byte == b'\n' || *byte == b'\r')
}

fn bracket_after_key(bytes: &[u8], key_end: usize) -> Option<usize> {
    let mut pos = skip_ascii_whitespace(bytes, key_end);
    if bytes.get(pos) != Some(&b'=') {
        return None;
    }
    pos = skip_ascii_whitespace(bytes, pos.saturating_add(1));
    if bytes.get(pos) == Some(&b'[') { Some(pos.saturating_add(1)) } else { None }
}

fn skip_ascii_whitespace(bytes: &[u8], pos: usize) -> usize {
    bytes
        .get(pos..)
        .and_then(|tail| tail.iter().position(|byte| !byte.is_ascii_whitespace()))
        .map_or(bytes.len(), |offset| pos.saturating_add(offset))
}

pub(super) fn quoted_array_values(text: &str, key: &str) -> BTreeSet<String> {
    let Some(after_key) = array_open_after(text, key) else {
        return BTreeSet::new();
    };
    let Some(end) = after_key.find(']') else {
        return BTreeSet::new();
    };
    after_key
        .get(..end)
        .into_iter()
        .flat_map(str::lines)
        .flat_map(|line| quoted_values_in_line(line).into_iter())
        .filter(|value| !value.is_empty())
        .collect()
}

pub(super) fn package_name(manifest: &str) -> Option<String> {
    manifest.lines().find_map(|line| {
        let trimmed = line.trim_start();
        let after = trimmed.strip_prefix("name")?;
        let after = after.trim_start().strip_prefix('=')?;
        let value = after.trim_start().strip_prefix('"')?;
        Some(value.split_once('"')?.0.to_owned())
    })
}

pub(super) fn named_table_values(manifest: &str, table: &str) -> BTreeSet<String> {
    manifest
        .lines()
        .fold((false, BTreeSet::new()), |(in_table, names), line| {
            collect_table_line(table, in_table, names, line)
        })
        .1
}

fn collect_table_line(
    table: &str,
    in_table: bool,
    mut names: BTreeSet<String>,
    line: &str,
) -> (bool, BTreeSet<String>) {
    let trimmed = line.trim();
    if trimmed.starts_with('[') {
        return (trimmed == table, names);
    }
    if !in_table {
        return (in_table, names);
    }
    insert_table_key(&mut names, trimmed);
    (in_table, names)
}

fn insert_table_key(names: &mut BTreeSet<String>, trimmed: &str) {
    let Some((name, _rest)) = trimmed.split_once('=') else {
        return;
    };
    let cleaned = name.trim();
    if !cleaned.is_empty() {
        let _inserted = names.insert(cleaned.to_owned());
    }
}

pub(super) fn binary_names(manifest: &str) -> BTreeSet<String> {
    manifest
        .lines()
        .fold((false, BTreeSet::new()), |(in_bin, names), line| {
            collect_bin_line(in_bin, names, line)
        })
        .1
}

fn collect_bin_line(
    in_bin: bool,
    mut names: BTreeSet<String>,
    line: &str,
) -> (bool, BTreeSet<String>) {
    let trimmed = line.trim();
    if trimmed.starts_with('[') {
        return (trimmed.starts_with("[[bin]]"), names);
    }
    if !in_bin || !trimmed.starts_with("name") {
        return (in_bin, names);
    }
    insert_bin_name(&mut names, trimmed);
    (in_bin, names)
}

fn insert_bin_name(names: &mut BTreeSet<String>, trimmed: &str) {
    let Some((_key, value)) = trimmed.split_once('=') else {
        return;
    };
    let cleaned = value.trim().trim_matches('"');
    if !cleaned.is_empty() {
        let _inserted = names.insert(cleaned.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::{array_open_after, quoted_array_values};

    #[test]
    fn array_open_after_handles_single_space() {
        let text = "[workspace]\nmembers = [\n    \"a\",\n    \"b\",\n]\n";
        assert_eq!(array_open_after(text, "members"), Some("\n    \"a\",\n    \"b\",\n]\n"));
    }

    #[test]
    fn array_open_after_handles_double_space_around_eq() {
        let text = "[workspace]\nmembers  =  [\n    \"a\",\n]\n";
        assert_eq!(array_open_after(text, "members"), Some("\n    \"a\",\n]\n"));
    }

    #[test]
    fn array_open_after_handles_leading_indent() {
        let text = "[workspace]\n    members = [ \"a\", \"b\" ]\n";
        assert_eq!(array_open_after(text, "members"), Some(" \"a\", \"b\" ]\n"));
    }

    #[test]
    fn array_open_after_returns_none_for_missing_key() {
        let text = "[workspace]\nresolver = \"2\"\n";
        assert!(array_open_after(text, "members").is_none());
    }

    #[test]
    fn quoted_array_values_tolerates_double_space() {
        let text = "[workspace]\nmembers  = [\"crates/x\", \"crates/y\"]\n";
        let set = quoted_array_values(text, "members");
        assert!(set.contains("crates/x"));
        assert!(set.contains("crates/y"));
    }

    #[test]
    fn quoted_array_values_accepts_compact_member_array() {
        let text = "[workspace]\nmembers=[\"crates/foo\"]\n";
        let set = quoted_array_values(text, "members");
        assert!(set.contains("crates/foo"));
    }
}
