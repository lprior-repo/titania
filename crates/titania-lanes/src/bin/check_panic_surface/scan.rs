use std::path::Path;

use super::{
    Finding, LaneReport, PANIC_MACROS, PANIC_SURFACE_RULE, PanicMacroRule, RuleId, SourceLine,
    paths::rel_str,
};
use crate::SourceLineState;

/// Scan one Rust source file and append any panic-surface findings.
pub(super) fn scan_file(root: &Path, path: &Path, report: &mut LaneReport) {
    report.record_scan();
    let Ok(content) = std::fs::read_to_string(path) else {
        return;
    };

    let display = rel_str(root, path);
    let mut state = ScanState::default();
    let mut sink = ScanSink { display: &display, report };
    content.lines().enumerate().for_each(|(idx, raw)| {
        state.scan_line(raw, line_no_from_idx(idx), &mut sink);
    });
}

struct ScanSink<'a> {
    display: &'a str,
    report: &'a mut LaneReport,
}

#[derive(Default)]
struct ScanState {
    cfg_depth: u32,
    kani_proof_depth: u32,
    global_depth: u32,
    cfg_scope_depths: Vec<u32>,
    kani_scope_depths: Vec<u32>,
    state: SourceLineState,
}

impl ScanState {
    fn scan_line(&mut self, raw: &str, line_no: u32, sink: &mut ScanSink<'_>) {
        scan_state_line(self, raw, line_no, sink);
    }
}

fn scan_state_line(state: &mut ScanState, raw: &str, line_no: u32, sink: &mut ScanSink<'_>) {
    let parsed = SourceLine::parse(raw, &mut state.state);
    if parsed.is_non_code() {
        apply_brace_delta(state, raw.trim_start());
        return;
    }
    let trimmed = raw.trim_start();
    let openings = ScopeOpenings::from_trimmed(trimmed);
    enter_scopes(state, openings);
    push_panic_macro_if_present(state, parsed.code(), line_no, sink);
    apply_brace_delta(state, trimmed);
    close_scopes(state, openings);
}

fn push_panic_macro_if_present(
    state: &ScanState,
    code: &str,
    line_no: u32,
    sink: &mut ScanSink<'_>,
) {
    let Some(macro_rule) = first_panic_macro(code).filter(|_| !inside_test_or_kani(state)) else {
        return;
    };
    let Ok(rule) = RuleId::new(macro_rule.rule_id()) else {
        return;
    };
    sink.report.push(Finding::new(
        rule,
        sink.display,
        line_no,
        format!(
            "production panic/assert macro `{}` is forbidden ({PANIC_SURFACE_RULE})",
            macro_rule.macro_name()
        ),
    ));
}

fn enter_scopes(state: &mut ScanState, openings: ScopeOpenings) {
    enter_scope(
        openings.cfg,
        &mut state.cfg_depth,
        &mut state.cfg_scope_depths,
        state.global_depth,
    );
    enter_scope(
        openings.kani_proof,
        &mut state.kani_proof_depth,
        &mut state.kani_scope_depths,
        state.global_depth,
    );
}

fn enter_scope(
    opening: ScopeOpened,
    depth: &mut u32,
    scope_depths: &mut Vec<u32>,
    global_depth: u32,
) {
    if !opening.is_open() {
        return;
    }
    *depth = depth.saturating_add(1);
    scope_depths.push(global_depth.saturating_add(1));
}

fn close_scopes(state: &mut ScanState, openings: ScopeOpenings) {
    close_scope(
        openings.cfg,
        &mut state.cfg_depth,
        &mut state.cfg_scope_depths,
        state.global_depth,
    );
    close_scope(
        openings.kani_proof,
        &mut state.kani_proof_depth,
        &mut state.kani_scope_depths,
        state.global_depth,
    );
}

fn close_scope(
    opening: ScopeOpened,
    depth: &mut u32,
    scope_depths: &mut Vec<u32>,
    global_depth: u32,
) {
    if opening.is_open() || !scope_closed(global_depth, scope_depths) {
        return;
    }
    if scope_depths.pop().is_some() {
        *depth = depth.saturating_sub(1);
    }
}

fn scope_closed(global_depth: u32, depths: &[u32]) -> bool {
    depths.last().is_some_and(|target| global_depth < *target)
}

const fn inside_test_or_kani(state: &ScanState) -> bool {
    state.cfg_depth > 0 || state.kani_proof_depth > 0
}

fn apply_brace_delta(state: &mut ScanState, line: &str) {
    state.global_depth = state.global_depth.saturating_add_signed(line_brace_delta(line));
}

#[derive(Clone, Copy)]
struct ScopeOpenings {
    cfg: ScopeOpened,
    kani_proof: ScopeOpened,
}

impl ScopeOpenings {
    fn from_trimmed(trimmed: &str) -> Self {
        let cfg = is_cfg_attr_open(trimmed, &["test", "kani"]);
        Self {
            cfg: scope_opened(cfg),
            kani_proof: scope_opened(!cfg && trimmed.starts_with("#[kani::proof]")),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScopeOpened {
    Yes,
    No,
}

impl ScopeOpened {
    const fn is_open(self) -> bool {
        matches!(self, Self::Yes)
    }
}

const fn scope_opened(value: bool) -> ScopeOpened {
    if value { ScopeOpened::Yes } else { ScopeOpened::No }
}

/// Net `{` - `}` count for a line: positive when more open than close.
fn line_brace_delta(line: &str) -> i32 {
    let opens =
        i32::try_from(line.bytes().filter(|b| *b == b'{').count()).map_or(i32::MAX, |value| value);
    let closes =
        i32::try_from(line.bytes().filter(|b| *b == b'}').count()).map_or(i32::MAX, |value| value);
    opens.saturating_sub(closes)
}

fn is_cfg_attr_open(line: &str, scopes: &[&str]) -> bool {
    let Some(rest) = line.strip_prefix("#[cfg(") else {
        return false;
    };
    let Some(inside) = rest.strip_suffix(")]") else {
        return false;
    };
    scopes.iter().any(|s| inside.split(',').any(|p| p.trim() == *s))
}

fn first_panic_macro(line: &str) -> Option<PanicMacroRule> {
    PANIC_MACROS.iter().copied().find(|rule| panic_macro_matches(line, rule.macro_name()))
}

fn panic_macro_matches(line: &str, macro_name: &str) -> bool {
    let Some(idx) = line.find(macro_name) else {
        return false;
    };
    let bytes = line.as_bytes();
    let before_ok = idx == 0 || bytes.get(idx.wrapping_sub(1)).is_none_or(|b| !is_word_byte(*b));
    let Some(after_idx) = idx.checked_add(macro_name.len()) else {
        return false;
    };
    let after_ok = bytes.get(after_idx).is_none_or(|b| !is_word_byte(*b));
    before_ok && after_ok
}

const fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Convert a 0-indexed line offset into a 1-indexed `u32` line number.
///
/// Saturates at `u32::MAX` on overflow. This mirrors
/// `titania_lanes::helpers::line_no_from_idx` locally.
fn line_no_from_idx(idx: usize) -> u32 {
    u32::try_from(idx.saturating_add(1)).map_or(u32::MAX, |value| value)
}
