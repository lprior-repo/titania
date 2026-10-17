use rand::{Rng, thread_rng};

pub fn choose_index(len: usize) -> usize {
    let mut rng = thread_rng();
    rng.gen_range(0..len)
}
