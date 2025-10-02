//! Project API commands
//!
//! Tauri commands for project CRUD operations

use crate::models::{Project, TreeNode};
use crate::utils::error::ChiknError;
use std::collections::HashMap;

/// Create a new .chikn project
#[tauri::command]
pub async fn create_project(name: String, path: String) -> Result<String, ChiknError> {
    // Generate project ID
    let id = uuid::Uuid::new_v4().to_string();

    // Create .chikn directory structure
    let project_path = format!("{}/{}.chikn", path, name);
    std::fs::create_dir_all(&project_path)?;
    std::fs::create_dir_all(format!("{}/manuscript", project_path))?;
    std::fs::create_dir_all(format!("{}/research", project_path))?;
    std::fs::create_dir_all(format!("{}/templates", project_path))?;
    std::fs::create_dir_all(format!("{}/settings", project_path))?;

    // Create initial project.yaml
    let project = Project {
        id: id.clone(),
        name: name.clone(),
        path: project_path.clone(),
        hierarchy: vec![
            TreeNode::Folder {
                id: "manuscript".to_string(),
                name: "Manuscript".to_string(),
                children: vec![],
            },
            TreeNode::Folder {
                id: "research".to_string(),
                name: "Research".to_string(),
                children: vec![],
            },
        ],
        documents: HashMap::new(),
        created: chrono::Utc::now().to_rfc3339(),
        modified: chrono::Utc::now().to_rfc3339(),
    };

    // Write project.yaml
    let yaml = serde_yaml::to_string(&project)
        .map_err(|e| ChiknError::Serialization(e))?;
    std::fs::write(format!("{}/project.yaml", project_path), yaml)?;

    Ok(project_path)
}

/// Load an existing .chikn project
#[tauri::command]
pub async fn load_project(path: String) -> Result<Project, ChiknError> {
    // Read project.yaml
    let yaml_path = format!("{}/project.yaml", path);
    let yaml_content = std::fs::read_to_string(&yaml_path)?;

    // Parse YAML
    let project: Project = serde_yaml::from_str(&yaml_content)
        .map_err(|e| ChiknError::Serialization(e))?;

    Ok(project)
}

/// Save project metadata (hierarchy, settings)
#[tauri::command]
pub async fn save_project(project: Project) -> Result<(), ChiknError> {
    // Write project.yaml
    let yaml = serde_yaml::to_string(&project)
        .map_err(|e| ChiknError::Serialization(e))?;
    std::fs::write(format!("{}/project.yaml", project.path), yaml)?;

    Ok(())
}
