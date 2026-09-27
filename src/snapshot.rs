use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::state::AppState;

/// Built-in snapshot ids the control plane always offers. Imported OCI
/// snapshots (see `deploy/gcp/import-oci.sh`) add more names dynamically.
pub const IDS: [&str; 5] = ["base", "python-3.12", "node-22", "rust", "go"];

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub id: String,
    pub name: String,
    pub language: String,
    /// Always `declared` today; readiness is reported by `packed`.
    pub status: &'static str,
    /// True when this node can actually start a cell from this snapshot.
    pub packed: bool,
    /// Content address of the imported OCI image, when there is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    /// Source image ref recorded at import time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IndexEntry {
    /// Canonical OCI manifest digest, e.g. `sha256:abc…`.
    pub digest: String,
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub language: String,
}

#[derive(Debug, Default, Deserialize)]
struct Index {
    #[serde(default)]
    snapshots: BTreeMap<String, IndexEntry>,
}

/// `<rootfs_dir>/index.json`: the name -> content-digest map written by
/// the OCI importer.  Missing or malformed = no imported snapshots.
pub fn load_index(rootfs_dir: &Path) -> BTreeMap<String, IndexEntry> {
    let raw = match std::fs::read_to_string(rootfs_dir.join("index.json")) {
        Ok(raw) => raw,
        Err(_) => return BTreeMap::new(),
    };
    serde_json::from_str::<Index>(&raw)
        .map(|idx| idx.snapshots)
        .unwrap_or_default()
}

/// `sha256:abc` -> `blobs/sha256-abc` (`:` is awkward in shell paths).
pub fn digest_dir(rootfs_dir: &Path, digest: &str) -> PathBuf {
    rootfs_dir.join("blobs").join(digest.replace(':', "-"))
}

/// Unpacked rootfs for `id`: the imported content-addressed blob when the
/// index has one, else the legacy `rootfs_dir/<id>` tree from pack-rootfs.sh.
pub fn resolve(rootfs_dir: &Path, id: &str) -> Option<PathBuf> {
    if let Some(entry) = load_index(rootfs_dir).get(id) {
        let p = digest_dir(rootfs_dir, &entry.digest);
        if p.is_dir() {
            return Some(p);
        }
    }
    let legacy = rootfs_dir.join(id);
    legacy.is_dir().then_some(legacy)
}

/// True when this node has the snapshot unpacked and a cell can use it.
pub fn packed(rootfs_dir: &Path, id: &str) -> bool {
    resolve(rootfs_dir, id).is_some()
}

/// A declared name is valid if it is built in or imported on this node.
pub fn exists(rootfs_dir: &Path, id: &str) -> bool {
    IDS.contains(&id) || load_index(rootfs_dir).contains_key(id)
}

pub fn catalog(rootfs_dir: &Path) -> Vec<Snapshot> {
    let index = load_index(rootfs_dir);
    let builtin = [
        ("base", "shell"),
        ("python-3.12", "python"),
        ("node-22", "javascript"),
        ("rust", "rust"),
        ("go", "go"),
    ];
    let mut out: Vec<Snapshot> = builtin
        .iter()
        .map(|(id, language)| {
            let entry = index.get(*id);
            Snapshot {
                id: (*id).to_string(),
                name: (*id).to_string(),
                language: entry
                    .map(|e| e.language.clone())
                    .filter(|l| !l.is_empty())
                    .unwrap_or_else(|| (*language).to_string()),
                status: "declared",
                packed: packed(rootfs_dir, id),
                digest: entry.map(|e| e.digest.clone()),
                image: entry.map(|e| e.image.clone()).filter(|i| !i.is_empty()),
            }
        })
        .collect();
    for (name, entry) in &index {
        if IDS.contains(&name.as_str()) {
            continue;
        }
        out.push(Snapshot {
            id: name.clone(),
            name: name.clone(),
            language: if entry.language.is_empty() {
                "unknown".into()
            } else {
                entry.language.clone()
            },
            status: "declared",
            packed: packed(rootfs_dir, name),
            digest: Some(entry.digest.clone()),
            image: Some(entry.image.clone()).filter(|i| !i.is_empty()),
        });
    }
    out
}

pub async fn list_snapshots(State(state): State<AppState>) -> Result<Json<Vec<Snapshot>>> {
    Ok(Json(catalog(&state.config.rootfs_dir)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "cloudcell-snap-{tag}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn resolves_imported_blob_by_name() {
        let dir = tmpdir("import");
        std::fs::create_dir_all(dir.join("blobs/sha256-deadbeef")).unwrap();
        std::fs::write(
            dir.join("index.json"),
            r#"{"version":1,"snapshots":{"go":{"digest":"sha256:deadbeef","image":"docker://golang:1.23","language":"go"}}}"#,
        )
        .unwrap();

        assert_eq!(resolve(&dir, "go"), Some(dir.join("blobs/sha256-deadbeef")));
        assert!(packed(&dir, "go"));
        assert!(exists(&dir, "go"));
        assert!(!exists(&dir, "not-imported"));

        let cat = catalog(&dir);
        let go = cat.iter().find(|s| s.id == "go").unwrap();
        assert_eq!(go.digest.as_deref(), Some("sha256:deadbeef"));
        assert_eq!(go.image.as_deref(), Some("docker://golang:1.23"));
        assert!(go.packed);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unresolved_digest_is_not_packed() {
        let dir = tmpdir("missing");
        std::fs::write(
            dir.join("index.json"),
            r#"{"snapshots":{"go":{"digest":"sha256:gone"}}}"#,
        )
        .unwrap();
        assert!(!packed(&dir, "go"));
        assert!(exists(&dir, "go"));
        let go = catalog(&dir).into_iter().find(|s| s.id == "go").unwrap();
        assert!(!go.packed);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn legacy_tree_still_resolves() {
        let dir = tmpdir("legacy");
        std::fs::create_dir_all(dir.join("rust")).unwrap();
        assert_eq!(resolve(&dir, "rust"), Some(dir.join("rust")));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn malformed_index_is_ignored() {
        let dir = tmpdir("bad");
        std::fs::write(dir.join("index.json"), r#"{not json"#).unwrap();
        assert!(load_index(&dir).is_empty());
        assert!(!packed(&dir, "go"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
