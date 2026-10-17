//! String-backed detectors for rules ast-grep cannot express.
//!
//! Real ast-grep powers the structural FUNC_* rules (see [`super::super::engine`]).
//! Two rule families stay string-backed here:
//!
//! * `BYPASS_INLINE_SUPPRESSION` — comments are not AST nodes, so ast-grep
//!   cannot see `// ast-grep-ignore` / `// sg-ignore`.
//! * `ARCHITECTURE_IMPORT_CORE_*` and `ARCHITECTURE_EFFECT_CORE_*` — each rule
//!   couples a path-scope filter (file under `core/` / `domain/`) with
//!   code-text detection that covers `use` imports (direct, grouped,
//!   multiline, `as`-aliased), fully-qualified paths without imports
//!   (`std::fs::read_to_string` with no `use`), and macro bodies (scanned
//!   as code text). Boundary-name exclusions hold (`fs_extra` is not `fs`,
//!   `brand::thread_rng` is not `rand::thread_rng`). Expressing all of that
//!   as one ast-grep pattern requires a path-aware selector the engine does
//!   not own; the hand-rolled detector stays until a path-aware port lands.
//!
//! Detection is comment- and string-blind: every detector scans
//! [`code_only_source`], so mentions inside `//` comments, `/* */` blocks,
//! and string/char literals never fire. `include_str!` of a frozen catalog
//! and `std::sync::LazyLock` memoization are intentionally NOT violations:
//! the former is compile-time deterministic input, the latter is
//! write-once immutable caching (see `ARCHITECTURE_EFFECT_CORE_STATE`).

mod code_scan;

use code_scan::{code_only_source, detect_code_line};

pub(super) fn detect_inline_suppression(source: &str) -> bool {
    source.lines().any(line_comment_contains_inline_suppression)
}

fn line_comment_contains_inline_suppression(line: &str) -> bool {
    line.split_once("//")
        .map(|(_, comment)| comment)
        .is_some_and(contains_inline_suppression_marker)
        || line
            .split_once("/*")
            .map(|(_, comment)| comment)
            .is_some_and(contains_inline_suppression_marker)
}

fn contains_inline_suppression_marker(comment: &str) -> bool {
    comment.contains("ast-grep-ignore") || comment.contains("sg-ignore")
}

pub(super) fn detect_core_infra_import(source: &str) -> bool {
    detect_code_line(source, |line| {
        line.contains("use tokio::")
            || line.contains("use axum::")
            || line.contains("use sqlx::")
            || line.contains("use reqwest::")
    }) || fully_qualified_uses_path(source, &["tokio::", "axum::", "sqlx::", "reqwest::"])
}

pub(super) fn detect_core_fs_import(source: &str) -> bool {
    direct_import_contains(source, "std::", &["fs", "env", "net"])
        || grouped_std_import_contains(source, &["fs", "env", "net"])
        || fully_qualified_uses_path(source, &["std::fs::", "std::env::", "std::net::"])
}

pub(super) fn detect_core_time_import(source: &str) -> bool {
    direct_import_contains(source, "std::time::", &["SystemTime", "Instant"])
        || grouped_import_contains(source, "use std::time::", &["SystemTime", "Instant"])
        || fully_qualified_uses_path(source, &["std::time::SystemTime", "std::time::Instant"])
}

pub(super) fn detect_core_random_import(source: &str) -> bool {
    direct_import_contains(source, "rand::", &["thread_rng", "Rng"])
        || grouped_rand_import_contains(source, &["thread_rng", "Rng"])
        || fully_qualified_uses_path(source, &["rand::thread_rng", "rand::Rng"])
}

/// Detect process-spawning capability in core code.
///
/// Covers `use std::process::Command` (direct, grouped, `as`-aliased),
/// fully-qualified `std::process::Command::new` with no import, the
/// `tokio::process` async counterpart, and the `std::os::{unix,windows}`
/// process extensions. Shell code behind a typed port is out of scope:
/// the lane's path filter keeps this detector on core/domain paths only.
pub(super) fn detect_core_process_import(source: &str) -> bool {
    direct_import_contains(source, "std::", &["process"])
        || grouped_std_import_contains(source, &["process"])
        || fully_qualified_uses_path(
            source,
            &[
                "std::process::",
                "tokio::process",
                "std::os::unix::process",
                "std::os::windows::process",
            ],
        )
}

/// Detect async-runtime effects in core code.
///
/// Core stays synchronous: `async fn`, `.await`, and direct use of the
/// Tokio task/time runtime (`tokio::spawn`, `tokio::task`, `tokio::time`)
/// all belong to the shell behind a typed port. Comment and string
/// mentions never fire.
pub(super) fn detect_core_async_effect(source: &str) -> bool {
    let code = code_only_source(source);
    code_contains_token(&code, "async fn")
        || code.contains(".await")
        || fully_qualified_uses_path(source, &["tokio::spawn", "tokio::task", "tokio::time"])
}

/// Detect ambient mutable state in core code.
///
/// Banned: `static mut`, blocking/global locks (`std::sync::Mutex`,
/// `std::sync::RwLock`), interior mutability cells (`std::cell::Cell`,
/// `std::cell::RefCell`, `std::cell::UnsafeCell`), and ambient thread
/// control (`std::thread::spawn`, `std::thread::sleep`).
///
/// Explicitly allowed: `std::sync::LazyLock` / `std::sync::OnceLock`
/// write-once immutable memoization over frozen compile-time input
/// (the repair-catalog pattern), and `include_str!` of frozen files.
/// Neither appears in the banned list by construction.
pub(super) fn detect_core_state_effect(source: &str) -> bool {
    fully_qualified_uses_path(
        source,
        &[
            "static mut",
            "std::sync::Mutex",
            "std::sync::RwLock",
            "std::cell::Cell",
            "std::cell::RefCell",
            "std::cell::UnsafeCell",
            "std::thread::spawn",
            "std::thread::sleep",
        ],
    )
}

/// `true` when comment/string-stripped `source` uses any of `needles` as a
/// real path token (not a substring of a longer identifier).
///
/// `needles` ending in `::` (e.g. `std::fs::`) need only a leading
/// boundary: the trailing `::` already pins the segment. Other needles
/// (e.g. `rand::Rng`, `static mut`) need both boundaries so `rand::RngCore`
/// and `static mutable` do not fire.
fn fully_qualified_uses_path(source: &str, needles: &[&str]) -> bool {
    let code = code_only_source(source);
    needles.iter().any(|needle| code_contains_token(&code, needle))
}

/// `true` when `needle` occurs in `code` on identifier boundaries.
fn code_contains_token(code: &str, needle: &str) -> bool {
    code.match_indices(needle).any(|(start, _)| {
        leading_boundary_ok(code, start) && trailing_boundary_ok(code, start, needle)
    })
}

/// `true` when the byte before `start` cannot extend an identifier.
fn leading_boundary_ok(code: &str, start: usize) -> bool {
    code.split_at_checked(start).is_some_and(|(before, _)| {
        before.chars().next_back().is_none_or(|prev| !is_ident_char(prev))
    })
}

/// `true` when the byte after the needle cannot extend an identifier.
///
/// Needles ending in `::` skip the trailing check: the `::` terminator is
/// already an exact segment boundary.
fn trailing_boundary_ok(code: &str, start: usize, needle: &str) -> bool {
    needle.ends_with("::")
        || code
            .split_at_checked(start.saturating_add(needle.len()))
            .is_none_or(|(_, after)| after.chars().next().is_none_or(|next| !is_ident_char(next)))
}

/// `true` for ASCII alphanumerics and `_` (identifier continuations).
fn is_ident_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn direct_import_contains(source: &str, path_prefix: &str, names: &[&str]) -> bool {
    let code = code_only_source(source);
    code.match_indices("use ")
        .filter_map(|(start, _)| direct_import_member(&code, start, path_prefix))
        .any(|member| names.iter().any(|name| grouped_member_matches(member, name)))
}

fn direct_import_member<'a>(source: &'a str, start: usize, path_prefix: &str) -> Option<&'a str> {
    let (_, from_use) = source.split_at_checked(start)?;
    use_item_prefix_allowed(source, start).then_some(())?;
    Some(from_use.strip_prefix("use ")?.strip_prefix(path_prefix)?.split_once(';')?.0.trim())
}

fn use_item_prefix_allowed(source: &str, start: usize) -> bool {
    source
        .split_at_checked(start)
        .map(|(before, _)| before)
        .map(current_line_prefix)
        .is_some_and(visibility_prefix_allows_use)
}

fn current_line_prefix(before_use: &str) -> &str {
    before_use.rsplit_once('\n').map_or(before_use, |(_, line)| line).trim_start()
}

fn visibility_prefix_allows_use(prefix: &str) -> bool {
    let prefix = prefix.trim_end();
    prefix.is_empty()
        || prefix == "pub"
        || prefix.strip_prefix("pub(").is_some_and(|rest| rest.ends_with(')'))
}

fn grouped_std_import_contains(source: &str, names: &[&str]) -> bool {
    grouped_import_contains(source, "use std::", names)
}

fn grouped_rand_import_contains(source: &str, names: &[&str]) -> bool {
    grouped_import_contains(source, "use rand::", names)
}

fn grouped_import_contains(source: &str, prefix: &str, names: &[&str]) -> bool {
    let code = code_only_source(source);
    code.match_indices(prefix)
        .filter_map(|(start, _)| grouped_import_body(&code, start, prefix))
        .any(|body| names.iter().any(|name| grouped_body_contains_name(body, name)))
}

fn grouped_import_body<'a>(source: &'a str, start: usize, prefix: &str) -> Option<&'a str> {
    let (_, from_prefix) = source.split_at_checked(start)?;
    use_item_prefix_allowed(source, start).then_some(())?;
    Some(from_prefix.strip_prefix(prefix)?.trim_start().strip_prefix('{')?.split_once('}')?.0)
}

fn grouped_body_contains_name(body: &str, name: &str) -> bool {
    body.split(',').map(str::trim).any(|member| grouped_member_matches(member, name))
}

fn grouped_member_matches(member: &str, name: &str) -> bool {
    member
        .strip_prefix(name)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with("::") || rest.starts_with(" as "))
}
