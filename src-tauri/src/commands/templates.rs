use chickenscratch_core::core::project::{hierarchy, reader, writer};
use chickenscratch_core::{ChiknError, Document, Project, TreeNode};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub content: String,
}

fn default_templates() -> Vec<Template> {
    vec![
        Template {
            id: "scene".to_string(),
            name: "Scene".to_string(),
            content: "**POV:** \n\n**Setting:** \n\n**Goal:** \n\n---\n\n".to_string(),
        },
        Template {
            id: "chapter".to_string(),
            name: "Chapter".to_string(),
            content: "## Chapter Title\n\n".to_string(),
        },
        Template {
            id: "character".to_string(),
            name: "Character Sheet".to_string(),
            content: "## Character Name\n\n**Age:** \n\n**Occupation:** \n\n**Appearance:** \n\n### Personality\n\n**Traits:** \n\n**Motivations:** \n\n**Flaws:** \n\n### Background\n\n\n\n### Role in Story\n\n".to_string(),
        },
        Template {
            id: "setting".to_string(),
            name: "Setting".to_string(),
            content: "## Location Name\n\n**Type:** \n\n**Time period:** \n\n### Description\n\n\n\n### Atmosphere\n\n\n\n### Significance to Story\n\n".to_string(),
        },
        Template {
            id: "outline".to_string(),
            name: "Outline".to_string(),
            content: "## Act 1\n\n- Inciting incident\n- Key scenes\n\n## Act 2\n\n- Rising action\n- Midpoint\n\n## Act 3\n\n- Climax\n- Resolution\n".to_string(),
        },
    ]
}

#[tauri::command]
pub fn list_templates() -> Vec<Template> {
    default_templates()
}

#[tauri::command]
pub fn create_from_template(
    project_path: String,
    template_id: String,
    name: String,
    parent_id: Option<String>,
) -> Result<Project, ChiknError> {
    let mut project = reader::read_project(Path::new(&project_path))?;

    let template = default_templates()
        .into_iter()
        .find(|t| t.id == template_id)
        .ok_or_else(|| ChiknError::NotFound(format!("Template not found: {}", template_id)))?;

    let doc_id = uuid::Uuid::new_v4().to_string();
    let slug =
        chickenscratch_core::utils::slug::unique_slug(&name, "manuscript/", &project.documents);
    let doc_path = format!("manuscript/{}.md", slug);
    let now = chrono::Utc::now().to_rfc3339();

    let document = Document {
        id: doc_id.clone(),
        name: name.clone(),
        path: doc_path.clone(),
        content: template.content,
        parent_id: parent_id.clone(),
        created: now.clone(),
        modified: now,
        ..Default::default()
    };

    project.documents.insert(doc_id.clone(), document);

    let node = TreeNode::Document {
        id: doc_id,
        name,
        path: doc_path,
    };

    match parent_id {
        Some(pid) => hierarchy::add_child_to_folder(&mut project.hierarchy, &pid, node)?,
        None => {
            // Default to Manuscript folder
            let ms_id = project.hierarchy.iter().find_map(|n| {
                if let TreeNode::Folder { id, name, .. } = n {
                    if name == "Manuscript" {
                        Some(id.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            });
            if let Some(mid) = ms_id {
                hierarchy::add_child_to_folder(&mut project.hierarchy, &mid, node)?;
            } else {
                hierarchy::add_document_to_hierarchy(&mut project.hierarchy, node);
            }
        }
    }

    writer::write_project(&mut project)?;
    Ok(project)
}

#[tauri::command]
pub fn save_as_template(project_path: String, doc_id: String) -> Result<Template, ChiknError> {
    let project = reader::read_project(Path::new(&project_path))?;
    let doc = project
        .documents
        .get(&doc_id)
        .ok_or_else(|| ChiknError::NotFound(format!("Document not found: {}", doc_id)))?;

    // For now, return the template data. In the future, save to templates/ folder.
    Ok(Template {
        id: format!("custom-{}", uuid::Uuid::new_v4()),
        name: doc.name.clone(),
        content: doc.content.clone(),
    })
}
