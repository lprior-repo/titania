fn find_char_in(text: &str, start: usize, target: char) -> Option<usize> {
    let rest = text.get(start..)?;
    rest.find(target).map(|off| start.saturating_add(off))
}

fn extract_enum_body(text: &str, enum_name: &str) -> Option<String> {
    let marker = format!("pub enum {enum_name}");
    let start = text.find(&marker)?;
    let open_pos = find_char_in(text, start, '{')?;
    balanced_block_from_open(text, open_pos)
}

fn balanced_block_from_open(text: &str, open_pos: usize) -> Option<String> {
    text.as_bytes()
        .get(open_pos..)?
        .iter()
        .enumerate()
        .scan(0_i32, |depth, (offset, byte)| {
            *depth = next_depth(*depth, *byte);
            Some((open_pos.saturating_add(offset), *depth, *byte))
        })
        .find_map(|(idx, depth, byte)| closed_block(text, open_pos, idx, depth, byte))
}

fn closed_block(text: &str, open_pos: usize, idx: usize, depth: i32, byte: u8) -> Option<String> {
    if depth != 0 || byte != b'}' {
        return None;
    }
    let end = idx.saturating_add(1);
    text.get(open_pos..end).map(str::to_string)
}

const fn next_depth(depth: i32, byte: u8) -> i32 {
    match byte {
        b'{' => depth.saturating_add(1),
        b'}' => depth.saturating_sub(1),
        _ => depth,
    }
}

fn extract_enum_variants(text: &str, enum_name: &str) -> StateSet {
    extract_enum_body(text, enum_name).map_or_else(StateSet::new, |body| {
        body.lines().filter_map(|line| variant_name(line, enum_name)).collect()
    })
}

fn variant_name(line: &str, enum_name: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
        return None;
    }
    let first_word = trimmed.split_whitespace().next()?;
    accepted_variant_name(first_word.trim_end_matches(',').trim_end_matches('('), enum_name)
}

fn accepted_variant_name(name: &str, enum_name: &str) -> Option<String> {
    let first = name.chars().next()?;
    let valid = !name.is_empty()
        && name != enum_name
        && first.is_ascii_uppercase()
        && name.chars().all(|c| c.is_alphanumeric() || c == '_');
    if valid { Some(name.to_string()) } else { None }
}

fn extract_block_after(text: &str, marker: &str, end_marker: &str) -> Option<String> {
    let start = text.find(marker)?;
    let end = find_substr(text, start, end_marker)?;
    let end_inclusive = end.saturating_add(end_marker.len());
    text.get(start..end_inclusive).map(str::to_string)
}

fn find_substr(text: &str, start: usize, needle: &str) -> Option<usize> {
    let hay = text.get(start..)?;
    hay.find(needle).map(|off| start.saturating_add(off))
}

fn collect_stepstate_refs(text: &str) -> StateSet {
    text.match_indices("StepState::")
        .filter_map(|(start, needle)| stepstate_ref_at(text, start.saturating_add(needle.len())))
        .collect()
}

fn stepstate_ref_at(text: &str, start: usize) -> Option<String> {
    let name: String =
        text.get(start..)?.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
    if name.is_empty() { None } else { Some(name) }
}

fn find_function_body(text: &str, fn_name: &str) -> Option<String> {
    function_patterns(fn_name).iter().find_map(|pat| body_after_pattern(text, pat))
}

fn function_patterns(fn_name: &str) -> [String; 4] {
    [
        format!("pub fn {fn_name}("),
        format!("pub fn {fn_name}<"),
        format!("fn {fn_name}("),
        format!("fn {fn_name}<"),
    ]
}

fn body_after_pattern(text: &str, pattern: &str) -> Option<String> {
    let start = text.find(pattern)?;
    let open_pos = find_char_in(text, start, '{')?;
    balanced_block_from_open(text, open_pos)
}
