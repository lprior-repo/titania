// Fixture: triggers BOUNDED_EXECUTION_UNBOUNDED_LOOP
// Unbounded retry without a named bound: a `loop` block and a condition
// `while` loop. Neither cites Bound::FiniteInput, Bound::Capped, nor
// Bound::ProtocolTerminal, so the required bound obligation blocks shipping.

pub fn retry_until_ready(flag: &std::sync::atomic::AtomicBool) -> u32 {
    let mut attempts = 0;
    loop {
        if flag.load(std::sync::atomic::Ordering::SeqCst) {
            break attempts;
        }
        attempts += 1;
    }
}

pub fn spin_while_busy(total: &mut i32) {
    while *total < 100 {
        *total += 1;
    }
}
