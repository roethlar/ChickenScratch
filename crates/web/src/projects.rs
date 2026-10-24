//! Web-only endpoints the browser Welcome screen needs because it has no
//! native folder picker: what projects exist under the root, and where the
//! root is.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use axum::{extract::State, Json};
use serde::Serialize;

use crate::state::AppState;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProjectEntry {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct Info {
    pub root: String,
    pub version: &'static str,
    /// Whether `/api/events` is live (AI streaming).
    pub events: bool,
}

const MAX_DEPTH: usize = 3;

/// `.chikn` folders under `root`, at most three levels deep, skipping hidden
/// entries and `.git`, never descending into a project (projects do not
/// nest) and never following symlinked folders.
///
/// `name` is the folder stem: the manifest is not parsed here because reading
/// `project.yaml` outside the engine would be format I/O outside
/// `chickenscratch-core` (I2).
pub fn list_projects(root: &Path) -> Vec<ProjectEntry> {
    let mut out = Vec::new();
    walk(root, MAX_DEPTH, &mut out);
    out.sort_by_key(|p| p.name.to_lowercase());
    out
}

fn walk(dir: &Path, depth: usize, out: &mut Vec<ProjectEntry>) {
    let Ok(read) = fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = read.filter_map(Result::ok).collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        if !meta.is_dir() {
            continue;
        }
        if let Some(stem) = name.strip_suffix(".chikn") {
            if path.join("project.yaml").is_file() {
                out.push(ProjectEntry {
                    name: stem.to_string(),
                    path: path.to_string_lossy().into_owned(),
                });
            }
            continue;
        }
        if depth > 1 {
            walk(&path, depth - 1, out);
        }
    }
}

pub async fn list(State(state): State<Arc<AppState>>) -> Json<Vec<ProjectEntry>> {
    let root = state.root.clone();
    let entries = tokio::task::spawn_blocking(move || list_projects(&root))
        .await
        .unwrap_or_default();
    Json(entries)
}

pub async fn info(State(state): State<Arc<AppState>>) -> Json<Info> {
    Json(Info {
        root: state.root.to_string_lossy().into_owned(),
        version: env!("CARGO_PKG_VERSION"),
        events: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_chikn_folders_with_manifests_and_skips_the_rest() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("Notes.chikn")).unwrap();
        fs::write(root.join("Notes.chikn/project.yaml"), "name: Notes\n").unwrap();
        fs::create_dir_all(root.join("sub/Deep.chikn")).unwrap();
        fs::write(root.join("sub/Deep.chikn/project.yaml"), "name: Deep\n").unwrap();
        fs::create_dir_all(root.join("Empty.chikn")).unwrap(); // no manifest
        fs::create_dir_all(root.join(".hidden/Secret.chikn")).unwrap();
        fs::write(root.join(".hidden/Secret.chikn/project.yaml"), "x\n").unwrap();
        fs::create_dir_all(root.join("plain-folder")).unwrap();

        let names: Vec<String> = list_projects(root).into_iter().map(|p| p.name).collect();
        assert_eq!(names, vec!["Deep".to_string(), "Notes".to_string()]);
    }

    #[test]
    fn does_not_descend_past_three_levels() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("a/b/c/Far.chikn")).unwrap();
        fs::write(root.join("a/b/c/Far.chikn/project.yaml"), "x\n").unwrap();
        fs::create_dir_all(root.join("a/b/Near.chikn")).unwrap();
        fs::write(root.join("a/b/Near.chikn/project.yaml"), "x\n").unwrap();

        let names: Vec<String> = list_projects(root).into_iter().map(|p| p.name).collect();
        assert_eq!(names, vec!["Near".to_string()]);
    }
}
