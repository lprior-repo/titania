
pub async fn fetch_data() -> i32 {
    let pending = async { 1 };
    pending.await
}

pub fn spawn_background() {
    tokio::spawn(async { 1 });
}
