
pub mod sub {
    pub type Result<T, E> = core::result::Result<T, E>;
}

pub fn load(input: sub::Result<u8, String>) -> sub::Result<u8, String> {
    input
}
