use tokio::{task::spawn};

pub fn go() {
    spawn(async { 1 });
}
