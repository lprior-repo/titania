pub(crate) use rand::Rng;

pub fn expose_rng<T>(rng: T) -> T {
    rng
}
