pub mod error;
pub mod plugin_manager;
pub mod routes;
pub mod schema;
pub mod storage;

use storage::storage_manager::StorageManager;

pub struct AppState {
    pub storage_manager: StorageManager,
}
