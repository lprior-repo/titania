pub struct Dial {
    level: u32,
}

impl Dial {
    pub fn repeat(&self, times: u32) -> u32 {
        self.level * times
    }
}

pub mod iter {
    pub fn repeat(value: u32) -> u32 {
        value
    }
}

pub fn use_own_repeat_names(dial: &Dial) -> u32 {
    let via_method = dial.repeat(3);
    let via_module = iter::repeat(1);
    via_method + via_module
}
