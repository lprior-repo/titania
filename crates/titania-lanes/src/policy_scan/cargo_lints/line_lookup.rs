//! Source-line lookup for Cargo lint table entries.

/// Find a lint key's 1-indexed line within one TOML lint section.
#[must_use]
pub(super) fn find_lint_line(content: &str, table_prefix: &str, category: &str, key: &str) -> u32 {
    let section = format!("[{table_prefix}.{category}]");
    content
        .lines()
        .enumerate()
        .scan(false, move |in_section, (line_index, line)| {
            Some(section_line(&section, in_section, line_index, line))
        })
        .flatten()
        .find_map(|(line, trimmed)| line_matches_key(trimmed, key).then_some(line))
        .map_or(0, |line| line)
}

fn section_line<'a>(
    section: &str,
    in_section: &mut bool,
    line_index: usize,
    line: &'a str,
) -> Option<(u32, &'a str)> {
    let trimmed = line.trim();
    if is_table_header(trimmed) {
        *in_section = trimmed == section;
        return None;
    }
    if *in_section {
        return Some((line_number(line_index), trimmed));
    }
    None
}

fn line_matches_key(line: &str, key: &str) -> bool {
    match line.split_once('=') {
        Some((actual_key, _)) => actual_key.trim() == key,
        None => false,
    }
}

fn is_table_header(line: &str) -> bool {
    line.starts_with('[') && !line.starts_with("[[") && line.ends_with(']')
}

fn line_number(zero_based: usize) -> u32 {
    u32::try_from(zero_based.saturating_add(1)).map_or(u32::MAX, |line| line)
}
