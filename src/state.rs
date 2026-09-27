use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::OnceCell;

use crate::cell::CellRegistry;
use crate::config::Config;
use crate::mail::Mailer;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db: SqlitePool,
    pub cells: CellRegistry,
    pub mailer: Mailer,
    /// Lazily probes `sand --capabilities` once per process.
    pub caps: Arc<OnceCell<std::result::Result<(), String>>>,
}

impl AppState {
    pub fn new(config: Config, db: SqlitePool) -> Self {
        Self {
            config,
            db,
            cells: CellRegistry::new(),
            mailer: Mailer::log(),
            caps: Arc::new(OnceCell::new()),
        }
    }

    pub fn with_mailer(mut self, mailer: Mailer) -> Self {
        self.mailer = mailer;
        self
    }
}
