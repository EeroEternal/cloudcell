use sqlx::SqlitePool;

use crate::cell::CellRegistry;
use crate::config::Config;
use crate::mail::Mailer;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db: SqlitePool,
    pub cells: CellRegistry,
    pub mailer: Mailer,
}

impl AppState {
    pub fn new(config: Config, db: SqlitePool) -> Self {
        Self {
            config,
            db,
            cells: CellRegistry::new(),
            mailer: Mailer::log(),
        }
    }

    pub fn with_mailer(mut self, mailer: Mailer) -> Self {
        self.mailer = mailer;
        self
    }
}
