pub fn broken_value() -> u32 {
    "not a number"
}

pub fn spin_forever() {
    loop {
        std::hint::spin_loop();
    }
}
