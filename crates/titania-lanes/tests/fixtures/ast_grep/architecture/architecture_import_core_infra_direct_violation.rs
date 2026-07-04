// Fixture: triggers ARCHITECTURE_IMPORT_CORE_INFRA for direct imports.
// Core/domain code importing a concrete infrastructure module directly.

use tokio::task;

pub async fn spawn_work() {
    let _handle = task::spawn(async {});
}
