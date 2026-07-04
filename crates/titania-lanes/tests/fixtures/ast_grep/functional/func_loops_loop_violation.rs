// Fixture: triggers FUNC_LOOPS_LOOP
// Infinite loop block in production source — should use a bounded iterator or async.

pub fn wait_for_signal(signaled: &std::sync::Arc<std::sync::atomic::AtomicBool>) {
    let mut retries = 0;
    loop {
        if signaled.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
        retries += 1;
        if retries > 1000 {
            break;
        }
    }
}

pub fn process_queue(queue: &mut Vec<String>) {
    loop {
        let item = queue.pop();
        match item {
            Some(val) => println!("Processing: {val}"),
            None => break,
        }
    }
}
