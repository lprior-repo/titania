pub mod gen {
    pub fn repeat<T>(value: T) -> Vec<T> {
        vec![value]
    }
}

pub mod iter {
    pub fn repeat<T>(value: T) -> Vec<T> {
        vec![value]
    }
}

pub fn use_generic_repeat() -> Vec<u8> {
    gen::repeat::<u8>(7)
}

pub fn use_local_iter_repeat() -> Vec<u8> {
    iter::repeat::<u8>(9)
}
