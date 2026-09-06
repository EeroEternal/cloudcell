use sqlx::SqlitePool;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db: SqlitePool,
}

impl AppState {
    pub fn new(config: Config, db: SqlitePool) -> Self {
        Self { config, db }
    }
}
