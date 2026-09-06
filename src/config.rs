use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub cors_origins: Vec<String>,
    pub public_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            database_url: "sqlite::memory:".to_string(),
            cors_origins: vec![
                "http://localhost:5173".to_string(),
                "http://127.0.0.1:5173".to_string(),
                "https://cloudcell.dev".to_string(),
                "https://www.cloudcell.dev".to_string(),
            ],
            public_url: "http://127.0.0.1:8080".to_string(),
        }
    }
}

impl Config {
    pub fn from_env() -> Self {
        let mut cfg = Self {
            database_url: "sqlite:cloudcell.db".to_string(),
            ..Self::default()
        };
        if let Ok(host) = std::env::var("CLOUDCELL_HOST") {
            cfg.host = host;
        }
        if let Ok(port) = std::env::var("CLOUDCELL_PORT")
            && let Ok(parsed) = port.parse()
        {
            cfg.port = parsed;
        }
        if let Ok(database_url) = std::env::var("CLOUDCELL_DATABASE_URL") {
            cfg.database_url = database_url;
        }
        if let Ok(origins) = std::env::var("CLOUDCELL_CORS_ORIGINS") {
            cfg.cors_origins = origins
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
        if let Ok(public_url) = std::env::var("CLOUDCELL_PUBLIC_URL") {
            cfg.public_url = public_url;
        }
        cfg
    }
}
