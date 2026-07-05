// Fixture: triggers ARCHITECTURE_IMPORT_CORE_TIME for Instant imports.
// Core/domain code reading a monotonic clock directly.

use std::time::Instant;

pub fn started_at() -> Instant {
    Instant::now()
}
