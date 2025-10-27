use std::path::Path;

use anyhow::{Context, Result};
use chicken_scratch::core::project::{reader, writer};
use chicken_scratch::Project;

/// Opens a `.chikn` project from disk.
pub fn open_project(path: &Path) -> Result<Project> {
    reader::read_project(path)
        .map_err(anyhow::Error::new)
        .with_context(|| format!("Failed to open project at {}", path.display()))
}

/// Saves the provided project back to disk.
pub fn save_project(project: &mut Project) -> Result<()> {
    writer::write_project(project)
        .map_err(anyhow::Error::new)
        .context("Unable to save project")
}
