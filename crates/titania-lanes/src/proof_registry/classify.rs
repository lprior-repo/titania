//! Proof-source content classification by tokens, never by extension.
//!
//! The smoke tag is matched on raw content because it lives in comments;
//! proof tokens are matched on comment- and string-stripped code.

use super::scanner::{Scan, scan_step};

/// Content classification of one proof-source body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceClass {
    /// Real proof tokens outside comments and string literals.
    GenuineProof,
    /// Explicitly non-authoritative smoke content; never production evidence.
    SmokeOnly,
    /// No proof tokens outside comments and string literals.
    ProofFree,
}

/// Classify one proof-source body by content, never by file extension.
///
/// The smoke tag is matched on raw content because it lives in comments;
/// proof tokens are matched on comment- and string-stripped code.
#[must_use]
pub fn classify_source(content: &str) -> SourceClass {
    if is_smoke_tagged(content) {
        SourceClass::SmokeOnly
    } else if has_proof_token(content) {
        SourceClass::GenuineProof
    } else {
        SourceClass::ProofFree
    }
}

/// One discovered candidate proof source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredSource {
    /// Workspace-relative source path.
    pub path: String,
    /// `true` when the source is production scope and needs disposition.
    pub production: bool,
}

/// Smoke marker matched on raw content because it lives in comments.
const SMOKE_MARKER: &str = "fixture-smoke";

/// Proof tokens matched as whole words in stripped code.
const PROOF_TOKENS: &[&str] = &[
    "proof",
    "spec",
    "ensures",
    "requires",
    "invariant",
    "assert",
    "assume_specification",
    "verus",
    "kani",
    "flux",
    "ghost",
    "tracked",
];

/// `true` for ASCII word characters; word boundaries delimit tokens.
pub(crate) const fn is_word_char(next: char) -> bool {
    next.is_ascii_alphanumeric() || next == '_'
}

/// `true` when the finished word is a proof token.
fn is_proof_word(word: &str) -> bool {
    PROOF_TOKENS.contains(&word)
}

/// Fold a finished word into the hit flag and clear the accumulator.
fn check_word(word: &mut String, found: bool) -> bool {
    let hit = is_proof_word(word);
    word.clear();
    found || hit
}

/// Word-buffer action for one scanned character.
#[derive(Debug, Clone, Copy)]
pub(crate) enum WordAction {
    /// Character carries no word content.
    None,
    /// Append the character to the current word.
    Push(char),
    /// Finish the current word against the token table.
    Check,
    /// Drop the current word; a raw-string opening consumed it.
    Clear,
}

/// Apply one word-buffer action, returning the updated hit flag.
pub(crate) fn apply_word_action(word: &mut String, action: WordAction, found: bool) -> bool {
    match action {
        WordAction::None => found,
        WordAction::Push(next) => {
            word.push(next);
            found
        }
        WordAction::Check => check_word(word, found),
        WordAction::Clear => {
            word.clear();
            found
        }
    }
}

/// `true` when the body carries the explicit non-authoritative smoke tag.
fn is_smoke_tagged(content: &str) -> bool {
    content.contains(SMOKE_MARKER)
}

/// `true` when stripped code carries at least one whole-word proof token.
fn has_proof_token(content: &str) -> bool {
    let (_, word, found) = content.chars().fold(
        (Scan::code(), String::new(), false),
        |(mode, mut word, found), next| {
            let (next_mode, action) = scan_step(mode, next);
            let found = apply_word_action(&mut word, action, found);
            (next_mode, word, found)
        },
    );
    found || is_proof_word(&word)
}
