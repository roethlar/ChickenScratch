use chickenscratch_core::core::project::{reader, writer};
use chickenscratch_core::scrivener::converter;
use chickenscratch_core::{ChiknError, Project};
use std::path::Path;
use std::process::Command;

#[tauri::command]
pub fn create_project(name: String, path: String) -> Result<Project, ChiknError> {
    let project_path = Path::new(&path).join(format!("{}.chikn", name));
    let mut project = writer::create_project(&project_path, &name)?;
    writer::write_project(&mut project)?;
    converter::git_commit(&project_path, &format!("Created project: {}", name));
    Ok(project)
}

#[tauri::command]
pub fn load_project(path: String) -> Result<Project, ChiknError> {
    reader::read_project(Path::new(&path))
}

#[tauri::command]
pub fn save_project(mut project: Project) -> Result<Project, ChiknError> {
    writer::write_project(&mut project)?;
    Ok(project)
}

#[tauri::command]
pub fn import_scrivener(scriv_path: String, output_path: String) -> Result<Project, ChiknError> {
    converter::import_scriv(Path::new(&scriv_path), Path::new(&output_path))
}

/// Opens a native file dialog that allows selecting .scriv packages.
/// macOS: uses AppleScript with Scrivener's UTI so packages are selectable.
/// Other platforms: .scriv is a regular directory, so uses a directory picker.
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
            // User cancelled
            return Ok(None);
        }

        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if path.is_empty() {
            Ok(None)
        } else {
            // AppleScript returns path with trailing slash — keep it, Path handles it
            Ok(Some(path.trim_end_matches('/').to_string()))
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
