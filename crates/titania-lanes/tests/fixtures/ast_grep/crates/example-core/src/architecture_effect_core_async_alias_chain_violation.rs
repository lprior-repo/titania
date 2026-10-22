use tokio as tk;
use tk as t2;

pub fn go() {
    t2::spawn(async { 1 });
}
