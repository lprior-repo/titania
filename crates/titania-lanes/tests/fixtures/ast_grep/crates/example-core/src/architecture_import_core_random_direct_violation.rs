
use rand::Rng;

pub fn choose_index<R: Rng>(rng: &mut R, len: usize) -> usize {
    rng.gen_range(0..len)
}
