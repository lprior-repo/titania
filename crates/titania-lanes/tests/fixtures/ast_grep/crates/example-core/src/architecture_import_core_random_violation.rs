// Fixture: triggers ARCHITECTURE_IMPORT_CORE_RANDOM
// Core/domain code using entropy sources directly.

use rand::thread_rng;

pub fn pick_item(items: &[&str]) -> &str {
    let mut rng = thread_rng();
    let idx = rng.gen_range(0..items.len());
    items[idx]
}
