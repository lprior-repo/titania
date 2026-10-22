
pub trait Tr {
    fn f(&self) -> u32;
}

pub struct X {
    depth: u32,
}

impl Tr for X {
    fn f(&self) -> u32 {
        if self.depth == 0 {
            0
        } else {
            <Self as Tr>::f(&X { depth: self.depth.saturating_sub(1) })
        }
    }
}
