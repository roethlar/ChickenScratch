//! Scrivener import/export API commands
//!
//! Tauri commands for .scriv ↔ .chikn conversion.

use crate::models::Project;
use crate::scrivener::{converter, exporter};
use crate::utils::error::ChiknError;
use std::path::Path;

/// Imports a Scrivener .scriv project to .chikn format.
///
/// # Arguments
/// * `scriv_path` - Path to .scriv directory
/// * `output_path` - Path where .chikn project will be created
///
/// # Returns
/// * `Ok(Project)` - Imported .chikn project
/// * `Err(ChiknError)` on failure
///
/// # Requirements
/// Requires Pandoc for RTF → Markdown conversion.
///
/// # Example (from frontend)
/// ```javascript
/// const project = await invoke('import_scrivener_project', {
///   scrivPath: '/Users/john/Documents/MyNovel.scriv',
///   outputPath: '/Users/john/Documents/MyNovel.chikn'
/// });
/// ```
#[tauri::command]
pub async fn import_scrivener_project(
    scriv_path: String,
    output_path: String,
) -> Result<Project, ChiknError> {
    let scriv = Path::new(&scriv_path);
    let output = Path::new(&output_path);

    converter::import_scriv(scriv, output)
}

/// Exports a .chikn project to Scrivener .scriv format.
///
/// # Arguments
/// * `project` - .chikn project to export
/// * `output_path` - Path where .scriv directory will be created
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(ChiknError)` on failure
///
/// # Requirements
/// Requires Pandoc for Markdown → RTF conversion.
///
/// # Example (from frontend)
/// ```javascript
/// await invoke('export_to_scrivener', {
///   project,
///   outputPath: '/Users/john/Documents/MyNovel.scriv'
/// });
/// ```
#[tauri::command]
pub async fn export_to_scrivener(project: Project, output_path: String) -> Result<(), ChiknError> {
    let output = Path::new(&output_path);

    exporter::export_to_scriv(&project, output)
}
