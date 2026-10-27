//! Thin Tauri wrappers over `chickenscratch_services::project` — transport only.
use chickenscratch_services::project as svc;
#[allow(unused_imports)]
use chickenscratch_services::project::*;
use chickenscratch_services::ChiknError;
use chickenscratch_services::Project;
use chickenscratch_services::{ProjectTokens, ProjectWriteLocks};
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use std::process::Command;
use tauri::State;

use crate::assets::OpenProject;

#[tauri::command]
pub fn create_project(
    name: String,
    path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    open: State<'_, OpenProject>,
) -> Result<Project, ChiknError> {
    let project = svc::create_project(name, path, &write_locks, &tokens)?;
    // The `chikn-asset` scheme serves pictures from the open project only.
    open.set(Some(PathBuf::from(&project.path)));
    Ok(project)
}

#[tauri::command]
pub fn load_project(
    path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    open: State<'_, OpenProject>,
) -> Result<LoadedProject, ChiknError> {
    let loaded = svc::load_project(path, &write_locks, &tokens)?;
    open.set(Some(PathBuf::from(&loaded.project.path)));
    Ok(loaded)
}

/// The UI closed its project: from now on the `chikn-asset` scheme refuses
/// every request until another project is loaded (PLAN_SCRIVENER_UPLOAD
/// slice 3, plan-4 R5-6). The web shell accepts the same command as a no-op.
#[tauri::command]
pub fn close_project(open: State<'_, OpenProject>) -> Result<(), ChiknError> {
    open.set(None);
    Ok(())
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
    open: State<'_, OpenProject>,
) -> Result<ImportedProject, ChiknError> {
    import_and_open(&open, || {
        svc::import_scrivener(scriv_path, output_path, &write_locks, &tokens)
    })
}

/// Run an import and, when it succeeds, make its project the open one. The
/// UI opens the imported project straight from this result, without a
/// `load_project` call, so the `chikn-asset` scheme must learn of it here
/// (review finding s3-6). A failed import leaves whatever was open alone.
fn import_and_open(
    open: &OpenProject,
    import: impl FnOnce() -> Result<ImportedProject, ChiknError>,
) -> Result<ImportedProject, ChiknError> {
    let imported = import()?;
    open.set(Some(PathBuf::from(&imported.project.path)));
    Ok(imported)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn imported(path: &str) -> ImportedProject {
        let project: Project = serde_json::from_value(serde_json::json!({
            "id": "p1",
            "name": "Novel",
            "path": path,
            "hierarchy": [],
            "documents": {},
            "created": "",
            "modified": ""
        }))
        .expect("a minimal project deserializes");
        ImportedProject {
            project,
            warnings: Vec::new(),
        }
    }

    /// A successful import becomes the open project for the asset scheme; a
    /// failed one leaves the previous project open (review finding s3-6).
    #[test]
    fn a_successful_import_opens_its_project_for_the_asset_scheme() {
        let open = OpenProject::default();
        open.set(Some(PathBuf::from("/before/Old.chikn")));

        let failed = import_and_open(&open, || Err(ChiknError::Unknown("no pandoc".to_string())));
        assert!(failed.is_err());
        assert_eq!(open.current(), Some(PathBuf::from("/before/Old.chikn")));

        let result = import_and_open(&open, || Ok(imported("/after/Novel.chikn")));
        assert_eq!(result.unwrap().project.path, "/after/Novel.chikn");
        assert_eq!(open.current(), Some(PathBuf::from("/after/Novel.chikn")));
    }
}
