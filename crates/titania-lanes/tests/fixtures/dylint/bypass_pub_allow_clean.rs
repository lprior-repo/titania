//! Fixture: repaired source with no `BYPASS_PUB_ALLOW` violation.
//!
//! No `#[allow(...)]` on any `pub` item. Used by `dylint_reject_repair.rs`
//! to prove the lane reports Clean after repair.

pub fn clean_public_function() -> u32 {
    42
}
