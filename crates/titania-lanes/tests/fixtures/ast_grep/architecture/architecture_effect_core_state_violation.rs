// Fixture: triggers ARCHITECTURE_EFFECT_CORE_STATE.
// Core/domain code must not use ambient mutable state; explicit values
// travel through typed ports.

static mut CALLS: i32 = 0;

pub fn record(lock: &std::sync::Mutex<i32>) -> i32 {
    unsafe { CALLS += 1 };
    lock.lock().map_or(-1, |guard| *guard)
}
