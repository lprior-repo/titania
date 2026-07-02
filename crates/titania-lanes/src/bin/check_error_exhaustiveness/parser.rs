use std::collections::BTreeSet;

/// Extract simple enum variant names from a Rust source string.
#[must_use]
pub fn extract_enum_variants(text: &str, name: &str) -> BTreeSet<String> {
    let Some(body) = extract_enum_body(text, name) else {
        return BTreeSet::new();
    };
    body.lines().filter_map(|line| extract_variant(line, name)).collect()
}

/// Find the balanced source body for a named function.
#[must_use]
pub fn find_function_body(text: &str, fn_name: &str) -> Option<String> {
    function_patterns(fn_name).iter().find_map(|pattern| {
        text.find(pattern.as_str()).and_then(|start| balanced_item(text, start))
    })
}

/// Collect `TypeName::Variant` references from source text.
#[must_use]
pub fn collect_qualified_refs(text: &str, type_name: &str) -> BTreeSet<String> {
    let needle = format!("{type_name}::");
    let mut out = BTreeSet::new();
    let mut cursor = 0;
    while let Some((name, next_cursor)) = next_qualified_ref(text, &needle, cursor) {
        let _ = out.insert(name);
        cursor = next_cursor;
    }
    out
}

fn extract_enum_body(text: &str, name: &str) -> Option<String> {
    let marker = format!("pub enum {name}");
    text.find(&marker).and_then(|start| balanced_item(text, start))
}

struct BalanceState {
    depth: i32,
    started: bool,
}

enum BodyStep {
    Continue,
    Complete(Option<String>),
}

fn balanced_item(text: &str, start: usize) -> Option<String> {
    let bytes = text.as_bytes();
    let mut state = BalanceState { depth: 0, started: false };
    let mut cursor = start;
    while let Some(&byte) = bytes.get(cursor) {
        match completed_body(text, start, cursor, byte, &mut state) {
            BodyStep::Complete(body) => return body,
            BodyStep::Continue => cursor = cursor.saturating_add(1),
        }
    }
    None
}

fn completed_body(
    text: &str,
    start: usize,
    cursor: usize,
    byte: u8,
    state: &mut BalanceState,
) -> BodyStep {
    if consume_body_byte(byte, state) {
        BodyStep::Complete(text.get(start..cursor.saturating_add(1)).map(str::to_string))
    } else {
        BodyStep::Continue
    }
}

const fn consume_body_byte(byte: u8, state: &mut BalanceState) -> bool {
    if byte == b'{' {
        state.depth = state.depth.saturating_add(1);
        state.started = true;
    } else if byte == b'}' {
        state.depth = state.depth.saturating_sub(1);
        return state.started && state.depth == 0;
    }
    false
}

fn extract_variant(line: &str, enum_name: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
        return None;
    }
    let first_word = trimmed.split_whitespace().next()?;
    let name = first_word.trim_end_matches(',').trim_end_matches('(');
    valid_variant_name(name, enum_name).then(|| name.to_string())
}

fn valid_variant_name(name: &str, enum_name: &str) -> bool {
    !name.is_empty()
        && name != enum_name
        && name.chars().next().is_some_and(|first| first.is_ascii_uppercase())
        && name.chars().all(|ch| ch.is_alphanumeric() || ch == '_')
}

fn function_patterns(fn_name: &str) -> [String; 4] {
    [
        format!("pub fn {fn_name}("),
        format!("pub fn {fn_name}<"),
        format!("fn {fn_name}("),
        format!("fn {fn_name}<"),
    ]
}

fn next_qualified_ref(text: &str, needle: &str, cursor: usize) -> Option<(String, usize)> {
    let tail = text.get(cursor..)?;
    let offset = tail.find(needle)?;
    let name_start = cursor.saturating_add(offset).saturating_add(needle.len());
    let name = ref_name_at(text, name_start)?;
    let next_cursor = name_start.saturating_add(name.len());
    Some((name, next_cursor))
}

fn ref_name_at(text: &str, name_start: usize) -> Option<String> {
    let name: String = text
        .get(name_start..)?
        .chars()
        .take_while(|ch| ch.is_alphanumeric() || *ch == '_')
        .collect();
    if name.is_empty() { None } else { Some(name) }
}
