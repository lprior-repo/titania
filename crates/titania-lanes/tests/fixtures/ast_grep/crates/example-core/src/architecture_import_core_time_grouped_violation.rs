// Fixture: triggers ARCHITECTURE_IMPORT_CORE_TIME for grouped time imports.
// Core/domain code reading wall-clock or monotonic time directly.
use std::time::{Duration, Instant, SystemTime};

pub fn started_at() -> (SystemTime, Instant, Duration) {
    (SystemTime::now(), Instant::now(), Duration::from_secs(1))
}
