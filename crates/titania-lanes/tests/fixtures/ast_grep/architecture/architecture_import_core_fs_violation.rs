// Fixture: triggers ARCHITECTURE_IMPORT_CORE_FS
// Core/domain code performing direct filesystem I/O.

use std::fs::*;

pub fn read_config() -> String {
    let content = read_to_string("config.toml").unwrap();
    content
}
