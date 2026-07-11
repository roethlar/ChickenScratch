use chickenscratch_core::core::git;
use chickenscratch_core::core::project::fidelity::{self, Fidelity};
use chickenscratch_core::core::project::{reader, writer};
use chickenscratch_core::{ChiknError, Project};
use chikn_converter;
use serde::Serialize;
use std::path::Path;
#[cfg(target_os = "macos")]
use std::process::Command;
use tauri::State;

use super::{ProjectTokens, ProjectWriteLocks};

#[tauri::command]
pub fn create_project(
    name: String,
    path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
) -> Result<Project, ChiknError> {
    let project_path = Path::new(&path).join(format!("{}.chikn", name));
    write_locks.with_project_lock(&project_path, || {
        let mut project = writer::create_project(&project_path, &name)?;
        // A project the engine itself just initialized probes Full by
        // construction; checkout probes, issues, and caches the token.
        let token = tokens.checkout(&project_path)?;
        writer::write_project(&mut project, &token)?;
        let _ = git::save_revision(&project_path, &format!("Created project: {}", name), &token);
        Ok(project)
    })
}

/// Result of opening a project. `read_only` is true for Degraded projects
/// (see the fidelity probe): the project loads through the
/// repairs-disabled read, no write token exists, and every mutating
/// command refuses. `read_only_reasons` carries plain-English
/// explanations for the UI banner.
#[derive(Debug, Clone, Serialize)]
pub struct LoadedProject {
    pub project: Project,
    pub read_only: bool,
    pub read_only_reasons: Vec<String>,
}

#[tauri::command]
pub fn load_project(
    path: String,
    tokens: State<'_, ProjectTokens>,
) -> Result<LoadedProject, ChiknError> {
    let project_path = Path::new(&path);
    // Probe BEFORE anything touches the project: the probe is
    // side-effect-free, and classification decides which read path runs.
    match fidelity::probe_project_fidelity(project_path)? {
        Fidelity::Full => {
            let token = fidelity::acquire_write_token(project_path)?;
            tokens.store(project_path, token)?;
            let project = reader::read_project(project_path)?;
            Ok(LoadedProject {
                project,
                read_only: false,
                read_only_reasons: Vec::new(),
            })
        }
        Fidelity::Degraded { reasons } => {
            tokens.invalidate(project_path);
            let project = reader::read_project_readonly(project_path)?;
            Ok(LoadedProject {
                project,
                read_only: true,
                read_only_reasons: reasons.iter().map(ToString::to_string).collect(),
            })
        }
    }
}

#[tauri::command]
pub fn save_project(
    mut project: Project,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
) -> Result<Project, ChiknError> {
    let project_path = project.path.clone();
    write_locks.with_project_lock(&project_path, || {
        let token = tokens.checkout(&project_path)?;
        writer::write_project(&mut project, &token)?;
        Ok(project)
    })
}

#[tauri::command]
pub fn import_scrivener(
    scriv_path: String,
    output_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
) -> Result<Project, ChiknError> {
    let settings = crate::commands::settings::get_app_settings();
    let (pandoc_path, _) = crate::commands::settings::resolve_pandoc(&settings)?;
    write_locks.with_project_lock(&output_path, || {
        let project = chikn_converter::import_scriv(
            Path::new(&scriv_path),
            Path::new(&output_path),
            Some(pandoc_path.as_path()),
        )?;
        // Warm the token cache for the freshly imported (Full) project.
        let _ = tokens.checkout(&output_path);
        Ok(project)
    })
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
    session_target: Option<chickenscratch_core::SessionTarget>,
) -> Result<Project, ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        let token = tokens.checkout(&project_path)?;
        let mut project = reader::read_project(Path::new(&project_path))?;
        project.metadata.title = title;
        project.metadata.author = author;
        project.metadata.project_type = project_type;
        project.metadata.genre = genre;
        project.metadata.theme = theme;
        project.metadata.summary = summary;
        project.metadata.session_target = session_target.filter(|t| !t.is_empty());
        writer::write_project(&mut project, &token)?;
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
