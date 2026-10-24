//! Thin Tauri wrappers over `chickenscratch_services::project` — transport only.
use chickenscratch_services::project as svc;
#[allow(unused_imports)]
use chickenscratch_services::project::*;
use chickenscratch_services::ChiknError;
use chickenscratch_services::Project;
use chickenscratch_services::{ProjectTokens, ProjectWriteLocks};
#[cfg(target_os = "macos")]
use std::process::Command;
use tauri::State;

#[tauri::command]
pub fn create_project(
    name: String,
    path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
) -> Result<Project, ChiknError> {
    svc::create_project(name, path, &write_locks, &tokens)
}

#[tauri::command]
pub fn load_project(
    path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
) -> Result<LoadedProject, ChiknError> {
    svc::load_project(path, &write_locks, &tokens)
}

#[tauri::command]
pub fn save_project(
    project: Project,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
) -> Result<Project, ChiknError> {
    svc::save_project(project, &write_locks, &tokens)
}

#[tauri::command]
pub fn import_scrivener(
    scriv_path: String,
    output_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
) -> Result<ImportedProject, ChiknError> {
    svc::import_scrivener(scriv_path, output_path, &write_locks, &tokens)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_project_metadata(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    title: Option<String>,
    author: Option<String>,
    project_type: Option<String>,
    genre: Option<String>,
    theme: Option<String>,
    summary: Option<String>,
    session_target: Option<chickenscratch_services::SessionTarget>,
) -> Result<Project, ChiknError> {
    svc::update_project_metadata(
        project_path,
        &write_locks,
        &tokens,
        title,
        author,
        project_type,
        genre,
        theme,
        summary,
        session_target,
    )
}

#[tauri::command]
pub fn pick_scriv_folder() -> Result<Option<String>, ChiknError> {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("osascript")
            .arg("-e")
            .arg("POSIX path of (choose file of type {\"com.literatureandlatte.scrivener3.scriv\", \"com.literatureandlatte.scrivener2.scriv\"} with prompt \"Select Scrivener Project\")")
            .output()
            .map_err(|e| ChiknError::Unknown(format!("Failed to open file dialog: {}", e)))?;

        if !output.status.success() {
            return Ok(None);
        }

        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if path.is_empty() {
            Ok(None)
        } else {
            Ok(Some(path.trim_end_matches(['/', '\\']).to_string()))
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        use rfd::FileDialog;
        let folder = FileDialog::new()
            .set_title("Select Scrivener Project")
            .pick_folder();
        Ok(folder.map(|p| p.to_string_lossy().to_string()))
    }
}
