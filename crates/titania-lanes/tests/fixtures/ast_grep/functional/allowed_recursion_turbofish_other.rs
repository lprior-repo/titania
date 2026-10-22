
pub struct Pair {
    value: u32,
}

impl Pair {
    pub fn start<T>(&self, n: u32) -> u32 {
        self.finish::<u8>(n)
    }

    pub fn finish<T>(&self, n: u32) -> u32 {
        n.saturating_add(self.value)
    }

    pub fn make<T>() -> u32 {
        Self::produce::<u8>()
    }

    pub fn produce<T>() -> u32 {
        7
    }
}
