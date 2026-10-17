
use tokio::task;

pub async fn spawn_work() {
    let _handle = task::spawn(async {});
}
