// Fixture: triggers ARCHITECTURE_IMPORT_CORE_RANDOM for grouped rand imports.
// Core/domain code depending directly on entropy APIs.
use rand::{Rng, thread_rng};

pub fn choose_index(len: usize) -> usize {
    let mut rng = thread_rng();
    rng.gen_range(0..len)
}
