//! Enumerates cargo xtask loom models from the "Available models:" JSON array.
//!
//! Rust re-implementation of the bash lane `scripts/loom-list.sh`. Run via
//! `cargo run --bin loom_list` from the repository root or via the matching
//! Moon task in `.moon/tasks/all.yml`.
//!
//! Exit codes: 0 = clean or not applicable, 1 = parse/exec failure,
//! 2 = usage error.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

include!("loom_list/lane.rs");

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    main_exit(&args)
}
