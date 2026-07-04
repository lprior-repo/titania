// Fixture: allowed — no wall-clock imports in core/domain.
// Core code receives time as a parameter or uses an abstracted clock.

pub trait Clock {
    fn now(&self) -> u64;
}

pub struct TimestampedValue {
    pub value: String,
    pub timestamp: u64,
}

impl TimestampedValue {
    pub fn new(value: String, clock: &dyn Clock) -> Self {
        Self {
            value,
            timestamp: clock.now(),
        }
    }
}
