use rand::RngCore;

pub fn accept_rng_core<R: RngCore>(rng: &mut R) -> u32 {
    rng.next_u32()
}
