//! Fixture: deliberate `BYPASS_PUB_ALLOW` violation.
//!
//! A `#[allow(...)]` on a `pub` item must trigger the typed dylint lint.
//! Used by `dylint_reject_repair.rs` to prove the type-aware lints fire.

#[allow(dead_code)]
pub fn smuggled_allow() -> u32 {
    42
}
