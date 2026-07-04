// Fixture: triggers ARCHITECTURE_IMPORT_CORE_INFRA
// Core/domain code importing infrastructure crates directly.

use tokio::*;

pub async fn fetch_data() {
    // Infrastructure concern leaking into core layer.
}
