use chickenscratch_core::core::project::{reader, writer};
use chickenscratch_core::scrivener::converter;
use chickenscratch_core::{ChiknError, Project};
use std::path::Path;

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
