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

/// Opens a native file dialog that allows selecting .scriv packages on macOS.
/// On other platforms, falls back to a regular directory picker.
#[tauri::command]
pub fn pick_scriv_folder() -> Result<Option<String>, ChiknError> {
    #[cfg(target_os = "macos")]
    {
        // Use JXA (JavaScript for Automation) to open NSOpenPanel with package selection enabled
        let script = r#"
            ObjC.import('AppKit');
            var panel = $.NSOpenPanel.openPanel;
            panel.canChooseFiles = true;
            panel.canChooseDirectories = true;
            panel.allowsMultipleSelection = false;
            panel.setTitle($.NSString.stringWithString('Select Scrivener Project'));
            panel.setPrompt($.NSString.stringWithString('Import'));
            var result = panel.runModal;
            if (result == $.NSModalResponseOK) {
                ObjC.unwrap(panel.URL.path);
            } else {
                '';
            }
        "#;

        let output = Command::new("osascript")
            .arg("-l")
            .arg("JavaScript")
            .arg("-e")
            .arg(script)
            .output()
            .map_err(|e| ChiknError::Unknown(format!("Failed to open file dialog: {}", e)))?;

        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if path.is_empty() {
            Ok(None)
        } else {
            Ok(Some(path))
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        // On Linux/Windows, .scriv is just a directory — use rfd
        let folder = rfd::FileDialog::new()
            .set_title("Select Scrivener Project")
            .pick_folder();
        Ok(folder.map(|p| p.to_string_lossy().to_string()))
    }
}
