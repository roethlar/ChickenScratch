use chickenscratch_core::core::git;
use chickenscratch_core::core::project::fidelity::{self, Fidelity};
use chickenscratch_core::core::project::{reader, writer};
use chickenscratch_core::{ChiknError, Project};
use chikn_converter;
use serde::Serialize;
use std::path::Path;

use super::{ProjectTokens, ProjectWriteLocks};
pub fn create_project(
    name: String,
    path: String,
    write_locks: &ProjectWriteLocks,
    tokens: &ProjectTokens,
) -> Result<Project, ChiknError> {
    let project_path = Path::new(&path).join(format!("{}.chikn", name));
    write_locks.with_project_lock(&project_path, || {
        let mut project = writer::create_project(&project_path, &name)?;
        tokens.with_write_permit(&project_path, |permit| {
            writer::write_project(&mut project, permit)?;
            let _ =
                git::save_revision(&project_path, &format!("Created project: {}", name), permit);
            Ok(project)
        })
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

/// Inner body shared with the command-boundary tests: after a restart
/// with a conflicted `project.yaml`, the open itself must succeed
/// (read-only, recovery read) — asserting only that the recovery commands
/// run would not prove the display path.
pub(crate) fn load_project_inner(
    path: &str,
    write_locks: &ProjectWriteLocks,
    tokens: &ProjectTokens,
) -> Result<LoadedProject, ChiknError> {
    let project_path = Path::new(path);
    // Probe BEFORE anything touches the project: the probe is
    // side-effect-free, and classification decides which read path runs.
    // Mid-merge the probe may error outright (conflict markers in
    // project.yaml) or come back Degraded (markers in a .meta) — by
    // design. The recovery read keeps the project openable, read-only,
    // so the merge-in-progress UI can offer Complete/Abort after a
    // restart instead of stranding the writer (plan slice 4; the
    // unopenable-after-restart case was a live bug).
    let probe = fidelity::probe_project_fidelity(project_path);
    let mid_merge = matches!(git::merge_state(project_path), Ok(s) if s.in_progress);
    match probe {
        Ok(Fidelity::Full) => write_locks.with_project_lock(project_path, || {
            let token = fidelity::acquire_write_token(project_path)?;
            let project = token.with_write_permit(project_path, |permit| {
                reader::read_project_with_repair(project_path, permit)
            })?;
            tokens.store(project_path, token)?;
            Ok(LoadedProject {
                project,
                read_only: false,
                read_only_reasons: Vec::new(),
            })
        }),
        Ok(Fidelity::Degraded { reasons }) => {
            tokens.invalidate(project_path);
            let project = if mid_merge {
                reader::read_project_recovery(project_path)?
            } else {
                reader::read_project_readonly(project_path)?
            };
            Ok(LoadedProject {
                project,
                read_only: true,
                read_only_reasons: reasons.iter().map(ToString::to_string).collect(),
            })
        }
        Err(probe_err) => {
            if mid_merge {
                tokens.invalidate(project_path);
                let project = reader::read_project_recovery(project_path)?;
                Ok(LoadedProject {
                    project,
                    read_only: true,
                    read_only_reasons: vec![
                        "a merge is in progress — complete or abort it to resume editing"
                            .to_string(),
                    ],
                })
            } else {
                Err(probe_err)
            }
        }
    }
}
pub fn load_project(
    path: String,
    write_locks: &ProjectWriteLocks,
    tokens: &ProjectTokens,
) -> Result<LoadedProject, ChiknError> {
    load_project_inner(&path, write_locks, tokens)
}
pub fn save_project(
    mut project: Project,
    write_locks: &ProjectWriteLocks,
    tokens: &ProjectTokens,
) -> Result<Project, ChiknError> {
    let project_path = project.path.clone();
    write_locks.with_project_lock(&project_path, || {
        tokens.with_write_permit(&project_path, |permit| {
            writer::write_project(&mut project, permit)?;
            Ok(project)
        })
    })
}

/// Result of a Scrivener import: the new project plus plain-English
/// problems that did not stop the import (for the UI to show).
#[derive(Debug, Clone, Serialize)]
pub struct ImportedProject {
    pub project: Project,
    pub warnings: Vec<String>,
}
pub fn import_scrivener(
    scriv_path: String,
    output_path: String,
    write_locks: &ProjectWriteLocks,
    tokens: &ProjectTokens,
) -> Result<ImportedProject, ChiknError> {
    let settings = crate::settings::get_app_settings();
    let (pandoc_path, _) = crate::settings::resolve_pandoc(&settings)?;
    write_locks.with_project_lock(&output_path, || {
        let report = chikn_converter::import_scriv_with_report(
            Path::new(&scriv_path),
            Path::new(&output_path),
            Some(pandoc_path.as_path()),
        )?;
        // Warm the token cache for the freshly imported (Full) project.
        if let Ok(token) = fidelity::acquire_write_token(Path::new(&output_path)) {
            let _ = tokens.store(Path::new(&output_path), token);
        }
        Ok(ImportedProject {
            project: report.project,
            warnings: report.warnings,
        })
    })
}
#[allow(clippy::too_many_arguments)]
pub fn update_project_metadata(
    project_path: String,
    write_locks: &ProjectWriteLocks,
    tokens: &ProjectTokens,
    title: Option<String>,
    author: Option<String>,
    project_type: Option<String>,
    genre: Option<String>,
    theme: Option<String>,
    summary: Option<String>,
    session_target: Option<chickenscratch_core::SessionTarget>,
) -> Result<Project, ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        tokens.with_write_permit(&project_path, |permit| {
            let mut project = reader::read_project(Path::new(&project_path))?;
            project.metadata.title = title;
            project.metadata.author = author;
            project.metadata.project_type = project_type;
            project.metadata.genre = genre;
            project.metadata.theme = theme;
            project.metadata.summary = summary;
            project.metadata.session_target = session_target.filter(|t| !t.is_empty());
            writer::write_project(&mut project, permit)?;
            Ok(project)
        })
    })
}
