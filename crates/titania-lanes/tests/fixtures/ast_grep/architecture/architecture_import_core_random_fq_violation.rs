// Fixture: triggers ARCHITECTURE_IMPORT_CORE_RANDOM for fully-qualified paths.
// Core/domain code drawing entropy with no `use` import.

pub fn pick(items: &[&str]) -> &str {
    let mut rng = rand::thread_rng();
    let index = rand::Rng::gen_range(&mut rng, 0..items.len());
    items[index]
}
