//! Shared helpers for the titania-lanes bin implementations.

#![allow(
    clippy::implicit_saturating_sub,
    reason = "saturating_sub is the intended contract."
)]
#![allow(
    clippy::only_used_in_recursion,
    reason = "recursive helpers are reviewed individually."
)]
#![allow(
    clippy::manual_unwrap_or_default,
    reason = "match-style defaults are required for typed-error recovery."
)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]
#![deny(clippy::string_slice)]
#![deny(clippy::get_unwrap)]
#![deny(clippy::arithmetic_side_effects)]
#![deny(clippy::dbg_macro)]
#![deny(clippy::as_conversions)]
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

/// Convert a 0-indexed byte offset into a 1-indexed `u32` line number, with
/// overflow clamped to `u32::MAX` to keep the helper total.
#[must_use]
pub fn line_no_from_idx(idx: usize) -> u32 {
    u32::try_from(idx.saturating_add(1)).map_or(u32::MAX, |v| v)
}

/// Saturating add that returns [`usize::MAX`] on overflow instead of
/// panicking in debug builds.
#[must_use]
pub fn saturating_add_usize(a: usize, b: usize) -> usize {
    a.checked_add(b).map_or(usize::MAX, |v| v)
}

/// Compute the net brace delta of `text` (open braces minus close braces).
/// Block comments and string literals are not currently honored; callers
/// must pre-strip them or use a different helper.
#[must_use]
pub fn brace_delta(text: &str) -> i32 {
    let mut delta: i32 = 0;
    for &b in text.as_bytes() {
        if b == b'{' {
            delta = delta.saturating_add(1);
        } else if b == b'}' {
            delta = delta.saturating_sub(1);
        }
    }
    delta
}

/// Borrow the input with leading whitespace removed.
#[must_use]
pub fn strip_leading_whitespace(s: &str) -> &str {
    s.trim_start()
}

/// Borrow the input with leading and trailing whitespace removed.
#[must_use]
pub fn strip_whitespace(s: &str) -> &str {
    s.trim()
}

/// Render `p` as a forward-slash path string, regardless of host separator.
#[must_use]
pub fn normalize_slashes(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// Render `p` relative to `root` if possible, otherwise the normalized
/// absolute path. The result always uses forward slashes.
#[must_use]
pub fn relative_path(root: &Path, p: &Path) -> String {
    p.strip_prefix(root).map_or_else(|_| normalize_slashes(p), normalize_slashes)
}

/// Recursively walk `dir` and append every `*.rs` file to `out` using paths
/// relative to `root`. Symlinks and `.git` directories are skipped.
pub fn walk_rs_files(dir: &Path, root: &Path, out: &mut Vec<PathBuf>) {
    let Some(entries) = read_dir_skip(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if name.as_os_str() == ".git" {
            continue;
        }
        let Some(file_type) = entry.file_type().ok() else { continue };
        if file_type.is_dir() {
            walk_rs_files(&path, root, out);
        } else if file_type.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
            out.push(relative_path(root, &path).into());
        }
    }
}

/// Wrap `std::fs::read_dir` so the caller can early-return on error.
fn read_dir_skip(dir: &Path) -> Option<std::fs::ReadDir> {
    std::fs::read_dir(dir).ok()
}

/// Validated 1-indexed source line number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct LineNo(pub u32);

impl LineNo {
    /// Construct a validated `LineNo` from a 1-indexed line number. Returns
    /// `None` for zero.
    #[must_use]
    pub const fn new(n: u32) -> Option<Self> {
        if n == 0 { None } else { Some(Self(n)) }
    }

    /// Borrow the inner `u32` line number.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Saturating subtraction for `usize`. Returns `0` on underflow.
#[must_use]
pub const fn line_diff(start: usize, end: usize) -> usize {
    end.saturating_sub(start)
}

/// Invoke `f` for every byte of `text` until either the slice is exhausted
/// or `f` returns `false`. The first `false` short-circuits the iteration.
pub fn for_each_byte<F: FnMut(u8) -> bool>(text: &str, mut f: F) {
    for &b in text.as_bytes() {
        if !f(b) {
            return;
        }
    }
}