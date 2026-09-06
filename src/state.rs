use sqlx::SqlitePool;

use crate::cell::CellRegistry;
use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db: SqlitePool,
    pub cells: CellRegistry,
}

impl AppState {
    pub fn new(config: Config, db: SqlitePool) -> Self {
        Self {
            config,
            db,
            cells: CellRegistry::new(),
        }
    }
}
