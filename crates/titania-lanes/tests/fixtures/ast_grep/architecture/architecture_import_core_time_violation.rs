// Fixture: triggers ARCHITECTURE_IMPORT_CORE_TIME
// Core/domain code reading the wall clock directly.

use std::time::SystemTime;

pub fn now() -> std::time::Duration {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
}
