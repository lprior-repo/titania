//! Exact-path waivers for non-production nightly-feature boundaries.
//!
//! These exceptions are intentionally narrow: Dylint is a rustc-private
//! compiler-plugin crate, while ordinary production crates remain covered by
//! `NIGHTLY_FEATURE_001`.

const DYLINT_PLUGIN_FEATURE: &str = "rustc_private";
const DYLINT_FIXTURE_FEATURES: &[&str] = &["allow_internal_unstable", "allow_internal_unsafe"];

pub(super) fn is_dylint_boundary_feature(file: &str, name: &str) -> bool {
    let normalized = file.replace('\\', "/");
    let plugin =
        name == DYLINT_PLUGIN_FEATURE && normalized.ends_with("crates/titania-dylint/src/lib.rs");
    let fixture = DYLINT_FIXTURE_FEATURES.contains(&name)
        && normalized.ends_with("crates/titania-dylint/tests/internal_escape.rs");
    plugin || fixture
}
