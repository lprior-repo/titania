use super::scanner::{Scan, scan_step};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceClass {
    GenuineProof,
    SmokeOnly,
    ProofFree,
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredSource {
    pub path: String,
    pub production: bool,
}

const SMOKE_MARKER: &str = "fixture-smoke";

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

pub(crate) const fn is_word_char(next: char) -> bool {
    next.is_ascii_alphanumeric() || next == '_'
}

fn is_proof_word(word: &str) -> bool {
    PROOF_TOKENS.contains(&word)
}

fn check_word(word: &mut String, found: bool) -> bool {
    let hit = is_proof_word(word);
    word.clear();
    found || hit
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum WordAction {
    None,
    Push(char),
    Check,
    Clear,
}

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

fn is_smoke_tagged(content: &str) -> bool {
    content.contains(SMOKE_MARKER)
}

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
