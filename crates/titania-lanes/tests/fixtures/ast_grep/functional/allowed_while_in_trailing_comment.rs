

pub fn poll_state(state: &mut State) {
    let counter = 0;
    let running = true;
    let result = do_work();
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
