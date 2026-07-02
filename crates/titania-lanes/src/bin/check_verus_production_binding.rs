//! ABSOLUTE gate: rejects Verus proof claims without production binding.
//!
//! Explicit fixture smoke files may opt out with a typed `NotApplicable`
//! classification; all real proof files still require `#[path]` plus
//! `assume_specification` binding.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

include!("check_verus_production_binding/lane.rs");

fn main() -> std::process::ExitCode {
    main_exit()
}
