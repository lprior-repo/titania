
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
