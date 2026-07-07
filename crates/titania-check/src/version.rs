//! Build-time constants for the `titania-check` binary.
//!
//! `build.rs` provides optional `rustc-env` values. The constants fall back to
//! stable literals when a sandboxed build cannot provide metadata.

/// Short git SHA captured at build time, or `"unknown"` when unavailable.
pub const BUILD_GIT_SHA: &str = match option_env!("TITANIA_BUILD_GIT_SHA") {
    Some(value) => value,
    None => "unknown",
};

/// Workspace name read from build metadata, or `"titania"` when unavailable.
pub const WORKSPACE_NAME: &str = match option_env!("TITANIA_WORKSPACE_NAME") {
    Some(value) => value,
    None => "titania",
};
