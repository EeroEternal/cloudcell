use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::api_key::ApiKeyRecord;
use crate::config::Config;
use crate::sandbox::Sandbox;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub sandboxes: Arc<Mutex<HashMap<String, Sandbox>>>,
    pub keys: Arc<Mutex<HashMap<String, ApiKeyRecord>>>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            sandboxes: Arc::new(Mutex::new(HashMap::new())),
            keys: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}
