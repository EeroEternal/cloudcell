use axum::Json;
use serde::Serialize;

use crate::error::Result;

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub id: String,
    pub name: String,
    pub language: String,
    /// Catalog status. `declared` means the id is reserved; packing erofs onto
    /// a cell node is not implemented on this control plane yet.
    pub status: &'static str,
}

pub fn catalog() -> Vec<Snapshot> {
    vec![
        Snapshot {
            id: "base".into(),
            name: "base".into(),
            language: "shell".into(),
            status: "declared",
        },
        Snapshot {
            id: "python-3.12".into(),
            name: "python-3.12".into(),
            language: "python".into(),
            status: "declared",
        },
        Snapshot {
            id: "node-22".into(),
            name: "node-22".into(),
            language: "javascript".into(),
            status: "declared",
        },
    ]
}

pub fn exists(id: &str) -> bool {
    catalog().iter().any(|s| s.id == id)
}

pub async fn list_snapshots() -> Result<Json<Vec<Snapshot>>> {
    Ok(Json(catalog()))
}
