use std::{
    path::{Path, PathBuf},
    process::ExitCode,
};

use titania_lanes::{
    Finding, LaneExit, LaneReport, SourceLine, current_target_project, exit,
    helpers::{line_no_from_idx, relative_path},
};

/// Default forbidden tokens (Holzman Rust slice 1).
///
/// Tokens are stored as their canonical surface (`panic!`, `unwrap`,
/// `expect`, `todo!`, `unimplemented!`, `dbg!`). Macro tokens match as
/// raw substrings because the `!` is part of the macro syntax. Method
/// tokens (`unwrap`, `expect`) match only when preceded by a method
/// receiver (`.` or `::`) so we do not false-positive on identifiers
/// like `myexpect`.
const DEFAULT_FORBIDDEN: &[&str] =
    &["panic!", "unwrap", "expect", "todo!", "unimplemented!", "dbg!"];
const FORBIDDEN_FLAG: &str = "--forbidden=";
pub fn main_exit(args: &[String]) -> ExitCode {
    let forbidden = parse_forbidden(args);
    let root = match target_root() {
        Ok(root) => root,
        Err(code) => return code,
    };
    emit_scan_header(&root, &forbidden);
    scan_and_exit(&root, &forbidden)
}

/// Resolve the target project root directory.
///
/// # Errors
/// Returns [`LaneExit::Usage`] when the current directory cannot be read
/// or no valid Cargo target project can be discovered from it
/// (see [`current_target_project`]).
fn target_root() -> Result<PathBuf, ExitCode> {
    current_target_project().map(|target| target.as_std_path().to_path_buf()).map_err(|error| {
        eprintln!("[forbidden-scan] cannot resolve target project: {error}");
        exit(LaneExit::Usage)
    })
}

fn emit_scan_header(root: &Path, forbidden: &[ForbiddenToken]) {
    eprintln!("CWD: {}", root.display());
    eprintln!("ScanDomain: crates/*/src");
    eprintln!(
        "ForbiddenTokens: {}",
        forbidden.iter().map(ForbiddenToken::as_str).collect::<Vec<_>>().join(",")
    );
}

fn scan_and_exit(root: &Path, forbidden: &[ForbiddenToken]) -> ExitCode {
    let mut report = LaneReport::new();
    for file in collect_source_files(root) {
        scan_file(root, &file, forbidden, &mut report);
    }
    eprint!("{}", report.render());
    if report.is_clean() { clean_exit() } else { violations_exit() }
}

fn clean_exit() -> ExitCode {
    eprintln!("NoViolationFound");
    exit(LaneExit::Clean)
}

fn violations_exit() -> ExitCode {
    eprintln!("ViolationFound: forbidden token surface is non-empty");
    exit(LaneExit::Violations)
}

fn parse_forbidden(args: &[String]) -> Vec<ForbiddenToken> {
    let override_set = args
        .iter()
        .find(|arg| arg.starts_with(FORBIDDEN_FLAG))
        .map(|arg| parse_override_set(arg.as_str()));
    match override_set {
        Some(set) if !set.is_empty() => set,
        Some(_) | None => default_forbidden_set(),
    }
}

fn parse_override_set(arg: &str) -> Vec<ForbiddenToken> {
    let body = arg.strip_prefix(FORBIDDEN_FLAG).map_or("", core::convert::identity);
    body.split(',').filter_map(ForbiddenToken::parse).collect()
}

fn default_forbidden_set() -> Vec<ForbiddenToken> {
    DEFAULT_FORBIDDEN.iter().filter_map(|s| ForbiddenToken::parse(s)).collect()
}

fn collect_source_files(root: &Path) -> Vec<PathBuf> {
    let crates_dir = root.join("crates");
    let Ok(entries) = std::fs::read_dir(crates_dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .map(|e| e.path().join("src"))
        .filter(|p| p.is_dir())
        .flat_map(walk_rust_files)
        .collect()
}

fn walk_rust_files(dir: PathBuf) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let mut stack: Vec<PathBuf> = vec![dir];
    while let Some(top) = stack.pop() {
        append_rust_files(&top, &mut stack, &mut out);
    }
    out.sort();
    out
}

fn append_rust_files(top: &Path, stack: &mut Vec<PathBuf>, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(top) else {
        return;
    };
    for entry in entries.filter_map(Result::ok).map(|e| e.path()) {
        record_path(entry, stack, out);
    }
}

fn record_path(path: PathBuf, stack: &mut Vec<PathBuf>, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        stack.push(path);
    } else if path.extension().is_some_and(|e| e == "rs") {
        out.push(path);
    }
}

fn scan_file(root: &Path, path: &Path, forbidden: &[ForbiddenToken], report: &mut LaneReport) {
    report.record_scan();
    let Ok(content) = std::fs::read_to_string(path) else {
        return;
    };
    let display = relative_path(root, path);
    scan_content(&content, &display, forbidden, report);
}

fn scan_content(
    content: &str,
    display: &str,
    forbidden: &[ForbiddenToken],
    report: &mut LaneReport,
) {
    let mut block_comment = false;
    for (idx, line) in content.lines().enumerate() {
        let source_line = SourceLine::parse(line, &mut block_comment);
        scan_source_line(&source_line, idx, display, forbidden, report);
    }
}

fn scan_source_line(
    line: &SourceLine,
    idx: usize,
    display: &str,
    forbidden: &[ForbiddenToken],
    report: &mut LaneReport,
) {
    if line.is_non_code() {
        return;
    }
    let line_no = line_no_from_idx(idx);
    forbidden.iter().filter(|token| token.is_present_in(line.code())).for_each(|token| {
        report.push(Finding::new(
            "FORBIDDEN-001",
            display,
            line_no,
            format!("forbidden token `{}`", token.as_str()),
        ));
    });
}

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
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        let kind = if trimmed.ends_with('!') { TokenKind::Macro } else { TokenKind::Method };
        Some(Self { name: trimmed.to_owned(), kind })
    }

    fn as_str(&self) -> &str {
        &self.name
    }

    /// Check whether the match at `idx` is a real surface occurrence
    /// (per [`TokenKind`]) rather than a substring of a larger
    /// identifier.
    fn matches_at(&self, code: &str, idx: usize) -> bool {
        matches_token(code, &self.name, self.kind, idx)
    }

    /// Check whether this token is present anywhere in `code`.
    #[expect(
        clippy::excessive_nesting,
        reason = "Simple linear search loop with early return; extracted matches_token to limit nesting of the check itself."
    )]
    fn is_present_in(&self, code: &str) -> bool {
        let mut search_start = 0usize;
        while let Some(idx) = code.get(search_start..).and_then(|tail| tail.find(&self.name)) {
            if self.matches_at(code, idx) {
                return true;
            }
            search_start = idx.saturating_add(1);
        }
        false
    }
}

/// Decide whether the match at `idx` is a real surface occurrence
/// (per [`TokenKind`]) rather than a substring of a larger
/// identifier.
fn matches_token(code: &str, name: &str, kind: TokenKind, idx: usize) -> bool {
    let bytes = code.as_bytes();
    let name_len = name.len();
    let after = idx.saturating_add(name_len);
    match kind {
        TokenKind::Macro => {
            // Reject identifier-prefix matches: the byte before
            // the match (if any) must not be alphanumeric/underscore.
            idx == 0 || bytes.get(idx.wrapping_sub(1)).is_none_or(|b| !is_word_byte(*b))
        }
        TokenKind::Method => {
            // Require a method-call receiver directly before:
            // `.unwrap` or `::unwrap` (e.g. `Result::unwrap(...)`).
            // Reject identifier-prefix matches so `myexpect` is not
            // flagged.
            let before_ok = match bytes.get(idx.wrapping_sub(1)) {
                Some(b'.' | b':') => true,
                Some(b) if is_word_byte(*b) => return false,
                _ => idx == 0,
            };
            let after_ok = bytes.get(after).is_some_and(|b| *b == b'(');
            before_ok && after_ok
        }
    }
}

const fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
