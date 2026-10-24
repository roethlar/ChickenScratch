//! `GET /api/fs/dirs?path=…`: the folder browser behind the web shell's
//! folder picker (backup folder, Scrivener source). Lists sub-folders of one
//! folder inside the projects root; hidden entries and symlinks are skipped
//! and nothing outside the root is ever listed.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::commands::CmdError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct DirsQuery {
    /// Absolute folder inside the root; empty or missing means the root.
    #[serde(default)]
    path: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct DirEntry {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct DirListing {
    pub path: String,
    /// Parent folder, or null at the root.
    pub parent: Option<String>,
    pub dirs: Vec<DirEntry>,
}

pub fn list_dirs(dir: &Path) -> Vec<DirEntry> {
    let Ok(read) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<DirEntry> = read
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                return None;
            }
            let path = entry.path();
            let meta = fs::symlink_metadata(&path).ok()?;
            if !meta.is_dir() {
                return None;
            }
            Some(DirEntry {
                name,
                path: path.to_string_lossy().into_owned(),
            })
        })
        .collect();
    out.sort_by_key(|d| d.name.to_lowercase());
    out
}

pub async fn dirs(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DirsQuery>,
) -> Result<Json<DirListing>, CmdError> {
    let dir = if query.path.trim().is_empty() {
        state.root.clone()
    } else {
        state.gate.existing(&query.path)?
    };
    if !dir.is_dir() {
        return Err(CmdError::BadRequest("Not a folder".to_string()));
    }
    let parent = if dir == state.root {
        None
    } else {
        dir.parent()
            .filter(|p| p.starts_with(&state.root))
            .map(|p| p.to_string_lossy().into_owned())
    };
    let listing_dir = dir.clone();
    let entries = tokio::task::spawn_blocking(move || list_dirs(&listing_dir))
        .await
        .map_err(|_| CmdError::Internal)?;
    Ok(Json(DirListing {
        path: dir.to_string_lossy().into_owned(),
        parent,
        dirs: entries,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_plain_subfolders_only() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("b")).unwrap();
        fs::create_dir_all(root.join("A")).unwrap();
        fs::create_dir_all(root.join(".hidden")).unwrap();
        fs::write(root.join("file.txt"), "x").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(root.join("A"), root.join("link")).unwrap();

        let names: Vec<String> = list_dirs(root).into_iter().map(|d| d.name).collect();
        assert_eq!(names, vec!["A".to_string(), "b".to_string()]);
    }
}
