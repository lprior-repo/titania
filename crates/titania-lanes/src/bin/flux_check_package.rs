//! `cargo flux` wrapper that rejects unsupported target selectors.
//!
//! Rust re-implementation of the bash lane `scripts/flux-check-package.sh`. Run via
//! `cargo run --bin flux-check-package -- <package> [cargo-flux options]`
//! from the repository root, or via the matching Moon task in
//! `.moon/tasks/all.yml`.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

include!("flux_check_package/lane.rs");

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    main_exit(&args)
}
