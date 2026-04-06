use chickenscratch_core::ChiknError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentProject {
    pub name: String,
    pub path: String,
}

fn config_dir() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("chickenscratch");
    fs::create_dir_all(&path).ok();
    path
}

fn recent_path() -> PathBuf {
    config_dir().join("recent-projects.json")
}

#[tauri::command]
pub fn get_recent_projects() -> Vec<RecentProject> {
    let path = recent_path();
    if path.exists() {
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    }
}

#[tauri::command]
pub fn add_recent_project(name: String, path: String) -> Result<(), ChiknError> {
    let mut recent = get_recent_projects();

    // Remove if already present (will re-add at top)
    recent.retain(|r| r.path != path);

    // Add at the front
    recent.insert(0, RecentProject { name, path });

    // Keep only last 10
    recent.truncate(10);

    let json = serde_json::to_string_pretty(&recent)
        .map_err(|e| ChiknError::Unknown(format!("Failed to serialize: {}", e)))?;
    fs::write(recent_path(), json)?;
    Ok(())
}

#[tauri::command]
pub fn check_pandoc() -> Result<String, ChiknError> {
    let output = std::process::Command::new("pandoc")
        .arg("--version")
        .output()
        .map_err(|_| ChiknError::Unknown("Pandoc is not installed. Install it from https://pandoc.org for import/export features.".to_string()))?;

    let version = String::from_utf8_lossy(&output.stdout);
    let first_line = version.lines().next().unwrap_or("unknown").to_string();
    Ok(first_line)
}
