// Fixture: triggers ARCHITECTURE_IMPORT_CORE_FS for module imports.
// Core/domain code importing filesystem module directly.
use std::fs;

pub fn load_config() -> std::io::Result<String> {
    fs::read_to_string("config.toml")
}
