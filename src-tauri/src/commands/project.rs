use chickenscratch_core::core::git;
use chickenscratch_core::core::project::{reader, writer};
use chickenscratch_core::scrivener::converter;
use chickenscratch_core::{ChiknError, Project};
use std::path::Path;
#[cfg(target_os = "macos")]
use std::process::Command;
use tauri::State;

use super::ProjectWriteLocks;

#[tauri::command]
pub fn create_project(
    name: String,
    path: String,
    write_locks: State<'_, ProjectWriteLocks>,
) -> Result<Project, ChiknError> {
    let project_path = Path::new(&path).join(format!("{}.chikn", name));
    write_locks.with_project_lock(&project_path, || {
        let mut project = writer::create_project(&project_path, &name)?;
        writer::write_project(&mut project)?;
        let _ = git::save_revision(&project_path, &format!("Created project: {}", name));
        Ok(project)
    })
}

#[tauri::command]
pub fn load_project(path: String) -> Result<Project, ChiknError> {
    reader::read_project(Path::new(&path))
}

#[tauri::command]
pub fn save_project(
    mut project: Project,
    write_locks: State<'_, ProjectWriteLocks>,
) -> Result<Project, ChiknError> {
    let project_path = project.path.clone();
    write_locks.with_project_lock(project_path, || {
        writer::write_project(&mut project)?;
        Ok(project)
    })
}

#[tauri::command]
pub fn import_scrivener(
    scriv_path: String,
    output_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
) -> Result<Project, ChiknError> {
    let settings = crate::commands::settings::get_app_settings();
    let pandoc = settings.general.pandoc_path.as_deref().map(Path::new);
    write_locks.with_project_lock(&output_path, || {
        converter::import_scriv(Path::new(&scriv_path), Path::new(&output_path), pandoc)
    })
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_project_metadata(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    title: Option<String>,
    author: Option<String>,
    project_type: Option<String>,
    genre: Option<String>,
    theme: Option<String>,
    summary: Option<String>,
    session_target: Option<chickenscratch_core::SessionTarget>,
) -> Result<Project, ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        let mut project = reader::read_project(Path::new(&project_path))?;
        project.metadata.title = title;
        project.metadata.author = author;
        project.metadata.project_type = project_type;
        project.metadata.genre = genre;
        project.metadata.theme = theme;
        project.metadata.summary = summary;
        project.metadata.session_target = session_target.filter(|t| !t.is_empty());
        writer::write_project(&mut project)?;
        Ok(project)
    })
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
