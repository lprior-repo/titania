pub(crate) use rand::thread_rng;

pub fn pick() -> u8 {
    thread_rng().gen_range(0..10)
}
