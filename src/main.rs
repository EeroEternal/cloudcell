use cloudcell::db;
use cloudcell::{config::Config, error::Result, server, state::AppState};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env();
    let addr = format!("{}:{}", config.host, config.port);
    info!(
        %addr,
        public_url = %config.public_url,
        database_url = %config.database_url,
        sand = ?config.sand_bin,
        "starting cloudcell api"
    );

    let db = db::connect(&config.database_url).await?;
    let app = server::create_router(AppState::new(config, db));
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind to {addr}: {e}"))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {e}"))?;

    Ok(())
}
