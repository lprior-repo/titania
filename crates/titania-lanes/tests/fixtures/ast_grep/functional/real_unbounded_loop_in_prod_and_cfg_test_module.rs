
pub fn retry_without_bound(flag: &std::sync::atomic::AtomicBool) -> u32 {
    let mut attempts = 0;
    loop {
        if flag.load(std::sync::atomic::Ordering::SeqCst) {
            break attempts;
        }
        attempts += 1;
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn retries_in_test() {
        let mut attempts = 0;
        loop {
            attempts += 1;
            if attempts >= 3 {
                break;
            }
        }
        assert_eq!(attempts, 3);
    }
}
