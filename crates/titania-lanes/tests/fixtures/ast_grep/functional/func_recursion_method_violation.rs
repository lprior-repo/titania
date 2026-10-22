
pub struct Walker {
    depth: u32,
}

impl Walker {
    pub fn walk(&self, n: u32) -> u32 {
        if n == 0 {
            self.depth
        } else {
            self.walk(n.saturating_sub(1))
        }
    }

    pub fn descend(n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            Self::descend(n.saturating_sub(1))
        }
    }
}
