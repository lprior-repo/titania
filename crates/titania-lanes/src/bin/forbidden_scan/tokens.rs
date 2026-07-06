/// Forbidden-surface token model extracted from the lane so the scanner
/// body stays under the 300-line source limit.
///
/// Tokens are stored as their canonical surface (`panic!`, `unwrap`,
/// `expect`, `todo!`, `unimplemented!`, `dbg!`). Macro tokens match as
/// raw substrings because the `!` is part of the macro syntax. Method
/// tokens (`unwrap`, `expect`) match only when preceded by a method
/// receiver (`.` or `::`) so we do not false-positive on identifiers
/// like `myexpect`.

#[derive(Clone, Debug, Eq, PartialEq)]
struct ForbiddenToken {
    name: String,
    kind: TokenKind,
}

/// What shape of Rust construct the forbidden surface is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TokenKind {
    /// Macro invocation, e.g. `panic!(...)` — matched as a raw
    /// substring because the `!` is part of the surface.
    Macro,
    /// Method call, e.g. `x.unwrap()` — matched only when preceded by
    /// a method-call receiver (`.` or `::`) and followed by `(`. This
    /// prevents false positives on identifiers like `myexpect` or
    /// `myexpect()`.
    Method,
}

impl ForbiddenToken {
    fn parse(raw: &str) -> Option<Self> {
        parse_token(raw)
    }

    fn as_str(&self) -> &str {
        &self.name
    }

    fn is_present_in(&self, code: &str) -> bool {
        token_present(&self.name, self.kind, code)
    }
}

fn parse_token(raw: &str) -> Option<ForbiddenToken> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(ForbiddenToken { name: trimmed.to_owned(), kind: token_kind(trimmed) })
}

fn token_kind(trimmed: &str) -> TokenKind {
    if trimmed.ends_with('!') { TokenKind::Macro } else { TokenKind::Method }
}

fn token_present(name: &str, kind: TokenKind, code: &str) -> bool {
    let mut search_start = 0usize;
    std::iter::from_fn(|| next_candidate(name, code, &mut search_start))
        .any(|idx| matches_at(name, kind, code, idx))
}

fn next_candidate(name: &str, code: &str, search_start: &mut usize) -> Option<usize> {
    let found = find_from(name, code, *search_start);
    if let Some(idx) = found {
        *search_start = idx.saturating_add(1);
    }
    found
}

fn find_from(name: &str, code: &str, search_start: usize) -> Option<usize> {
    code.get(search_start..)
        .and_then(|tail| tail.find(name))
        .map(|idx| search_start.saturating_add(idx))
}

/// Decide whether the match at `idx` is a real surface occurrence
/// (per [`TokenKind`]) rather than a substring of a larger identifier.
fn matches_at(name: &str, kind: TokenKind, code: &str, idx: usize) -> bool {
    match kind {
        TokenKind::Macro => is_macro_match(code, idx),
        TokenKind::Method => is_method_match(name, code, idx),
    }
}

fn is_macro_match(code: &str, idx: usize) -> bool {
    // Reject identifier-prefix matches: the byte before the match (if
    // any) must not be alphanumeric/underscore.
    macro_prefix_allowed(code.as_bytes(), idx)
}

fn macro_prefix_allowed(bytes: &[u8], idx: usize) -> bool {
    byte_before(bytes, idx).is_none_or(|byte| !is_word_byte(byte))
}

fn is_method_match(name: &str, code: &str, idx: usize) -> bool {
    let bytes = code.as_bytes();
    let after = idx.saturating_add(name.len());
    method_prefix_allowed(bytes, idx) && bytes.get(after).is_some_and(|byte| *byte == b'(')
}

fn method_prefix_allowed(bytes: &[u8], idx: usize) -> bool {
    // Require a method-call receiver directly before: `.unwrap` or
    // `::unwrap` (e.g. `Result::unwrap(...)`). Reject identifier-prefix
    // matches so `myexpect` is not flagged.
    match byte_before(bytes, idx) {
        Some(b'.' | b':') => true,
        Some(byte) if is_word_byte(byte) => false,
        _ => idx == 0,
    }
}

fn byte_before(bytes: &[u8], idx: usize) -> Option<u8> {
    idx.checked_sub(1).and_then(|pos| bytes.get(pos).copied())
}

const fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

#[cfg(test)]
mod token_tests {
    use super::ForbiddenToken;
    fn macro_token(name: &str) -> ForbiddenToken {
        let kind =
            if name.ends_with('!') { super::TokenKind::Macro } else { super::TokenKind::Method };
        ForbiddenToken { name: name.to_owned(), kind }
    }

    #[test]
    fn macro_token_matches_panic_bang() {
        let token = macro_token("panic!");
        assert!(token.is_present_in("panic!(\"boom\")"));
        assert!(token.is_present_in("let _ = panic!();"));
    }

    #[test]
    fn macro_token_rejects_identifier_prefixed_match() {
        // `mypanic!` must not be flagged as `panic!`.
        let token = macro_token("panic!");
        assert!(!token.is_present_in("mypanic!()"));
    }

    #[test]
    fn method_token_matches_dot_receiver() {
        let token = macro_token("unwrap");
        assert!(token.is_present_in("x.unwrap()"));
        // `unwrap_or_default` is a different method, not `unwrap`.
        assert!(!token.is_present_in("x.unwrap_or_default()"));
    }

    #[test]
    fn method_token_matches_double_colon_receiver() {
        let token = macro_token("unwrap");
        assert!(token.is_present_in("Result::unwrap(r)"));
    }

    #[test]
    fn method_token_matches_expect_with_message() {
        // Regression: the old plain-substring matcher missed this
        // because `.expect("msg")` never contains the literal
        // `expect()` token.
        let token = macro_token("expect");
        assert!(token.is_present_in("fs::read_to_string(\"/tmp/x\").expect(\"boom\")"));
    }

    #[test]
    fn method_token_rejects_identifier_prefixed_match() {
        // `myexpect()` must not be flagged as the `expect` method.
        let token = macro_token("expect");
        assert!(!token.is_present_in("myexpect()"));
        assert!(!token.is_present_in("myexpect"));
    }

    #[test]
    fn method_token_requires_open_paren() {
        let token = macro_token("unwrap");
        // No `(` after the name means it's just an identifier in scope.
        assert!(!token.is_present_in("let unwrap = 1;"));
        assert!(!token.is_present_in("x.unwrap"));
    }

    #[test]
    fn empty_token_string_is_rejected() {
        assert!(ForbiddenToken::parse("").is_none());
        assert!(ForbiddenToken::parse("   ").is_none());
    }

    #[test]
    fn dbg_macro_token_matches_bang_form() {
        let token = macro_token("dbg!");
        assert!(token.is_present_in("dbg!(x)"));
    }

    #[test]
    fn unwrap_or_method_token_matches() {
        let token = macro_token("unwrap_or");
        assert!(token.is_present_in("x.unwrap_or(1)"));
        assert!(!token.is_present_in("x.unwrap_or_default(1)"));
        assert!(!token.is_present_in("x.unwrap_or_else(f)"));
    }

    #[test]
    fn unwrap_or_else_method_token_matches() {
        let token = macro_token("unwrap_or_else");
        assert!(token.is_present_in("x.unwrap_or_else(|_| 1)"));
        assert!(!token.is_present_in("x.unwrap_or_default(1)"));
    }

    #[test]
    fn unwrap_or_default_method_token_matches() {
        let token = macro_token("unwrap_or_default");
        assert!(token.is_present_in("x.unwrap_or_default()"));
    }
}
