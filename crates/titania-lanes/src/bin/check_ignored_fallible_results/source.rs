#[derive(Clone, Debug, Eq, PartialEq)]
enum LineKind {
    NonCode,
    Signature,
    Expression,
}

/// Source line after comments and signatures are classified for scanning.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceLine {
    code: String,
    kind: LineKind,
}

impl SourceLine {
    /// Strip non-code segments and classify a raw source line.
    /// Delegates to the shared `titania_lanes::SourceLine::parse` for
    /// comment/string stripping, then classifies the result.
    pub fn parse(raw: &str, state: &mut titania_lanes::SourceLineState) -> Self {
        let shared = titania_lanes::SourceLine::parse(raw, state);
        let code = shared.code().trim().to_owned();
        let kind = classify_kind(&code);
        Self { code, kind }
    }

    /// Return the stripped and trimmed code segment.
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Report whether this line is a function signature.
    #[must_use]
    pub fn is_signature(&self) -> bool {
        self.kind == LineKind::Signature
    }

    /// Report whether this line is a source expression worth scanning.
    #[must_use]
    pub fn is_code_expression(&self) -> bool {
        self.kind == LineKind::Expression
    }
}

fn classify_kind(trimmed: &str) -> LineKind {
    if trimmed.is_empty() {
        LineKind::NonCode
    } else if is_signature_line(trimmed) {
        LineKind::Signature
    } else {
        LineKind::Expression
    }
}

fn is_signature_line(trimmed: &str) -> bool {
    let looks_like_fn = [
        "fn ",
        "pub fn ",
        "pub fn ",
        "pub fn ",
        "async fn ",
        "pub async fn ",
        "const fn ",
        "pub const fn ",
    ]
    .iter()
    .any(|prefix| trimmed.starts_with(prefix));
    looks_like_fn && trimmed.contains('(')
}
