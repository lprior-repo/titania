//! Cargo lane argument construction.
//!
//! Holds static argument tables that drive every cargo sub-command invocation.

use super::CargoLane;

const FMT_ARGS: &[&str] = &["fmt", "--all", "--check"];
const COMPILE_ARGS: &[&str] = &["check", "--workspace", "--frozen"];
const CLIPPY_ARGS: &[&str] = &[
    "clippy",
    "--workspace",
    "--lib",
    "--bins",
    "--examples",
    "--frozen",
    "--message-format=json",
    "--",
    "-F",
    "clippy::unwrap_used",
    "-F",
    "clippy::expect_used",
    "-F",
    "clippy::panic",
    "-F",
    "clippy::panic_in_result_fn",
    "-F",
    "clippy::todo",
    "-F",
    "clippy::unimplemented",
    "-F",
    "clippy::indexing_slicing",
    "-F",
    "clippy::string_slice",
    "-F",
    "clippy::get_unwrap",
    "-F",
    "clippy::arithmetic_side_effects",
    "-F",
    "clippy::dbg_macro",
    "-F",
    "clippy::as_conversions",
    "-F",
    "clippy::let_underscore_must_use",
    "-F",
    "clippy::await_holding_lock",
    "-D",
    "warnings",
];
const TEST_ARGS: &[&str] = &["test", "--workspace", "--frozen", "--", "--test-threads=1"];
const BUILD_ARGS: &[&str] = &["build", "--workspace", "--release", "--frozen"];
const FMT_VERSION_ARGS: &[&str] = &["fmt", "--version"];
const CARGO_VERSION_ARGS: &[&str] = &["--version"];

/// Return the static cargo sub-command arguments for *lane*.
pub(super) const fn args_for_lane(lane: CargoLane) -> &'static [&'static str] {
    match lane {
        CargoLane::Fmt => FMT_ARGS,
        CargoLane::Compile => COMPILE_ARGS,
        CargoLane::Clippy => CLIPPY_ARGS,
        CargoLane::Test => TEST_ARGS,
        CargoLane::Build => BUILD_ARGS,
    }
}

/// Version-query args for *lane*.
pub(super) const fn version_args(lane: CargoLane) -> &'static [&'static str] {
    match lane {
        CargoLane::Fmt => FMT_VERSION_ARGS,
        CargoLane::Compile | CargoLane::Clippy | CargoLane::Test | CargoLane::Build => {
            CARGO_VERSION_ARGS
        }
    }
}

/// Map a [`CargoLane`] back to the outer [`titania_core::Lane`].
pub(super) const fn core_lane(lane: CargoLane) -> titania_core::Lane {
    match lane {
        CargoLane::Fmt => titania_core::Lane::Fmt,
        CargoLane::Compile => titania_core::Lane::Compile,
        CargoLane::Clippy => titania_core::Lane::Clippy,
        CargoLane::Test => titania_core::Lane::Test,
        CargoLane::Build => titania_core::Lane::Build,
    }
}
