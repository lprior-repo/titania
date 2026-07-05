// Fixture: violation — real while-loop after a string containing /*.

pub fn clear_ready(mut ready: bool) -> bool {
    let marker = "/*"; while ready { let _ = marker; ready = false; }
    ready
}
