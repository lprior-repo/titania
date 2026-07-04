//! Strict-ai policy defaults for titania-check.
//!
//! All defaults are embedded in the binary at compile time. No filesystem
//! I/O is required to load them. Policy overrides are read from checked-in
//! `policy.toml` and `exceptions.toml` files, but the binary ships with
//! correct defaults regardless.
//!
//! See `crates/titania-policy/tests/defaults.rs` for the assertion tests.

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

use serde::{Deserialize, Serialize};

/// Embedded architecture policy for the strict-ai profile.
///
/// These values are the binary defaults. They can be overridden by a
/// checked-in `policy.toml` but are always available without filesystem
/// access.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchitecturePolicy {
    /// Directories that count as "core" for architecture import rules.
    pub core_dirs: Vec<String>,
    /// Crate names considered "infrastructure" (forbid imports from core).
    pub infra_crates: Vec<String>,
}

/// The embedded strict-ai policy defaults.
///
/// Constructed from compile-time constants. Calling [`Self::embedded`]
/// never performs filesystem I/O.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyDefaults {
    /// Schema version of the policy file format.
    pub schema_version: u32,
    /// Profile name (e.g. "strict-ai").
    pub profile_name: String,
    /// Source documents that defined these defaults.
    pub sources: Vec<String>,
    /// Architecture-specific sub-policy.
    pub architecture: ArchitecturePolicy,
    /// `true` when loaded from the embedded binary (no filesystem access).
    pub embedded: bool,
}

impl PolicyDefaults {
    /// Return the embedded binary defaults.
    ///
    /// This function performs no filesystem I/O. It returns the compiled
    /// strict-ai baseline that Titania applies before target-local
    /// `.titania/profiles/strict-ai/` overrides are loaded.
    #[must_use]
    pub fn embedded() -> Self {
        Self {
            schema_version: 1,
            profile_name: String::from("strict-ai"),
            sources: vec![String::from("v1-spec.md"), String::from("AGENTS.md")],
            architecture: ArchitecturePolicy {
                core_dirs: vec![
                    String::from("src/core"),
                    String::from("src/domain"),
                    String::from("crates/*-core/src"),
                ],
                infra_crates: vec![
                    String::from("tokio"),
                    String::from("axum"),
                    String::from("sqlx"),
                    String::from("reqwest"),
                ],
            },
            embedded: true,
        }
    }

    /// Return `true` when these defaults were loaded from the embedded
    /// binary without filesystem access.
    #[must_use]
    pub const fn no_fs_access(&self) -> bool {
        self.embedded
    }

    /// Return the list of source documents that defined these defaults.
    #[must_use]
    pub fn sources(&self) -> &[String] {
        &self.sources
    }
}
