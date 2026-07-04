// Fixture: triggers FUNC_LOOPS_WHILE
// While-loop in production source — should use a condition-based iterator.

pub fn countdown(n: u64) -> Vec<u64> {
    let mut result = Vec::new();
    let mut i = n;
    while i > 0 {
        result.push(i);
        i -= 1;
    }
    result
}

pub fn poll_until_ready(flag: &std::sync::Arc<std::sync::atomic::AtomicBool>) {
    let mut attempts = 0;
    while !flag.load(std::sync::atomic::Ordering::Relaxed) && attempts < 100 {
        std::thread::yield_now();
        attempts += 1;
    }
}
