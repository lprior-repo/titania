// Fixture: allowed — no entropy sources in core/domain.
// Core code receives randomness through a trait abstraction.

use std::ops::Range;

pub trait RandomSource {
    fn gen_range(&mut self, range: Range<usize>) -> usize;
}

pub struct DeterministicRng {
    counter: usize,
}

impl DeterministicRng {
    pub fn new(counter: usize) -> Self {
        Self { counter }
    }
}

impl RandomSource for DeterministicRng {
    fn gen_range(&mut self, range: Range<usize>) -> usize {
        let idx = self.counter % (range.end - range.start) + range.start;
        self.counter += 1;
        idx
    }
}

pub fn pick_item(items: &[&str], rng: &mut dyn RandomSource) -> &str {
    let idx = rng.gen_range(0..items.len());
    items[idx]
}
