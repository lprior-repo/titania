// H3: loop-keyword column accuracy — `loop` at 1-based line 4, 0-based col 4.
pub fn retry(flag: &std::sync::atomic::AtomicBool) -> u32 {
    let mut attempts = 0;
    loop {
        if flag.load(std::sync::atomic::Ordering::SeqCst) {
            break attempts;
        }
        attempts += 1;
    }
}
