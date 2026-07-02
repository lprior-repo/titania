//! Validates Cargo workspace shape: members, package names, and forbidden
//! dependencies / feature names.
//!
//! Rust re-implementation of the bash lane `scripts/check-workspace-assertions.sh`. Run via
//! `cargo run --bin check-workspace-assertions --` from the repository root
//! or via the matching Moon task in `.moon/tasks/all.yml`.
//!
//! Scan domain: the workspace `Cargo.toml` and one manifest per workspace
//! member under `crates/`. Exclusions: `target/`, `target/miri-tmp`, and
//! any `vb_ui` / `fuzz` legacy artifacts.
//!
//! This port inherits the same rule shape as the legacy bash original
//! (boundary crates may not depend on UI crates, runtime format crates, or
//! use a fixed list of forbidden feature names) but reads its expected
//! member list, package-name map, and feature table from the live titania
//! workspace. That keeps the lane a pure structural assertion: it does
//! NOT depend on a legacy `vb_*` crate set.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

/// Workspace assertion implementation for the binary entry point.
#[path = "check_workspace_assertions/mod.rs"]
pub mod workspace_assertions;

fn main() -> std::process::ExitCode {
    workspace_assertions::main_exit()
}
