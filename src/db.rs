use std::str::FromStr;

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

use crate::error::{Error, Result};

pub async fn connect(database_url: &str) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(database_url)
        .map_err(|e| Error::Config(format!("invalid database url: {e}")))?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true);

    let max_connections = if database_url.contains(":memory:") {
        1
    } else {
        8
    };

    let pool = SqlitePoolOptions::new()
        .max_connections(max_connections)
        .connect_with(options)
        .await?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| Error::Internal(anyhow::anyhow!("migration failed: {e}")))?;

    // Cells are subprocesses of this API. A restart cannot recover them.
    sqlx::query(
        "UPDATE sandboxes SET state = 'stopped', sock = NULL, pid = NULL WHERE state = 'running'",
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}
