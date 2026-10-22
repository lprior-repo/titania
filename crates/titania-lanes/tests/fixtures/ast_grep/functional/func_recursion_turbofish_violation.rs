
pub struct Walker {
    depth: u32,
}

impl Walker {
    pub fn walk<T>(&self, n: u32) -> u32 {
        if n == 0 {
            self.depth
        } else {
            self.walk::<u8>(n.saturating_sub(1))
        }
    }

    pub fn descend<T>(n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            Self::descend::<u8>(n.saturating_sub(1))
        }
    }
}

pub fn spin<T>(n: u32) -> u32 {
    if n == 0 {
        0
    } else {
        spin::<u8>(n.saturating_sub(1))
    }
}
