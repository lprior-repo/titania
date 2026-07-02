//! Enforces the test-integrity rule: every behavior test must keep a
//! matching non-`#[ignore]` exact assertion across revisions.
//!
//! Rust re-implementation of the bash lane `scripts/check-test-integrity.sh`. Run via
//! `cargo run --bin check-test-integrity -- [--self-test] [--base <rev>]`
//! from the repository root, or via the matching Moon task in
//! `.moon/tasks/all.yml`.
//!
//! Scan domain: changes since the base revision (default `HEAD`/`@-`) for
//! files that contain tests, plus full file deletion events. Exclusions:
//! generated build outputs, `target/`, and deleted files that never held
//! a `#[test]` declaration or exact assertion.
//!
//! `--self-test` runs in-process fixtures against scratch git repositories.
//! `--base <rev>` overrides the default base; the default honours
//! `TEST_INTEGRITY_BASE`. The lane integrates with the workspace by shelling
//! out to `git` or `jj`; when neither VCS is present it exits `LaneExit::Failure`.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

#[path = "check_test_integrity/mod.rs"]
/// Test-integrity lane implementation.
pub mod check_test_integrity;

fn main() -> std::process::ExitCode {
    check_test_integrity::main_exit()
}
