// Fixture: allowed — while and loop keywords only in trailing comments.
//
// Trailing comments are code lines before `//`, but text after `//` must not
// trigger findings.

pub fn poll_state(state: &mut State) {
    let counter = 0; // while counter < max { retry }
    let running = true; // while running { tick() }
    let result = do_work(); // loop { try_again(result) }
    state.apply(counter, running, result);
}

pub struct State {
    pub value: i32,
}

impl State {
    pub fn apply(&mut self, _c: i32, _r: bool, _res: ()) {
        self.value += 1;
    }
}
