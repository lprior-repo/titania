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
    let mut out = BTreeSet::new();
    let needle = format!("{type_name}::");
    collect_refs(text, &needle, 0, &mut out);
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

fn body_at_cursor(
    bytes: &[u8],
    cursor: usize,
    text: &str,
    start: usize,
    state: &mut BalanceState,
) -> Option<String> {
    bytes.get(cursor).and_then(|byte| match completed_body(text, start, cursor, *byte, state) {
        BodyStep::Complete(body) => body,
        BodyStep::Continue => None,
    })
}

fn balanced_item(text: &str, start: usize) -> Option<String> {
    let bytes = text.as_bytes();
    let mut state = BalanceState { depth: 0, started: false };
    (start..bytes.len()).find_map(|cursor| body_at_cursor(bytes, cursor, text, start, &mut state))
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

enum RefHit {
    Valid { name: String, next_cursor: usize },
    Invalid { next_cursor: usize },
}

impl RefHit {
    const fn next_cursor(&self) -> usize {
        match self {
            Self::Valid { next_cursor, .. } | Self::Invalid { next_cursor } => *next_cursor,
        }
    }

    fn into_name(self) -> Option<String> {
        match self {
            Self::Valid { name, .. } => Some(name),
            Self::Invalid { .. } => None,
        }
    }
}

fn next_ref_name_start(text: &str, needle: &str, cursor: usize) -> Option<usize> {
    let tail = text.get(cursor..)?;
    let offset = tail.find(needle)?;
    Some(cursor.saturating_add(offset).saturating_add(needle.len()))
}

fn next_qualified_ref(text: &str, needle: &str, cursor: usize) -> Option<RefHit> {
    let name_start = next_ref_name_start(text, needle, cursor)?;
    Some(ref_name_at(text, name_start).filter(|name| !name.is_empty()).map_or_else(
        || RefHit::Invalid { next_cursor: name_start.saturating_add(1) },
        |name| RefHit::Valid { next_cursor: name_start.saturating_add(name.len()), name },
    ))
}

fn collect_refs(text: &str, needle: &str, start: usize, out: &mut BTreeSet<String>) {
    let mut cursor = start;
    std::iter::from_fn(|| {
        let hit = next_qualified_ref(text, needle, cursor)?;
        cursor = hit.next_cursor();
        Some(hit)
    })
    .filter_map(RefHit::into_name)
    .fold((), |(), name| {
        let _ = out.insert(name);
    });
}

fn ref_name_at(text: &str, name_start: usize) -> Option<String> {
    let name: String = text
        .get(name_start..)?
        .chars()
        .take_while(|ch| ch.is_alphanumeric() || *ch == '_')
        .collect();
    if name.is_empty() { None } else { Some(name) }
}

#[cfg(test)]
mod tests {
    use super::collect_qualified_refs;

    #[test]
    fn invalid_typename_no_valid_following_continues_scanning() {
        // TypeName::: has no valid identifier after :: — scanner must skip past
        // and continue to find TypeName::Valid on the same line.
        let text = "let x = TypeName::: ; let y = TypeName::Valid;";
        let refs = collect_qualified_refs(text, "TypeName");
        assert_eq!(
            refs,
            std::collections::BTreeSet::from_iter([String::from("Valid")]),
            "scanner must continue past invalid TypeName:: and collect Valid"
        );
    }

    #[test]
    fn invalid_typename_no_valid_following_does_not_insert_empty() {
        // TypeName:: followed by non-identifier (space, punctuation) must not
        // insert an empty name into the output set.
        let text = "let x = TypeName:: ; let y = TypeName::Foo;";
        let refs = collect_qualified_refs(text, "TypeName");
        assert!(!refs.contains(""), "empty string must never be a collected variant name");
        assert_eq!(refs, std::collections::BTreeSet::from_iter([String::from("Foo")]));
    }

    #[test]
    fn valid_typename_after_invalid_is_collected() {
        // Multiple invalid TypeName:: hits must not prevent a later valid one
        // from being collected.
        let text = "TypeName::: TypeName::123 TypeName:::: TypeName::Bar;";
        let refs = collect_qualified_refs(text, "TypeName");
        assert_eq!(
            refs,
            std::collections::BTreeSet::from_iter([String::from("123"), String::from("Bar")]),
            "only non-empty valid names should be collected; 123 is a valid identifier"
        );
    }

    #[test]
    fn valid_name_cursor_advances_by_name_length_no_false_ref() {
        // TypeName::FooBar contains "Foo" which is part of what a user might
        // mistakenly search for. The cursor must advance by FooBar's full length
        // (6) so that scanning does not re-find a substring as a new ref.
        let text = "TypeName::FooBar TypeName::Foo";
        let refs = collect_qualified_refs(text, "TypeName");
        assert_eq!(
            refs,
            std::collections::BTreeSet::from_iter([String::from("FooBar"), String::from("Foo")]),
            "each TypeName::<valid> must be collected exactly once"
        );
    }

    #[test]
    fn no_duplicate_refs_from_adjacent_invalid_and_valid() {
        // TypeName::TypeName::Valid: the first TypeName:: is followed by another
        // TypeName which IS a valid identifier, so it collects "TypeName". The
        // second TypeName:: then needs to be found in the remainder.
        let text = "TypeName::TypeName::Valid;";
        let refs = collect_qualified_refs(text, "TypeName");
        // First hit: TypeName::TypeName -> valid name "TypeName"
        // Cursor advances past "TypeName" (len 8). Remaining: "::Valid;"
        // Next find("TypeName::") won't match "::Valid", so only "TypeName"
        // is collected (not "Valid" because the :: before Valid was consumed
        // by the first hit's cursor advance).
        assert_eq!(
            refs,
            std::collections::BTreeSet::from_iter([String::from("TypeName")]),
            "TypeName::TypeName::Valid should collect only the first TypeName as a ref"
        );
    }
}
