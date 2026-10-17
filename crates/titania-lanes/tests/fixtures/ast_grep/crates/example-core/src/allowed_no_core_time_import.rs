
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
