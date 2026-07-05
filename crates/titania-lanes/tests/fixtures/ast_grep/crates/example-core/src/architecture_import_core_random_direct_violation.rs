// Fixture: triggers ARCHITECTURE_IMPORT_CORE_RANDOM for rand trait imports.
// Core/domain code depending directly on the rand crate.

use rand::Rng;

pub fn choose_index<R: Rng>(rng: &mut R, len: usize) -> usize {
    rng.gen_range(0..len)
}
