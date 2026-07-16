//! # Project Reader
//!
//! Reads .chikn project files from disk into Project structs.
//!
//! ## Responsibilities
//! - Load project.yaml and parse into Project
//! - Read all document content (.md files)
//! - Build document hierarchy from filesystem
//! - Validate project structure
//!
//! ## Example
//! ```no_run
//! use std::path::Path;
//! use chickenscratch_core::core::project::reader::read_project;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let project = read_project(Path::new("MyNovel.chikn"))?;
//! println!("Loaded project: {}", project.name);
//! # Ok(()) }
//! ```

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::fidelity::WritePermit;
use super::format::{
    get_characters_path, get_document_meta_path, get_locations_path, get_manuscript_path,
    get_project_file_path, get_research_path, get_threads_path, DOCUMENT_EXTENSION,
};
use super::safe_path;
use crate::models::{Document, Project, Thread, TreeNode};
use crate::utils::error::ChiknError;

/// Project metadata structure as stored in project.yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    /// On-disk format version (see `format::FORMAT_VERSION`). Absent in
    /// projects written before v1.2 locked the format; any value is
    /// accepted on read. Declared first so it serializes at the top of
    /// project.yaml.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format_version: Option<String>,

    /// Project unique ID
    pub id: String,

    /// Project name
    pub name: String,

    /// Document hierarchy (root nodes)
    pub hierarchy: Vec<TreeNode>,

    /// Project creation timestamp
    pub created: String,

    /// Last modified timestamp
    pub modified: String,

    /// Project-level metadata
    #[serde(default)]
    pub metadata: crate::models::ProjectMeta,

    /// Unknown top-level keys, preserved verbatim across read→write cycles
    /// (tolerant readers, preserving writers — INVARIANTS.md I5). Sorted map
    /// so re-emission is deterministic. Declared last so preserved keys
    /// serialize after the known schema.
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, serde_yaml::Value>,
}

/// Document metadata structure as stored in .meta files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// Document unique ID
    #[serde(default = "generate_id")]
    pub id: String,

    /// Human-readable display name (e.g., "Chapter 1")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Creation timestamp
    #[serde(default = "current_timestamp")]
    pub created: String,

    /// Last modified timestamp
    #[serde(default = "current_timestamp")]
    pub modified: String,

    /// Parent ID (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,

    // Scrivener metadata (Phase 2)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub synopsis: Option<String>,

    /// Scrivener section type UUID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section_type: Option<String>,

    /// Include in compile flag.
    ///
    /// The Rust canonical writer emits `"Yes"` / `"No"` strings (legacy
    /// Scrivener convention). Other frontends — notably the Windows C# writer
    /// before this fix — wrote a YAML boolean. Accept either on read so a
    /// project authored on any frontend reopens cleanly.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_include_in_compile"
    )]
    pub include_in_compile: Option<String>,

    /// Original Scrivener UUID (for round-trip)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scrivener_uuid: Option<String>,

    /// IDs of related documents (connections)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub links: Option<Vec<String>>,

    /// Word count target
    #[serde(default)]
    pub word_count_target: u32,

    /// Custom compile order
    #[serde(default)]
    pub compile_order: i32,

    /// Comments anchored to spans in the content
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<crate::models::Comment>,

    /// Generic UI extensibility — see `Document::fields`. Sorted map for
    /// one canonical serialized key order.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub fields: std::collections::BTreeMap<String, serde_yaml::Value>,

    /// Unknown top-level sidecar keys, preserved verbatim across read→write
    /// cycles (tolerant readers, preserving writers — INVARIANTS.md I5).
    /// `fields` is the sanctioned extensibility surface; this map only
    /// guarantees that keys written by other or newer tools are never
    /// silently destroyed by a save.
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, serde_yaml::Value>,
}

/// The six novelist keys commit `10ec683` briefly added as top-level typed
/// sidecar fields before the format was locked as genre-agnostic (see
/// docs/plans/PHASE_FORMAT_FINALIZATION.md). MIGRATION SHIM ONLY — not typed
/// fields (I4). Sidecars written during that window carry these at the
/// `.meta` top level, where no current UI looks; they are lifted into
/// `fields`, where docs/UI_CONVENTIONS_NOVELIST.md places them, so the data
/// resurfaces. `fields` wins when it already has the key.
const LEGACY_NOVELIST_KEYS: [&str; 6] = [
    "pov_character",
    "location",
    "story_time",
    "duration_minutes",
    "threads",
    "characters_in_scene",
];

/// Move legacy top-level novelist keys (captured in `extra`) into `fields`.
/// Existing `fields` entries win; the stale top-level duplicate is dropped.
/// Applied on read (so the data is visible in memory) and on the write-time
/// merge (so the next save relocates the keys on disk).
pub(crate) fn lift_legacy_novelist_keys(metadata: &mut DocumentMetadata) {
    for key in LEGACY_NOVELIST_KEYS {
        if let Some(value) = metadata.extra.remove(key) {
            metadata.fields.entry(key.to_string()).or_insert(value);
        }
    }
}

#[derive(Clone, Debug)]
struct HierarchyDocumentIdentity {
    id: String,
    name: String,
}

/// Helper function to generate a new UUID
fn generate_id() -> String {
    Uuid::new_v4().to_string()
}

/// Accept either `"Yes"`/`"No"` strings or a YAML boolean for
/// `include_in_compile`. Booleans coerce to the canonical string form so
/// downstream code only deals with one shape.
fn deserialize_include_in_compile<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum BoolOrStr {
        Bool(bool),
        Str(String),
    }
    let opt: Option<BoolOrStr> = Option::deserialize(deserializer)?;
    Ok(opt.map(|v| match v {
        BoolOrStr::Bool(true) => "Yes".to_string(),
        BoolOrStr::Bool(false) => "No".to_string(),
        BoolOrStr::Str(s) => s,
    }))
}

/// Helper function to get current timestamp
fn current_timestamp() -> String {
    Utc::now().to_rfc3339()
}

/// Reads a .chikn project from disk.
///
/// Side-effect-free: missing folders and corrupt sidecars are represented in
/// memory without creating, renaming, or rewriting anything on disk. Use
/// [`read_project_with_repair`] only for an explicitly authorized Full open.
///
/// # Arguments
/// * `path` - Path to .chikn project directory
///
/// # Returns
/// Complete Project struct with all documents loaded
///
/// # Errors
/// - `NotFound`: Project path doesn't exist
/// - `InvalidFormat`: Missing required files/folders
/// - `Io`: File system errors during reading
/// - `Serialization`: YAML parsing errors
///
/// # Example
/// ```no_run
/// use std::path::Path;
/// use chickenscratch_core::core::project::reader::read_project;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let project = read_project(Path::new("MyNovel.chikn"))?;
/// # Ok(()) }
/// ```
pub fn read_project(path: &Path) -> Result<Project, ChiknError> {
    read_project_impl(path, RepairMode::ReadOnly)
}

/// Explicit, permit-backed repair for a project that freshly probed Full.
///
/// This retains benign self-heal for missing standard folders. Corrupt
/// sidecars are Degraded and are never quarantine-renamed by a read.
pub fn read_project_with_repair(
    path: &Path,
    permit: &WritePermit<'_>,
) -> Result<Project, ChiknError> {
    permit.ensure_valid_for(path)?;
    read_project_impl(path, RepairMode::SelfHeal)
}

/// Source-compatible name for an explicitly side-effect-free read.
///
/// Public [`read_project`] now has the same behavior; this alias remains for
/// callers that want to state read-only intent at the call site.
pub fn read_project_readonly(path: &Path) -> Result<Project, ChiknError> {
    read_project(path)
}

/// Display-only open for a project carrying an in-progress merge whose
/// worktree format files may be conflicted (plan slice 4, review rounds
/// 9–11). Two relaxations, both scoped to this entry point only:
///
/// 1. `project.yaml` metadata falls back to the pre-merge `HEAD` copy
///    when the worktree file fails to parse — conflict markers in the
///    root file otherwise make the project unopenable after restart,
///    stranding the writer outside the Complete/Abort recovery UI.
/// 2. The strict hierarchy↔documents matching is skipped: mid-merge the
///    tree is definitionally inconsistent (e.g. a remote delete/recreate
///    at the same path changes a document's id), and a hard error here
///    would fail the open. Skewed entries load as unlinked; the ordinary
///    load path keeps strict validation unchanged.
///
/// Never writes: repair stays `ReadOnly`, and callers must present the
/// result read-only until the merge is completed or aborted.
pub fn read_project_recovery(path: &Path) -> Result<Project, ChiknError> {
    validate_project_root(path)?;

    let metadata = match read_project_metadata(path) {
        Ok(m) => m,
        Err(worktree_err) => head_project_metadata(path).map_err(|head_err| {
            ChiknError::InvalidFormat(format!(
                "project.yaml is unreadable mid-merge ({worktree_err}) and no pre-merge \
                     copy could be recovered ({head_err})"
            ))
        })?,
    };
    validate_hierarchy_document_paths(&metadata.hierarchy)?;

    let hierarchy_identities = collect_hierarchy_document_identities(&metadata.hierarchy)?;
    let documents = read_all_documents(path, &hierarchy_identities, RepairMode::ReadOnly)?;
    // Deliberately NO validate_hierarchy_documents_match_loaded_documents:
    // HEAD metadata + worktree sidecars can skew mid-merge.

    let mut project = Project {
        id: metadata.id,
        name: metadata.name,
        path: path.to_string_lossy().to_string(),
        hierarchy: metadata.hierarchy,
        documents,
        created: metadata.created,
        modified: metadata.modified,
        metadata: metadata.metadata,
        threads: read_threads(path)?,
    };
    let _ = repair_project(&mut project, path, RepairMode::ReadOnly);
    Ok(project)
}

/// Parse `project.yaml` as committed at `HEAD` — the last agreed state
/// before the in-progress merge rewrote the worktree copy.
fn head_project_metadata(path: &Path) -> Result<ProjectMetadata, ChiknError> {
    let repo = git2::Repository::open(path)
        .map_err(|e| ChiknError::Unknown(format!("Not a git repo: {e}")))?;
    let head_tree = repo
        .head()
        .and_then(|h| h.peel_to_commit())
        .and_then(|c| c.tree())
        .map_err(|e| ChiknError::Unknown(format!("HEAD tree: {e}")))?;
    let entry = head_tree
        .get_path(Path::new(super::format::PROJECT_FILE))
        .map_err(|e| ChiknError::NotFound(format!("project.yaml not in HEAD: {e}")))?;
    let blob = repo
        .find_blob(entry.id())
        .map_err(|e| ChiknError::Unknown(format!("project.yaml blob: {e}")))?;
    let content = std::str::from_utf8(blob.content())
        .map_err(|e| ChiknError::InvalidFormat(format!("project.yaml not UTF-8 in HEAD: {e}")))?;
    serde_yaml::from_str(content).map_err(ChiknError::Serialization)
}

/// Whether a load may touch the disk to self-heal.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RepairMode {
    /// Explicitly authorized Full-project open: recreate missing standard
    /// folders. Corrupt sidecars are never mutated by a read.
    SelfHeal,
    /// Degraded open: pure read, disk stays byte-identical.
    ReadOnly,
}

fn read_project_impl(path: &Path, repair_mode: RepairMode) -> Result<Project, ChiknError> {
    // F-012: previously `validate_project_structure` ran first and rejected
    // the project the moment any of `manuscript/`, `research/`, `templates/`,
    // `settings/` was missing — even though the rest of the read+repair flow
    // can recreate them. The TODO/roadmap describe the project as
    // self-healing, so honor that claim by giving repair a chance before
    // strict validation.
    //
    // Split into two layers:
    //  1. `validate_project_root(path)` — non-recoverable failures (path
    //     missing, not a directory, no `project.yaml`). These genuinely
    //     mean "not a chikn project".
    //  2. `pre_repair_folders(path)` — silently recreate missing required
    //     subfolders. Repair failures are logged to stderr but don't block
    //     load; the user sees broken state rather than a confusing error
    //     screen, and the next save attempt will surface the real fs problem.
    validate_project_root(path)?;
    if repair_mode == RepairMode::SelfHeal {
        pre_repair_folders(path);
    }

    // Read project.yaml
    let metadata = read_project_metadata(path)?;
    validate_hierarchy_document_paths(&metadata.hierarchy)?;

    // Read all documents from manuscript and research folders
    let hierarchy_identities = collect_hierarchy_document_identities(&metadata.hierarchy)?;
    let documents = read_all_documents(path, &hierarchy_identities, repair_mode)?;
    validate_hierarchy_documents_match_loaded_documents(&metadata.hierarchy, &documents)?;

    let mut project = Project {
        id: metadata.id,
        name: metadata.name,
        path: path.to_string_lossy().to_string(),
        hierarchy: metadata.hierarchy,
        documents,
        created: metadata.created,
        modified: metadata.modified,
        metadata: metadata.metadata,
        threads: read_threads(path)?,
    };

    // Reconcile hierarchy with actual files on disk
    let repaired = repair_project(&mut project, path, repair_mode);
    if repaired {
        eprintln!(
            "Repaired {} in memory; project.yaml was not rewritten during load.",
            path.display()
        );
    }

    Ok(project)
}

/// Soft top-level validation: only the failures that genuinely mean "not a
/// chikn project". Sub-folder presence is handled by `pre_repair_folders`
/// (the format claims self-healing — F-012).
fn validate_project_root(path: &Path) -> Result<(), ChiknError> {
    if !path.exists() {
        return Err(ChiknError::NotFound(format!(
            "Project path does not exist: {}",
            path.display()
        )));
    }
    if !path.is_dir() {
        return Err(ChiknError::InvalidFormat(format!(
            "Project path is not a directory: {}",
            path.display()
        )));
    }
    let project_file = path.join(super::format::PROJECT_FILE);
    if !project_file.exists() {
        return Err(ChiknError::InvalidFormat(format!(
            "Missing required file: {}",
            super::format::PROJECT_FILE
        )));
    }
    let metadata = fs::symlink_metadata(&project_file)?;
    if metadata.file_type().is_symlink() {
        return Err(ChiknError::InvalidFormat(format!(
            "Project metadata is a symlink: {}",
            project_file.display()
        )));
    }
    Ok(())
}

/// Best-effort: create any missing required subfolders before the rest of
/// the read pipeline runs. Failures here are logged but don't abort the
/// load — the surrounding repair pass will try again, and a real fs problem
/// will surface during the next write.
fn pre_repair_folders(path: &Path) {
    let project_root = match path.canonicalize() {
        Ok(root) => root,
        Err(e) => {
            eprintln!(
                "pre-repair: failed to resolve project root {}: {} — continuing anyway",
                path.display(),
                e
            );
            return;
        }
    };

    for folder in super::format::REQUIRED_FOLDERS {
        match create_required_folder_if_safe(path, &project_root, folder) {
            Ok(true) => {
                eprintln!(
                    "pre-repair: created missing folder {}",
                    path.join(folder).display()
                );
            }
            Ok(false) => {}
            Err(e) => {
                eprintln!(
                    "pre-repair: failed to create {} safely: {} — continuing anyway",
                    path.join(folder).display(),
                    e
                );
            }
        }
    }
}

fn create_required_folder_if_safe(
    project_path: &Path,
    _project_root: &Path,
    folder: &str,
) -> Result<bool, ChiknError> {
    let (_, created) =
        safe_path::ensure_project_subdir_safe_with_status(project_path, Path::new(folder))?;
    Ok(created)
}

/// Reconciles project.yaml hierarchy with actual files on disk in memory.
/// Returns true if any additive repairs were made.
///
/// This deliberately avoids destructive pruning: a transient filesystem miss
/// must not remove document references from the loaded project and then
/// persist that smaller model on the next save. Missing files are logged, while
/// additive repairs like orphan adoption still happen in memory.
///
/// Handles:
/// 1. Hierarchy references a file that doesn't exist — keep it and warn
/// 2. A .md file exists on disk but isn't in hierarchy — add to hierarchy
/// 3. Document in the loaded map has no file on disk — keep it and warn
fn repair_project(project: &mut Project, project_path: &Path, repair_mode: RepairMode) -> bool {
    let mut repaired = false;

    // Pass 1: Warn on hierarchy entries that point to missing files. Keep the
    // references so a temporary sync/network miss does not become data loss.
    let missing_paths = missing_hierarchy_paths(&project.hierarchy, project_path);
    if !missing_paths.is_empty() {
        eprintln!(
            "Repair warning: {} hierarchy entries point to missing files and were kept: {}",
            missing_paths.len(),
            missing_paths.join(", ")
        );
    }

    // Pass 2: Find documents on disk that aren't in the hierarchy.
    // Entities under characters/ and locations/ live outside the hierarchy
    // by design — UIs surface them in dedicated sections by walking
    // `project.documents`. Adding them here would relocate them into the
    // main binder tree on every reload.
    let referenced_paths = collect_hierarchy_paths(&project.hierarchy);
    let mut orphans: Vec<(String, String, String)> = Vec::new(); // (id, name, path)
    for doc in project.documents.values() {
        if doc.path.starts_with("characters/") || doc.path.starts_with("locations/") {
            continue;
        }
        if !referenced_paths.contains(&doc.path) {
            orphans.push((doc.id.clone(), doc.name.clone(), doc.path.clone()));
        }
    }

    if !orphans.is_empty() {
        eprintln!(
            "Repaired: adding {} orphaned documents to hierarchy",
            orphans.len()
        );
        for (id, name, path) in orphans {
            project
                .hierarchy
                .push(TreeNode::Document { id, name, path });
        }
        repaired = true;
    }

    // Pass 3: Warn if documents in the map have no file on disk. Keep them in
    // memory for the same reason as hierarchy references above.
    let missing_ids: Vec<String> = project
        .documents
        .iter()
        .filter(|(_, doc)| {
            let full = project_path.join(&doc.path);
            !full.exists()
        })
        .map(|(id, _)| id.clone())
        .collect();

    if !missing_ids.is_empty() {
        eprintln!(
            "Repair warning: {} loaded documents have missing files and were kept in memory: {}",
            missing_ids.len(),
            missing_ids.join(", ")
        );
    }

    // Pass 4: Ensure default directories exist on disk when doing so is safe.
    // Disk repairs are the one pass a read-only (Degraded) open must skip:
    // the open has to leave the folder byte-identical.
    if repair_mode == RepairMode::SelfHeal {
        match project_path.canonicalize() {
            Ok(project_root) => {
                for folder in super::format::REQUIRED_FOLDERS {
                    match create_required_folder_if_safe(project_path, &project_root, folder) {
                        Ok(true) => {
                            eprintln!(
                                "Repaired: created missing folder on disk: {}",
                                project_path.join(folder).display()
                            );
                            repaired = true;
                        }
                        Ok(false) => {}
                        Err(e) => {
                            eprintln!(
                                "Repair skipped unsafe folder creation for {}: {}",
                                project_path.join(folder).display(),
                                e
                            );
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "Repair skipped required folder creation for {}: {}",
                    project_path.display(),
                    e
                );
            }
        }
    }

    // Pass 5: Ensure Manuscript and Research folders are in the hierarchy
    // (Templates and Settings are internal — they live on disk but not in the binder)
    let binder_folders: &[&str] = &["Manuscript", "Research", "Trash"];
    for name in binder_folders {
        let folder_name_lower = name.to_lowercase();
        let in_hierarchy = project.hierarchy.iter().any(|node| {
            matches!(node, TreeNode::Folder { name: n, .. } if n.to_lowercase() == folder_name_lower)
        });

        if !in_hierarchy {
            project.hierarchy.push(TreeNode::Folder {
                id: Uuid::new_v4().to_string(),
                name: name.to_string(),
                children: Vec::new(),
            });
            eprintln!("Repaired: added {} folder to hierarchy", name);
            repaired = true;
        }
    }

    repaired
}

fn missing_hierarchy_paths(hierarchy: &[TreeNode], project_path: &Path) -> Vec<String> {
    let mut result = Vec::new();
    for node in hierarchy {
        match node {
            TreeNode::Document { path, .. } => {
                let full = project_path.join(path);
                if !full.exists() {
                    result.push(path.clone());
                }
            }
            TreeNode::Folder { children, .. } => {
                result.extend(missing_hierarchy_paths(children, project_path));
            }
        }
    }
    result
}

fn validate_hierarchy_document_paths(hierarchy: &[TreeNode]) -> Result<(), ChiknError> {
    for node in hierarchy {
        match node {
            TreeNode::Document { path, .. } => validate_relative_document_path(path)?,
            TreeNode::Folder { children, .. } => validate_hierarchy_document_paths(children)?,
        }
    }
    Ok(())
}

/// Counts total Document nodes in a hierarchy.
#[cfg(test)]
fn count_hierarchy_docs(hierarchy: &[TreeNode]) -> usize {
    let mut count = 0;
    for node in hierarchy {
        match node {
            TreeNode::Document { .. } => count += 1,
            TreeNode::Folder { children, .. } => count += count_hierarchy_docs(children),
        }
    }
    count
}

/// Collects all document paths referenced in the hierarchy.
fn collect_hierarchy_paths(hierarchy: &[TreeNode]) -> std::collections::HashSet<String> {
    let mut paths = std::collections::HashSet::new();
    collect_paths_inner(hierarchy, &mut paths);
    paths
}

fn collect_paths_inner(hierarchy: &[TreeNode], paths: &mut std::collections::HashSet<String>) {
    for node in hierarchy {
        match node {
            TreeNode::Document { path, .. } => {
                paths.insert(path.clone());
            }
            TreeNode::Folder { children, .. } => {
                collect_paths_inner(children, paths);
            }
        }
    }
}

fn collect_hierarchy_document_identities(
    hierarchy: &[TreeNode],
) -> Result<HashMap<String, HierarchyDocumentIdentity>, ChiknError> {
    let mut identities = HashMap::new();
    let mut document_ids = HashSet::new();
    let mut document_paths = HashSet::new();
    collect_hierarchy_document_identities_inner(
        hierarchy,
        &mut identities,
        &mut document_ids,
        &mut document_paths,
    )?;
    Ok(identities)
}

fn collect_hierarchy_document_identities_inner(
    hierarchy: &[TreeNode],
    identities: &mut HashMap<String, HierarchyDocumentIdentity>,
    document_ids: &mut HashSet<String>,
    document_paths: &mut HashSet<String>,
) -> Result<(), ChiknError> {
    for node in hierarchy {
        match node {
            TreeNode::Document { id, name, path } => {
                if !document_ids.insert(id.clone()) {
                    return Err(ChiknError::InvalidFormat(format!(
                        "Duplicate hierarchy document id: {id}"
                    )));
                }
                let normalized_path = normalized_relative_document_path(path)?;
                if !document_paths.insert(normalized_path.clone()) {
                    return Err(ChiknError::InvalidFormat(format!(
                        "Duplicate hierarchy document path: {path}"
                    )));
                }
                identities.insert(
                    normalized_path,
                    HierarchyDocumentIdentity {
                        id: id.clone(),
                        name: name.clone(),
                    },
                );
            }
            TreeNode::Folder { children, .. } => {
                collect_hierarchy_document_identities_inner(
                    children,
                    identities,
                    document_ids,
                    document_paths,
                )?;
            }
        }
    }
    Ok(())
}

/// Reads project.yaml and parses into ProjectMetadata
fn read_project_metadata(path: &Path) -> Result<ProjectMetadata, ChiknError> {
    let project_file = get_project_file_path(path);
    let project_root = canonical_project_root(path)?;
    ensure_existing_read_file_safe(&project_file, &project_root, "project metadata")?;

    let content = fs::read_to_string(&project_file).map_err(ChiknError::Io)?;

    let metadata: ProjectMetadata =
        serde_yaml::from_str(&content).map_err(ChiknError::Serialization)?;

    Ok(metadata)
}

/// Reads all documents from manuscript and research folders
fn read_all_documents(
    project_path: &Path,
    hierarchy_identities: &HashMap<String, HierarchyDocumentIdentity>,
    repair_mode: RepairMode,
) -> Result<HashMap<String, Document>, ChiknError> {
    let mut documents = HashMap::new();
    let mut document_paths = HashSet::new();
    let project_root = canonical_project_root(project_path)?;

    // Read from the four document roots (recursively). characters/ and
    // locations/ are optional novelist conventions — projects without
    // them are still valid.
    for folder_path in [
        get_manuscript_path(project_path),
        get_research_path(project_path),
        get_characters_path(project_path),
        get_locations_path(project_path),
    ] {
        read_optional_document_root(
            &folder_path,
            project_path,
            &project_root,
            hierarchy_identities,
            &mut documents,
            &mut document_paths,
            repair_mode,
        )?;
    }

    Ok(documents)
}

/// Reads `threads.yaml` if present. Missing file → empty vec (no error).
fn read_threads(project_path: &Path) -> Result<Vec<Thread>, ChiknError> {
    let path = get_threads_path(project_path);
    let project_root = canonical_project_root(project_path)?;
    if !ensure_optional_read_file_safe(&path, &project_root, "threads metadata")? {
        return Ok(Vec::new());
    }
    let body = fs::read_to_string(&path)?;
    if body.trim().is_empty() {
        return Ok(Vec::new());
    }
    #[derive(serde::Deserialize)]
    struct ThreadsFile {
        #[serde(default)]
        threads: Vec<Thread>,
    }
    let parsed: ThreadsFile = serde_yaml::from_str(&body)?;
    Ok(parsed.threads)
}

#[allow(clippy::too_many_arguments)]
fn read_optional_document_root(
    folder_path: &Path,
    project_path: &Path,
    project_root: &Path,
    hierarchy_identities: &HashMap<String, HierarchyDocumentIdentity>,
    documents: &mut HashMap<String, Document>,
    document_paths: &mut HashSet<String>,
    repair_mode: RepairMode,
) -> Result<(), ChiknError> {
    match fs::symlink_metadata(folder_path) {
        Ok(metadata) => {
            ensure_read_directory_metadata_safe(folder_path, &metadata, project_root)?;
            read_documents_from_folder(
                folder_path,
                project_path,
                project_root,
                hierarchy_identities,
                documents,
                document_paths,
                repair_mode,
            )
        }
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Reads all documents from a folder recursively
#[allow(clippy::too_many_arguments)]
fn read_documents_from_folder(
    folder_path: &Path,
    project_path: &Path,
    project_root: &Path,
    hierarchy_identities: &HashMap<String, HierarchyDocumentIdentity>,
    documents: &mut HashMap<String, Document>,
    document_paths: &mut HashSet<String>,
    repair_mode: RepairMode,
) -> Result<(), ChiknError> {
    let metadata = fs::symlink_metadata(folder_path)?;
    ensure_read_directory_metadata_safe(folder_path, &metadata, project_root)?;

    // Iterate through all entries in the folder
    for entry in fs::read_dir(folder_path)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;

        if metadata.file_type().is_symlink() {
            return Err(ChiknError::InvalidFormat(format!(
                "Project document path is a symlink: {}",
                path.display()
            )));
        } else if metadata.is_file() {
            // Process .md files
            if let Some(extension) = path.extension() {
                if extension == DOCUMENT_EXTENSION {
                    let doc = read_document_with_root(
                        &path,
                        project_path,
                        project_root,
                        hierarchy_identities,
                        repair_mode,
                    )?;
                    let normalized_path = normalized_relative_document_path(&doc.path)?;
                    if !document_paths.insert(normalized_path) {
                        return Err(ChiknError::InvalidFormat(format!(
                            "Duplicate document path: {}",
                            doc.path
                        )));
                    }
                    if documents.contains_key(&doc.id) {
                        return Err(ChiknError::InvalidFormat(format!(
                            "Duplicate document id: {}",
                            doc.id
                        )));
                    }
                    documents.insert(doc.id.clone(), doc);
                }
            }
        } else if metadata.is_dir() {
            // Recursively process subdirectories
            read_documents_from_folder(
                &path,
                project_path,
                project_root,
                hierarchy_identities,
                documents,
                document_paths,
                repair_mode,
            )?;
        }
    }

    Ok(())
}

fn validate_hierarchy_documents_match_loaded_documents(
    hierarchy: &[TreeNode],
    documents: &HashMap<String, Document>,
) -> Result<(), ChiknError> {
    let mut documents_by_path = HashMap::new();
    for document in documents.values() {
        let normalized_path = normalized_relative_document_path(&document.path)?;
        documents_by_path.insert(normalized_path, document.id.as_str());
    }
    validate_hierarchy_documents_match_loaded_documents_inner(
        hierarchy,
        documents,
        &documents_by_path,
    )
}

fn validate_hierarchy_documents_match_loaded_documents_inner(
    hierarchy: &[TreeNode],
    documents: &HashMap<String, Document>,
    documents_by_path: &HashMap<String, &str>,
) -> Result<(), ChiknError> {
    for node in hierarchy {
        match node {
            TreeNode::Document { id, path, .. } => {
                let normalized_path = normalized_relative_document_path(path)?;
                if let Some(document) = documents.get(id) {
                    let loaded_path = normalized_relative_document_path(&document.path)?;
                    if loaded_path != normalized_path {
                        return Err(ChiknError::InvalidFormat(format!(
                            "Hierarchy document {id} points at {path}, but loaded document path is {}",
                            document.path
                        )));
                    }
                } else if let Some(actual_id) = documents_by_path.get(&normalized_path) {
                    return Err(ChiknError::InvalidFormat(format!(
                        "Hierarchy document {id} points at {path}, but that path loaded as document {actual_id}"
                    )));
                }
            }
            TreeNode::Folder { children, .. } => {
                validate_hierarchy_documents_match_loaded_documents_inner(
                    children,
                    documents,
                    documents_by_path,
                )?;
            }
        }
    }
    Ok(())
}

/// Reads a single document (content + metadata)
#[cfg(test)]
fn read_document(content_path: &Path, project_path: &Path) -> Result<Document, ChiknError> {
    let project_root = canonical_project_root(project_path)?;
    read_document_with_root(
        content_path,
        project_path,
        &project_root,
        &HashMap::new(),
        RepairMode::SelfHeal,
    )
}

fn read_document_with_root(
    content_path: &Path,
    _project_path: &Path,
    project_root: &Path,
    hierarchy_identities: &HashMap<String, HierarchyDocumentIdentity>,
    repair_mode: RepairMode,
) -> Result<Document, ChiknError> {
    ensure_existing_read_file_safe(content_path, project_root, "document file")?;

    // Read content (.md file)
    let content = fs::read_to_string(content_path)?;

    // Get filename stem (used as fallback if metadata missing)
    let file_stem = content_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| {
            ChiknError::InvalidFormat(format!(
                "Invalid document filename: {}",
                content_path.display()
            ))
        })?;

    // Read metadata (.meta file) if it exists
    let folder_path = content_path.parent().ok_or_else(|| {
        ChiknError::InvalidFormat(format!(
            "Document has no parent folder: {}",
            content_path.display()
        ))
    })?;

    let meta_path = get_document_meta_path(folder_path, file_stem);
    let canonical_content_path = content_path.canonicalize()?;
    let relative_path = canonical_content_path
        .strip_prefix(project_root)
        .map_err(|_| {
            ChiknError::InvalidFormat(format!(
                "Document path not within project: {}",
                content_path.display()
            ))
        })?
        .to_string_lossy()
        .replace('\\', "/")
        .to_string();
    let fallback_identity = hierarchy_identities.get(&relative_path);
    let metadata = read_document_metadata_or_default(
        &meta_path,
        project_root,
        fallback_identity,
        repair_mode,
    )?;

    // Use display name from metadata if available, otherwise use filename
    let display_name = metadata.name.unwrap_or_else(|| file_stem.to_string());

    Ok(Document {
        id: metadata.id,
        name: display_name,
        path: relative_path,
        content,
        parent_id: metadata.parent_id,
        created: metadata.created,
        modified: metadata.modified,
        synopsis: metadata.synopsis,
        label: metadata.label,
        status: metadata.status,
        keywords: metadata.keywords,
        links: metadata.links,
        include_in_compile: metadata.include_in_compile.as_deref() != Some("No"),
        word_count_target: metadata.word_count_target,
        compile_order: metadata.compile_order,
        comments: metadata.comments,
        fields: metadata.fields,
    })
}

fn read_document_metadata_or_default(
    meta_path: &Path,
    project_root: &Path,
    fallback_identity: Option<&HierarchyDocumentIdentity>,
    _repair_mode: RepairMode,
) -> Result<DocumentMetadata, ChiknError> {
    if !ensure_optional_read_file_safe(meta_path, project_root, "document metadata")? {
        return Ok(default_document_metadata(fallback_identity));
    }

    let meta_content = fs::read_to_string(meta_path)?;
    match serde_yaml::from_str::<DocumentMetadata>(&meta_content) {
        Ok(mut metadata) => {
            lift_legacy_novelist_keys(&mut metadata);
            Ok(metadata)
        }
        Err(e) => {
            eprintln!(
                "Corrupt document metadata treated as missing without changing the file: {} ({})",
                meta_path.display(),
                e
            );
            Ok(default_document_metadata(fallback_identity))
        }
    }
}

fn default_document_metadata(
    fallback_identity: Option<&HierarchyDocumentIdentity>,
) -> DocumentMetadata {
    DocumentMetadata {
        id: fallback_identity
            .map(|identity| identity.id.clone())
            .unwrap_or_else(generate_id),
        name: fallback_identity.map(|identity| identity.name.clone()),
        created: current_timestamp(),
        modified: current_timestamp(),
        parent_id: None,
        label: None,
        status: None,
        keywords: None,
        synopsis: None,
        section_type: None,
        include_in_compile: None,
        scrivener_uuid: None,
        links: None,
        word_count_target: 0,
        compile_order: 0,
        comments: Vec::new(),
        fields: std::collections::BTreeMap::new(),
        extra: Default::default(),
    }
}

fn validate_relative_document_path(document_path: &str) -> Result<(), ChiknError> {
    let path = Path::new(document_path);
    let mut has_normal_component = false;

    for component in path.components() {
        match component {
            Component::Normal(_) => has_normal_component = true,
            Component::CurDir => {
                return Err(invalid_document_path(
                    document_path,
                    "current-directory components are not allowed",
                ));
            }
            Component::ParentDir => {
                return Err(invalid_document_path(
                    document_path,
                    "parent-directory components are not allowed",
                ));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(invalid_document_path(
                    document_path,
                    "absolute paths are not allowed",
                ));
            }
        }
    }

    if !has_normal_component {
        return Err(invalid_document_path(
            document_path,
            "path must contain a file name",
        ));
    }

    Ok(())
}

pub(crate) fn normalized_relative_document_path(document_path: &str) -> Result<String, ChiknError> {
    let slash_path = document_path.replace('\\', "/");
    validate_relative_document_path(&slash_path)?;
    let components: Vec<String> = Path::new(&slash_path)
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();

    Ok(components.join("/"))
}

fn invalid_document_path(document_path: &str, reason: &str) -> ChiknError {
    ChiknError::InvalidFormat(format!(
        "Document path must be relative and within project ({}): {}",
        reason, document_path
    ))
}

fn canonical_project_root(project_path: &Path) -> Result<PathBuf, ChiknError> {
    let project_root = project_path.canonicalize()?;
    if !project_root.is_dir() {
        return Err(ChiknError::InvalidFormat(format!(
            "Project path is not a directory: {}",
            project_path.display()
        )));
    }
    Ok(project_root)
}

fn ensure_read_directory_metadata_safe(
    path: &Path,
    metadata: &fs::Metadata,
    project_root: &Path,
) -> Result<(), ChiknError> {
    if metadata.file_type().is_symlink() {
        return Err(ChiknError::InvalidFormat(format!(
            "Project directory is a symlink: {}",
            path.display()
        )));
    }
    if !metadata.is_dir() {
        return Err(ChiknError::InvalidFormat(format!(
            "Project path is not a directory: {}",
            path.display()
        )));
    }
    ensure_canonical_read_path_within_project(path, project_root)
}

fn ensure_existing_read_file_safe(
    path: &Path,
    project_root: &Path,
    kind: &str,
) -> Result<(), ChiknError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err(ChiknError::InvalidFormat(format!(
                    "Project {} is a symlink: {}",
                    kind,
                    path.display()
                )));
            }
            if !metadata.is_file() {
                return Err(ChiknError::InvalidFormat(format!(
                    "Project {} is not a file: {}",
                    kind,
                    path.display()
                )));
            }
            ensure_canonical_read_path_within_project(path, project_root)
        }
        Err(e) => Err(e.into()),
    }
}

fn ensure_optional_read_file_safe(
    path: &Path,
    project_root: &Path,
    kind: &str,
) -> Result<bool, ChiknError> {
    match fs::symlink_metadata(path) {
        Ok(_) => {
            ensure_existing_read_file_safe(path, project_root, kind)?;
            Ok(true)
        }
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}

fn ensure_canonical_read_path_within_project(
    path: &Path,
    project_root: &Path,
) -> Result<(), ChiknError> {
    let canonical_path = path.canonicalize()?;
    if !canonical_path.starts_with(project_root) {
        return Err(ChiknError::InvalidFormat(format!(
            "Project path escapes project root: {}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::project::format::{
        MANUSCRIPT_FOLDER, PROJECT_FILE, RESEARCH_FOLDER, SETTINGS_FOLDER, TEMPLATES_FOLDER,
    };
    use std::fs;
    use tempfile::TempDir;

    /// Helper to create a test .chikn project
    fn create_test_project() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("TestProject.chikn");

        // Create directory structure
        fs::create_dir(&project_path).unwrap();
        fs::create_dir(project_path.join(MANUSCRIPT_FOLDER)).unwrap();
        fs::create_dir(project_path.join(RESEARCH_FOLDER)).unwrap();
        fs::create_dir(project_path.join(TEMPLATES_FOLDER)).unwrap();
        fs::create_dir(project_path.join(SETTINGS_FOLDER)).unwrap();

        // Create project.yaml
        let project_yaml = format!(
            r#"id: "{}"
name: "Test Project"
created: "2025-01-01T00:00:00Z"
modified: "2025-01-01T00:00:00Z"
hierarchy:
  - type: Document
    id: "doc1"
    name: "Chapter 1"
    path: "manuscript/chapter-01.md"
"#,
            generate_id()
        );
        fs::write(project_path.join(PROJECT_FILE), project_yaml).unwrap();

        // Create a test document
        let doc_path = project_path.join(MANUSCRIPT_FOLDER).join("chapter-01.md");
        fs::write(&doc_path, "# Chapter 1\n\nOnce upon a time...").unwrap();

        // Create metadata file
        let meta_yaml = r#"id: "doc1"
created: "2025-01-01T00:00:00Z"
modified: "2025-01-01T00:00:00Z"
"#
        .to_string();
        fs::write(
            project_path.join(MANUSCRIPT_FOLDER).join("chapter-01.meta"),
            meta_yaml,
        )
        .unwrap();

        (temp_dir, project_path)
    }

    fn count_corrupt_meta_quarantines(project_path: &Path) -> usize {
        fs::read_dir(project_path.join(MANUSCRIPT_FOLDER))
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with("chapter-01.meta.corrupt-"))
            .count()
    }

    #[test]
    fn test_read_project_success() {
        let (_temp, project_path) = create_test_project();
        let result = read_project(&project_path);

        assert!(result.is_ok());
        let project = result.unwrap();
        assert_eq!(project.name, "Test Project");
        // Hierarchy includes the original doc + auto-added Manuscript/Research folders
        assert!(!project.hierarchy.is_empty());
        assert_eq!(project.documents.len(), 1);
    }

    #[test]
    fn test_read_project_metadata() {
        let (_temp, project_path) = create_test_project();
        let result = read_project_metadata(&project_path);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.name, "Test Project");
    }

    #[test]
    fn test_read_document() {
        let (_temp, project_path) = create_test_project();
        let doc_path = project_path.join(MANUSCRIPT_FOLDER).join("chapter-01.md");

        let result = read_document(&doc_path, &project_path);
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert_eq!(doc.name, "chapter-01");
        assert!(doc.content.contains("Once upon a time"));
        assert!(doc.path.starts_with("manuscript/"));
    }

    #[test]
    fn test_read_all_documents() {
        let (_temp, project_path) = create_test_project();
        let result = read_all_documents(&project_path, &HashMap::new(), RepairMode::SelfHeal);

        assert!(result.is_ok());
        let documents = result.unwrap();
        assert_eq!(documents.len(), 1);
        assert!(documents.contains_key("doc1"));
    }

    #[test]
    fn test_read_project_rejects_parent_dir_hierarchy_path() {
        let (_temp, project_path) = create_test_project();
        let project_yaml = format!(
            r#"id: "{}"
name: "Bad Path"
created: "2025-01-01T00:00:00Z"
modified: "2025-01-01T00:00:00Z"
hierarchy:
  - type: Document
    id: "doc1"
    name: "Chapter 1"
    path: "../outside.md"
"#,
            generate_id()
        );
        fs::write(project_path.join(PROJECT_FILE), project_yaml).unwrap();

        let result = read_project(&project_path);

        assert!(matches!(result, Err(ChiknError::InvalidFormat(_))));
    }

    #[test]
    fn test_read_project_rejects_duplicate_document_ids() {
        let (_temp, project_path) = create_test_project();
        fs::write(
            project_path.join(RESEARCH_FOLDER).join("duplicate.md"),
            "duplicate body",
        )
        .unwrap();
        fs::write(
            project_path.join(RESEARCH_FOLDER).join("duplicate.meta"),
            r#"id: "doc1"
created: "2025-01-01T00:00:00Z"
modified: "2025-01-01T00:00:00Z"
"#,
        )
        .unwrap();

        let result = read_project(&project_path);

        assert!(matches!(result, Err(ChiknError::InvalidFormat(_))));
    }

    #[test]
    fn test_read_project_rejects_duplicate_hierarchy_document_paths() {
        let (_temp, project_path) = create_test_project();
        let project_yaml = format!(
            r#"id: "{}"
name: "Duplicate Paths"
created: "2025-01-01T00:00:00Z"
modified: "2025-01-01T00:00:00Z"
hierarchy:
  - type: Document
    id: "doc1"
    name: "Chapter 1"
    path: "manuscript/chapter-01.md"
  - type: Document
    id: "doc2"
    name: "Chapter 1 Copy"
    path: "manuscript/chapter-01.md"
"#,
            generate_id()
        );
        fs::write(project_path.join(PROJECT_FILE), project_yaml).unwrap();

        let result = read_project(&project_path);

        assert!(matches!(result, Err(ChiknError::InvalidFormat(_))));
    }

    #[test]
    fn test_read_project_rejects_hierarchy_id_path_mismatch() {
        let (_temp, project_path) = create_test_project();
        let project_yaml = format!(
            r#"id: "{}"
name: "ID Mismatch"
created: "2025-01-01T00:00:00Z"
modified: "2025-01-01T00:00:00Z"
hierarchy:
  - type: Document
    id: "wrong-doc-id"
    name: "Chapter 1"
    path: "manuscript/chapter-01.md"
"#,
            generate_id()
        );
        fs::write(project_path.join(PROJECT_FILE), project_yaml).unwrap();

        let result = read_project(&project_path);

        assert!(matches!(result, Err(ChiknError::InvalidFormat(_))));
    }

    #[cfg(unix)]
    #[test]
    fn test_read_project_rejects_symlink_document_file() {
        use std::os::unix::fs as unix_fs;

        let (temp, project_path) = create_test_project();
        let outside = temp.path().join("outside.md");
        fs::write(&outside, "outside content").unwrap();
        unix_fs::symlink(
            &outside,
            project_path.join(MANUSCRIPT_FOLDER).join("linked.md"),
        )
        .unwrap();

        let result = read_project(&project_path);

        assert!(matches!(result, Err(ChiknError::InvalidFormat(_))));
    }

    #[cfg(unix)]
    #[test]
    fn test_read_project_rejects_symlink_document_meta() {
        use std::os::unix::fs as unix_fs;

        let (temp, project_path) = create_test_project();
        let outside = temp.path().join("outside.meta");
        fs::write(
            &outside,
            r#"id: "outside"
created: "2025-01-01T00:00:00Z"
modified: "2025-01-01T00:00:00Z"
"#,
        )
        .unwrap();
        let meta_path = project_path.join(MANUSCRIPT_FOLDER).join("chapter-01.meta");
        fs::remove_file(&meta_path).unwrap();
        unix_fs::symlink(&outside, &meta_path).unwrap();

        let result = read_project(&project_path);

        assert!(matches!(result, Err(ChiknError::InvalidFormat(_))));
    }

    #[cfg(unix)]
    #[test]
    fn test_read_project_rejects_symlink_document_folder() {
        use std::os::unix::fs as unix_fs;

        let (temp, project_path) = create_test_project();
        let outside = temp.path().join("outside-folder");
        fs::create_dir(&outside).unwrap();
        unix_fs::symlink(
            &outside,
            project_path.join(MANUSCRIPT_FOLDER).join("linked-folder"),
        )
        .unwrap();

        let result = read_project(&project_path);

        assert!(matches!(result, Err(ChiknError::InvalidFormat(_))));
    }

    #[cfg(unix)]
    #[test]
    fn test_read_project_rejects_symlink_threads_file() {
        use std::os::unix::fs as unix_fs;

        let (temp, project_path) = create_test_project();
        let outside = temp.path().join("threads.yaml");
        fs::write(&outside, "threads: []\n").unwrap();
        unix_fs::symlink(&outside, project_path.join("threads.yaml")).unwrap();

        let result = read_project(&project_path);

        assert!(matches!(result, Err(ChiknError::InvalidFormat(_))));
    }

    #[test]
    fn test_read_nested_documents() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("NestedTest.chikn");

        // Create project structure
        fs::create_dir(&project_path).unwrap();
        fs::create_dir(project_path.join(MANUSCRIPT_FOLDER)).unwrap();
        fs::create_dir(project_path.join(RESEARCH_FOLDER)).unwrap();
        fs::create_dir(project_path.join(TEMPLATES_FOLDER)).unwrap();
        fs::create_dir(project_path.join(SETTINGS_FOLDER)).unwrap();

        // Create nested folder structure
        let nested_folder = project_path.join(MANUSCRIPT_FOLDER).join("part-one");
        fs::create_dir(&nested_folder).unwrap();

        // Create document in nested folder
        fs::write(
            nested_folder.join("chapter-01.md"),
            "# Nested Chapter\n\nContent in subfolder",
        )
        .unwrap();

        // Create metadata
        let meta_yaml = r#"id: "nested-doc1"
created: "2025-01-01T00:00:00Z"
modified: "2025-01-01T00:00:00Z"
"#;
        fs::write(nested_folder.join("chapter-01.meta"), meta_yaml).unwrap();

        // Create project.yaml
        let project_yaml = format!(
            r#"id: "{}"
name: "Nested Test"
created: "2025-01-01T00:00:00Z"
modified: "2025-01-01T00:00:00Z"
hierarchy: []
"#,
            generate_id()
        );
        fs::write(project_path.join(PROJECT_FILE), project_yaml).unwrap();

        // Read all documents (should find nested)
        let documents =
            read_all_documents(&project_path, &HashMap::new(), RepairMode::SelfHeal).unwrap();

        assert_eq!(documents.len(), 1);
        assert!(documents.contains_key("nested-doc1"));

        let doc = documents.get("nested-doc1").unwrap();
        assert_eq!(doc.name, "chapter-01");
        // Verify relative path
        assert_eq!(doc.path, "manuscript/part-one/chapter-01.md");
    }

    #[test]
    fn test_read_document_relative_path() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("PathTest.chikn");

        // Create structure
        fs::create_dir(&project_path).unwrap();
        let nested = project_path.join("manuscript").join("subfolder");
        fs::create_dir_all(&nested).unwrap();

        // Create document
        let doc_path = nested.join("test.md");
        fs::write(&doc_path, "Test content").unwrap();

        // Create metadata
        fs::write(
            nested.join("test.meta"),
            "id: \"test-id\"\ncreated: \"2025-01-01T00:00:00Z\"\nmodified: \"2025-01-01T00:00:00Z\"\n"
        ).unwrap();

        // Read document
        let doc = read_document(&doc_path, &project_path).unwrap();

        // Verify path is relative, not absolute
        assert!(!doc.path.starts_with("/"));
        assert_eq!(doc.path, "manuscript/subfolder/test.md");
    }

    #[test]
    fn test_repair_keeps_missing_file_reference_in_memory_and_on_disk() {
        let (_temp, project_path) = create_test_project();
        let project_yaml_before = fs::read_to_string(project_path.join(PROJECT_FILE)).unwrap();

        // Delete the document file but leave project.yaml referencing it
        fs::remove_file(project_path.join("manuscript/chapter-01.md")).unwrap();
        fs::remove_file(project_path.join("manuscript/chapter-01.meta")).unwrap();

        // Load should succeed, but missing-file references must not be pruned:
        // the file may be temporarily unavailable and return later.
        let project = read_project(&project_path).unwrap();

        assert_eq!(count_hierarchy_docs(&project.hierarchy), 1);
        assert!(
            collect_hierarchy_paths(&project.hierarchy).contains("manuscript/chapter-01.md"),
            "dangling hierarchy reference should be preserved"
        );
        assert_eq!(project.documents.len(), 0);
        assert_eq!(
            fs::read_to_string(project_path.join(PROJECT_FILE)).unwrap(),
            project_yaml_before,
            "read repair must not rewrite project.yaml"
        );
    }

    #[test]
    fn test_read_project_does_not_persist_missing_file_repair() {
        let (_temp, project_path) = create_test_project();
        let project_file = project_path.join(PROJECT_FILE);
        let original_project_yaml = fs::read_to_string(&project_file).unwrap();

        // Delete the document file but leave project.yaml referencing it.
        fs::remove_file(project_path.join("manuscript/chapter-01.md")).unwrap();
        fs::remove_file(project_path.join("manuscript/chapter-01.meta")).unwrap();

        let project = read_project(&project_path).unwrap();

        assert_eq!(count_hierarchy_docs(&project.hierarchy), 1);
        assert!(
            collect_hierarchy_paths(&project.hierarchy).contains("manuscript/chapter-01.md"),
            "load-time repair must keep the missing document reference in memory"
        );
        assert_eq!(project.documents.len(), 0);
        assert_eq!(
            fs::read_to_string(&project_file).unwrap(),
            original_project_yaml,
            "load-time repair must not rewrite project.yaml"
        );
    }

    #[test]
    fn test_public_read_preserves_corrupt_document_meta_and_keeps_hierarchy_identity() {
        let (_temp, project_path) = create_test_project();
        let project_file = project_path.join(PROJECT_FILE);
        let project_yaml_before = fs::read_to_string(&project_file).unwrap();
        let meta_path = project_path.join("manuscript/chapter-01.meta");
        fs::write(&meta_path, "id: [").unwrap();
        let corrupt_bytes = fs::read(&meta_path).unwrap();

        let project = read_project(&project_path).unwrap();

        let document = project.documents.get("doc1").unwrap();
        assert_eq!(document.id, "doc1");
        assert_eq!(document.name, "Chapter 1");
        assert_eq!(document.path, "manuscript/chapter-01.md");
        assert!(meta_path.exists(), "corrupt meta must stay in place");
        assert_eq!(fs::read(&meta_path).unwrap(), corrupt_bytes);
        assert_eq!(count_corrupt_meta_quarantines(&project_path), 0);
        assert_eq!(
            fs::read_to_string(&project_file).unwrap(),
            project_yaml_before,
            "reading corrupt metadata must not rewrite project.yaml"
        );
    }

    #[test]
    fn test_readonly_read_does_not_quarantine_corrupt_meta() {
        // The Degraded open path must leave the folder byte-identical:
        // corrupt sidecars are treated as missing in memory, never renamed.
        let (_temp, project_path) = create_test_project();
        let meta_path = project_path.join("manuscript/chapter-01.meta");
        fs::write(&meta_path, "id: [").unwrap();
        let corrupt_bytes = fs::read(&meta_path).unwrap();

        let project = read_project_readonly(&project_path).unwrap();

        let document = project.documents.get("doc1").unwrap();
        assert_eq!(document.id, "doc1");
        assert_eq!(document.name, "Chapter 1");
        assert!(meta_path.exists(), "corrupt meta must stay in place");
        assert_eq!(fs::read(&meta_path).unwrap(), corrupt_bytes);
        assert_eq!(count_corrupt_meta_quarantines(&project_path), 0);
    }

    #[test]
    fn test_public_read_does_not_create_missing_folders() {
        let (_temp, project_path) = create_test_project();
        fs::remove_dir(project_path.join(RESEARCH_FOLDER)).unwrap();
        fs::remove_dir(project_path.join(TEMPLATES_FOLDER)).unwrap();
        fs::remove_dir(project_path.join(SETTINGS_FOLDER)).unwrap();

        let project = read_project(&project_path).unwrap();

        assert_eq!(project.documents.len(), 1);
        assert!(
            !project_path.join(RESEARCH_FOLDER).exists(),
            "read-only open must not self-heal folders on disk"
        );
        assert!(!project_path.join(TEMPLATES_FOLDER).exists());
        assert!(!project_path.join(SETTINGS_FOLDER).exists());

        // Explicitly authorized Full-project open retains benign self-heal.
        let token = super::super::fidelity::acquire_write_token(&project_path).unwrap();
        token
            .with_write_permit(&project_path, |permit| {
                read_project_with_repair(&project_path, permit).map(|_| ())
            })
            .unwrap();
        assert!(project_path.join(RESEARCH_FOLDER).exists());
    }

    #[test]
    fn test_missing_document_meta_uses_hierarchy_identity() {
        let (_temp, project_path) = create_test_project();
        fs::remove_file(project_path.join("manuscript/chapter-01.meta")).unwrap();

        let project = read_project(&project_path).unwrap();

        let document = project.documents.get("doc1").unwrap();
        assert_eq!(document.id, "doc1");
        assert_eq!(document.name, "Chapter 1");
        assert_eq!(document.path, "manuscript/chapter-01.md");
    }

    #[test]
    fn test_read_characters_and_locations_folders() {
        // Novelist convention: docs under characters/ and locations/ should be
        // picked up as regular Documents alongside manuscript/research.
        let (_temp, project_path) = create_test_project();

        // Add an entity in characters/
        let chars_dir = project_path.join("characters");
        fs::create_dir(&chars_dir).unwrap();
        fs::write(
            chars_dir.join("sarah-bennett.md"),
            "# Sarah Bennett\n\nProtagonist notes.",
        )
        .unwrap();
        fs::write(
            chars_dir.join("sarah-bennett.meta"),
            "id: char-sarah\ncreated: 2026-04-30T00:00:00Z\nmodified: 2026-04-30T00:00:00Z\n",
        )
        .unwrap();

        // Add a location
        let locs_dir = project_path.join("locations");
        fs::create_dir(&locs_dir).unwrap();
        fs::write(locs_dir.join("motel.md"), "# Motel Room 12").unwrap();
        fs::write(
            locs_dir.join("motel.meta"),
            "id: loc-motel\ncreated: 2026-04-30T00:00:00Z\nmodified: 2026-04-30T00:00:00Z\n",
        )
        .unwrap();

        let project = read_project(&project_path).expect("read project");
        assert!(project.documents.contains_key("char-sarah"));
        assert!(project.documents.contains_key("loc-motel"));
        assert!(project.documents["char-sarah"]
            .path
            .starts_with("characters/"));
        assert!(project.documents["loc-motel"]
            .path
            .starts_with("locations/"));
    }

    #[test]
    fn test_threads_round_trip_via_yaml() {
        // Write threads through the writer, read them back through the reader.
        let (_temp, project_path) = create_test_project();
        let mut project = read_project(&project_path).expect("read");
        project.threads = vec![
            crate::models::Thread {
                id: "main-plot".into(),
                name: "Main Plot".into(),
                color: Some("#3b82f6".into()),
                description: Some("Sarah uncovers the truth.".into()),
                extra: Default::default(),
            },
            crate::models::Thread {
                id: "romance".into(),
                name: "Sarah & Marcus".into(),
                color: Some("#ef4444".into()),
                description: None,
                extra: Default::default(),
            },
        ];
        let token = super::super::fidelity::acquire_write_token(&project_path).expect("token");
        token
            .with_write_permit(&project_path, |permit| {
                super::super::writer::write_project(&mut project, permit)
            })
            .expect("write");

        let reread = read_project(&project_path).expect("re-read");
        assert_eq!(reread.threads.len(), 2);
        assert_eq!(reread.threads[0].id, "main-plot");
        assert_eq!(reread.threads[0].color.as_deref(), Some("#3b82f6"));
        assert_eq!(reread.threads[1].name, "Sarah & Marcus");
        assert!(reread.threads[1].description.is_none());
    }

    #[test]
    fn test_threads_missing_file_yields_empty() {
        let (_temp, project_path) = create_test_project();
        let project = read_project(&project_path).expect("read");
        assert!(project.threads.is_empty());
    }

    #[test]
    fn test_threads_corrupt_file_fails_load() {
        let (_temp, project_path) = create_test_project();
        fs::write(project_path.join("threads.yaml"), "threads: [").unwrap();

        let result = read_project(&project_path);
        assert!(
            result.is_err(),
            "corrupt threads.yaml must not silently default to empty"
        );
    }

    #[test]
    fn test_include_in_compile_accepts_bool_or_string() {
        // Bool true → "Yes", bool false → "No", string passes through.
        // Covers `.meta` files written by the Windows C# writer (bool) and
        // the Rust canonical writer ("Yes"/"No").
        let cases: &[(&str, Option<&str>)] = &[
            ("include_in_compile: true", Some("Yes")),
            ("include_in_compile: false", Some("No")),
            ("include_in_compile: \"Yes\"", Some("Yes")),
            ("include_in_compile: \"No\"", Some("No")),
        ];
        for (snippet, expected) in cases {
            let yaml = format!("id: x\ncreated: \"2025\"\nmodified: \"2025\"\n{snippet}\n");
            let meta: DocumentMetadata = serde_yaml::from_str(&yaml)
                .unwrap_or_else(|e| panic!("parse failed for {snippet}: {e}"));
            assert_eq!(
                meta.include_in_compile.as_deref(),
                *expected,
                "snippet {snippet}"
            );
        }

        // Missing key stays None — "Yes by default" is a downstream concern.
        let meta: DocumentMetadata =
            serde_yaml::from_str("id: x\ncreated: \"2025\"\nmodified: \"2025\"\n").unwrap();
        assert!(meta.include_in_compile.is_none());
    }

    #[test]
    fn test_repair_adds_orphan_to_hierarchy() {
        let (_temp, project_path) = create_test_project();

        // Add a new .md file that's NOT in project.yaml
        fs::write(
            project_path.join("manuscript/orphan.md"),
            "# Orphan\n\nThis file was restored but not in hierarchy.",
        )
        .unwrap();
        fs::write(
            project_path.join("manuscript/orphan.meta"),
            "id: orphan-1\ncreated: \"2025-01-01T00:00:00Z\"\nmodified: \"2025-01-01T00:00:00Z\"\n",
        )
        .unwrap();

        let project = read_project(&project_path).unwrap();

        // Should have 2 docs: original + orphan added to hierarchy
        assert_eq!(project.documents.len(), 2);
        assert_eq!(count_hierarchy_docs(&project.hierarchy), 2);

        // The orphan should be findable
        assert!(project.documents.contains_key("orphan-1"));
    }
}
