use std::iter::{successors as succ};

pub fn forever_numbers() -> Vec<u8> {
    succ::<u8, _>(Some(1), |prev| Some(prev + 1)).take(2).collect()
}
