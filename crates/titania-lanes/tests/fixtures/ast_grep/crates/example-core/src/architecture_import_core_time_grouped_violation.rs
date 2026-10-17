use std::time::{Duration, Instant, SystemTime};

pub fn started_at() -> (SystemTime, Instant, Duration) {
    (SystemTime::now(), Instant::now(), Duration::from_secs(1))
}
