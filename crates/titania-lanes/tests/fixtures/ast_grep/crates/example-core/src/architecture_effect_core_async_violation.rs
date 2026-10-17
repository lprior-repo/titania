// Fixture: triggers ARCHITECTURE_EFFECT_CORE_ASYNC.
// Core/domain code must stay synchronous; async execution belongs to the
// shell behind a typed port.

pub async fn fetch_data() -> i32 {
    let pending = async { 1 };
    pending.await
}

pub fn spawn_background() {
    tokio::spawn(async { 1 });
}
