//! Plot-thread commands. Threads are a novelist UI convention persisted as
//! `threads.yaml` at the project root. The format itself stays genre-agnostic;
//! these commands let any frontend manage them with a typed API.

use chickenscratch_core::core::project::{reader, writer};
use chickenscratch_core::{ChiknError, Project, Thread};
use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;
use tauri::State;

use super::{ProjectTokens, ProjectWriteLocks};

/// A dangling reference from a scene to an entity that no longer exists.
#[derive(Debug, Clone, Serialize)]
pub struct DanglingRef {
    pub doc_id: String,
    pub doc_name: String,
    /// "pov_character" | "location" | "characters_in_scene" | "threads"
    pub field: String,
    pub missing_id: String,
}

const MAX_ENTITIES_PER_TYPE: usize = 1024;
const MAX_ENTITY_PATH_DEPTH: usize = 8;

/// Walk every document's `fields` map and report references to entities or
/// threads that don't exist. Non-fatal — UIs surface this as a soft warning.
///
/// This validates only the novelist convention reference keys documented in
/// `docs/UI_CONVENTIONS_NOVELIST.md`: `pov_character`,
/// `characters_in_scene`, `location`, and `threads`. Other custom `fields`
/// keys are preserved as opaque UI-owned data and are not interpreted here.
#[tauri::command]
pub fn validate_references(project_path: String) -> Result<Vec<DanglingRef>, ChiknError> {
    // Pure query: the repairs-disabled read never touches the disk.
    let project = reader::read_project_readonly(Path::new(&project_path))?;

    // Build slug → name lookups. Slug = filename stem under the entity folder.
    let character_slugs = collect_entity_slugs(&project, "characters")?;
    let location_slugs = collect_entity_slugs(&project, "locations")?;
    let thread_ids: HashSet<String> = project.threads.iter().map(|t| t.id.clone()).collect();

    let mut dangling = Vec::new();
    for doc in project.documents.values() {
        let check = |field: &str,
                     value: &serde_yaml::Value,
                     set: &std::collections::HashSet<String>,
                     out: &mut Vec<DanglingRef>| {
            if let Some(s) = value.as_str() {
                if !s.is_empty() && !set.contains(s) {
                    out.push(DanglingRef {
                        doc_id: doc.id.clone(),
                        doc_name: doc.name.clone(),
                        field: field.to_string(),
                        missing_id: s.to_string(),
                    });
                }
            } else if let Some(seq) = value.as_sequence() {
                for v in seq {
                    if let Some(s) = v.as_str() {
                        if !s.is_empty() && !set.contains(s) {
                            out.push(DanglingRef {
                                doc_id: doc.id.clone(),
                                doc_name: doc.name.clone(),
                                field: field.to_string(),
                                missing_id: s.to_string(),
                            });
                        }
                    }
                }
            }
        };

        if let Some(v) = doc.fields.get("pov_character") {
            check("pov_character", v, &character_slugs, &mut dangling);
        }
        if let Some(v) = doc.fields.get("characters_in_scene") {
            check("characters_in_scene", v, &character_slugs, &mut dangling);
        }
        if let Some(v) = doc.fields.get("location") {
            check("location", v, &location_slugs, &mut dangling);
        }
        if let Some(v) = doc.fields.get("threads") {
            check("threads", v, &thread_ids, &mut dangling);
        }
    }
    Ok(dangling)
}

fn collect_entity_slugs(project: &Project, folder: &str) -> Result<HashSet<String>, ChiknError> {
    let prefix = format!("{folder}/");
    let mut slugs = HashSet::new();
    let mut count = 0usize;

    for doc in project
        .documents
        .values()
        .filter(|doc| doc.path.starts_with(&prefix))
    {
        count += 1;
        if count > MAX_ENTITIES_PER_TYPE {
            return Err(ChiknError::InvalidFormat(format!(
                "Too many {folder} entities; limit is {MAX_ENTITIES_PER_TYPE}"
            )));
        }

        let path = Path::new(&doc.path);
        let depth_under_folder = path.components().skip(1).count();
        if depth_under_folder > MAX_ENTITY_PATH_DEPTH {
            return Err(ChiknError::InvalidFormat(format!(
                "{folder} entity path is too deep: {}",
                doc.path
            )));
        }

        if let Some(slug) = path.file_stem().and_then(|s| s.to_str()) {
            slugs.insert(slug.to_string());
        }
    }

    Ok(slugs)
}

#[tauri::command]
pub fn list_threads(project_path: String) -> Result<Vec<Thread>, ChiknError> {
    // Pure query: the repairs-disabled read never touches the disk.
    let project = reader::read_project_readonly(Path::new(&project_path))?;
    Ok(project.threads)
}

#[tauri::command]
pub fn create_thread(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    name: String,
    color: Option<String>,
    description: Option<String>,
) -> Result<Project, ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(ChiknError::InvalidFormat(
                "Thread name cannot be empty".to_string(),
            ));
        }
        let token = tokens.checkout(&project_path)?;
        let mut project = reader::read_project(Path::new(&project_path))?;

        let id = unique_thread_id(trimmed, &project.threads);
        project.threads.push(Thread {
            id,
            name: trimmed.to_string(),
            color: color.and_then(non_empty),
            description: description.and_then(non_empty),
            extra: Default::default(),
        });
        writer::write_project(&mut project, &token)?;
        Ok(project)
    })
}

#[tauri::command]
pub fn update_thread(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    id: String,
    name: Option<String>,
    color: Option<String>,
    description: Option<String>,
) -> Result<Project, ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        let token = tokens.checkout(&project_path)?;
        let mut project = reader::read_project(Path::new(&project_path))?;
        let thread = project
            .threads
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| ChiknError::NotFound(format!("Thread not found: {}", id)))?;

        if let Some(n) = name.and_then(non_empty) {
            thread.name = n;
        }
        if let Some(c) = color {
            thread.color = non_empty(c);
        }
        if let Some(d) = description {
            thread.description = non_empty(d);
        }
        writer::write_project(&mut project, &token)?;
        Ok(project)
    })
}

/// Delete a thread. Strips the ref from every scene's `fields.threads` list
/// so we don't leave dangling references.
#[tauri::command]
pub fn delete_thread(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    id: String,
) -> Result<Project, ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        let token = tokens.checkout(&project_path)?;
        let mut project = reader::read_project(Path::new(&project_path))?;
        project.threads.retain(|t| t.id != id);

        for doc in project.documents.values_mut() {
            if let Some(value) = doc.fields.get_mut("threads") {
                if let Some(seq) = value.as_sequence_mut() {
                    seq.retain(|v| v.as_str().map(|s| s != id).unwrap_or(true));
                }
            }
        }
        writer::write_project(&mut project, &token)?;
        Ok(project)
    })
}

fn non_empty(s: String) -> Option<String> {
    if s.trim().is_empty() {
        None
    } else {
        Some(s)
    }
}

fn unique_thread_id(name: &str, existing: &[Thread]) -> String {
    let base: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let base = if base.is_empty() {
        "thread".to_string()
    } else {
        base
    };
    if !existing.iter().any(|t| t.id == base) {
        return base;
    }
    let mut n = 2;
    loop {
        let candidate = format!("{}-{}", base, n);
        if !existing.iter().any(|t| t.id == candidate) {
            return candidate;
        }
        n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chickenscratch_core::Document;
    use std::collections::HashMap;

    fn project_with_documents(paths: impl IntoIterator<Item = String>) -> Project {
        let mut documents = HashMap::new();
        for path in paths {
            let id = path.clone();
            documents.insert(
                id.clone(),
                Document {
                    id,
                    name: path.clone(),
                    path,
                    ..Default::default()
                },
            );
        }

        Project {
            id: "project".to_string(),
            name: "Project".to_string(),
            path: String::new(),
            hierarchy: Vec::new(),
            documents,
            created: String::new(),
            modified: String::new(),
            metadata: Default::default(),
            threads: Vec::new(),
        }
    }

    #[test]
    fn collect_entity_slugs_rejects_too_many_entities() {
        let paths = (0..=MAX_ENTITIES_PER_TYPE)
            .map(|idx| format!("characters/character-{idx}.md"))
            .collect::<Vec<_>>();
        let project = project_with_documents(paths);

        let result = collect_entity_slugs(&project, "characters");

        assert!(
            matches!(result, Err(ChiknError::InvalidFormat(message)) if message.contains("Too many characters entities"))
        );
    }

    #[test]
    fn collect_entity_slugs_rejects_too_deep_entity_path() {
        let project = project_with_documents([format!(
            "characters/{}/sarah.md",
            ["a", "b", "c", "d", "e", "f", "g", "h"].join("/")
        )]);

        let result = collect_entity_slugs(&project, "characters");

        assert!(
            matches!(result, Err(ChiknError::InvalidFormat(message)) if message.contains("entity path is too deep"))
        );
    }
}
