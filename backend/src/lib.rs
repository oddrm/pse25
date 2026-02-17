pub mod error;
pub mod plugin_manager;
pub mod routes;
pub mod schema;
pub mod storage;

use storage::storage_manager::StorageManager;

use crate::plugin_manager::manager::PluginManager;
use std::sync::Arc;

pub struct AppState {
    pub storage_manager: StorageManager,
    pub plugin_manager: Arc<tokio::sync::Mutex<PluginManager>>,
}
