//! # Project Fidelity Probe and Write Capability
//!
//! The write-guard: the engine must never save over a project it cannot
//! fully read (INVARIANTS.md I5/I6, PLAN_TRUST_FOUNDATIONS.md Slice 1).
//!
//! Three pieces:
//!
//! 1. [`probe_project_fidelity`] — a side-effect-free preflight that
//!    classifies a project as [`Fidelity::Full`] (safe to write) or
//!    [`Fidelity::Degraded`] (anything the current engine cannot fully
//!    resolve, or that load-time self-heal would mutate in a
//!    content-threatening way). The probe performs no folder creation, no
//!    sidecar quarantine renames, no writes of any kind.
//! 2. [`WriteToken`] — a non-`Clone`, engine-issued session capability bound
//!    to a canonical project root and stamped with a write epoch.
//! 3. [`WritePermit`] — a short-lived operation capability issued only after
//!    a cached token re-probes current fidelity. Mutating APIs require the
//!    permit, so an externally degraded project cannot reuse yesterday's
//!    `Full` classification.
//!
//! A token is only issued when the probe returns `Full`. A freshly created
//! project probes `Full` by construction, so the project-creation path
//! acquires its token the same way.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use super::format::{
    CHARACTERS_FOLDER, DOCUMENT_EXTENSION, FORMAT_VERSION, LOCATIONS_FOLDER, MANUSCRIPT_FOLDER,
    PROJECT_FILE, RESEARCH_FOLDER,
};
use super::reader::{DocumentMetadata, ProjectMetadata};
use crate::models::TreeNode;
use crate::utils::error::ChiknError;

/// Folders whose `.md` files the reader actually loads. A hierarchy entry
/// pointing anywhere else can never resolve to loaded content.
const LOADED_DOCUMENT_ROOTS: &[&str] = &[
    MANUSCRIPT_FOLDER,
    RESEARCH_FOLDER,
    CHARACTERS_FOLDER,
    LOCATIONS_FOLDER,
];

/// Binary asset types a binder may reference (imported research: documents,
/// images, audio, video). The engine treats these as opaque — it never loads
/// or writes their content (`writer` refuses non-.md document writes) — so
/// their presence is fidelity-neutral as long as the file exists.
const OPAQUE_ASSET_EXTENSIONS: &[&str] = &[
    "pdf", "png", "jpg", "jpeg", "gif", "webp", "tif", "tiff", "svg", "mp3", "wav", "m4a", "ogg",
    "flac", "mp4", "mov", "webm",
];

/// Result of a fidelity probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fidelity {
    /// Every hierarchy document resolves, no content-threatening repair
    /// condition exists, and the format version is supported. Safe to
    /// issue a write capability.
    Full,
    /// The project cannot be fully read by this engine, or opening it
    /// normally would trigger content-threatening self-heal. It must only
    /// be opened read-only.
    Degraded { reasons: Vec<DegradedReason> },
}

/// Why a project probed Degraded. `Display` renders plain English suitable
/// for direct surfacing to a writer (no jargon).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DegradedReason {
    /// A hierarchy document path does not end in `.md` — this engine
    /// cannot load its content (e.g. April-era `.html` projects).
    LegacyDocumentPath { path: String },
    /// A hierarchy document can never resolve to loaded content: the file
    /// is missing, unreadable, outside the folders the engine reads, or
    /// has bytes that yield no content.
    UnresolvedDocument { path: String, detail: String },
    /// A document sidecar (`.meta`) exists but cannot be parsed. It remains
    /// untouched on disk and the project can only be read side-effect-free.
    CorruptSidecar { path: String },
    /// A document file exists on disk but is not referenced by the
    /// hierarchy. A normal load would adopt it and the next save would
    /// persist the altered hierarchy.
    OrphanDocument { path: String },
    /// Duplicate or conflicting document identity (duplicate ids or paths,
    /// or a sidecar id that disagrees with the hierarchy).
    ConflictingIdentity { detail: String },
    /// `project.yaml` declares a format version newer than this engine
    /// writes — saving would silently downgrade the project.
    NewerFormatVersion { found: String },
    /// `project.yaml` declares a format version this engine cannot
    /// interpret at all.
    UnsupportedFormatVersion { found: String },
}

impl std::fmt::Display for DegradedReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DegradedReason::LegacyDocumentPath { path } => write!(
                f,
                "'{path}' was written by an older version of the app and cannot be opened for editing"
            ),
            DegradedReason::UnresolvedDocument { path, detail } => {
                write!(f, "'{path}' is listed in the project but cannot be loaded ({detail})")
            }
            DegradedReason::CorruptSidecar { path } => {
                write!(f, "the notes file '{path}' is damaged")
            }
            DegradedReason::OrphanDocument { path } => {
                write!(f, "'{path}' exists on disk but is not listed in the project")
            }
            DegradedReason::ConflictingIdentity { detail } => {
                write!(f, "two parts of the project disagree about a document's identity ({detail})")
            }
            DegradedReason::NewerFormatVersion { found } => write!(
                f,
                "this project was saved by a newer version of the app (format {found})"
            ),
            DegradedReason::UnsupportedFormatVersion { found } => write!(
                f,
                "this project declares a format ('{found}') this version of the app does not understand"
            ),
        }
    }
}

/// Render a reason list as one plain-English sentence fragment.
pub fn describe_reasons(reasons: &[DegradedReason]) -> String {
    reasons
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

// ── Write epoch registry ─────────────────────────────────────────────────────

fn epochs() -> &'static Mutex<HashMap<PathBuf, u64>> {
    static EPOCHS: OnceLock<Mutex<HashMap<PathBuf, u64>>> = OnceLock::new();
    EPOCHS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn current_epoch(canonical_root: &Path) -> u64 {
    epochs()
        .lock()
        .map(|map| map.get(canonical_root).copied().unwrap_or(0))
        .unwrap_or(0)
}

/// Bump the write epoch for a project root. Every outstanding token for
/// that root becomes stale; callers must re-probe and re-acquire.
pub(crate) fn bump_write_epoch(canonical_root: &Path) {
    if let Ok(mut map) = epochs().lock() {
        *map.entry(canonical_root.to_path_buf()).or_insert(0) += 1;
    }
}

// ── WriteToken ───────────────────────────────────────────────────────────────

/// Engine-issued session capability. Non-`Clone` and non-constructible
/// outside the engine: the only way to obtain one is
/// [`acquire_write_token`], which probes fidelity first. A token cannot
/// directly authorize a mutation; callers must open a fresh
/// [`WritePermit`] for each logical operation.
#[derive(Debug)]
pub struct WriteToken {
    /// Canonical (symlink-resolved) project root this token authorizes.
    root: PathBuf,
    /// Write epoch at issue time. Tree-replacing operations bump the
    /// project's epoch, making earlier tokens stale.
    epoch: u64,
}

impl WriteToken {
    /// The canonical project root this token was issued for.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// True when a tree-replacing operation has invalidated this token.
    pub fn is_stale(&self) -> bool {
        current_epoch(&self.root) != self.epoch
    }

    /// Run one logical write operation after re-checking current fidelity.
    ///
    /// The permit is scoped to the closure so application code cannot cache
    /// it as session state. Nested steps reuse the same permit and perform
    /// cheap root/epoch checks rather than re-probing an intentionally
    /// intermediate tree (for example halfway through folder deletion).
    pub fn with_write_permit<T>(
        &self,
        project_path: &Path,
        operation: impl FnOnce(&WritePermit<'_>) -> Result<T, ChiknError>,
    ) -> Result<T, ChiknError> {
        self.ensure_valid_root(project_path)?;
        self.ensure_epoch_fresh()?;

        match probe_project_fidelity(&self.root)? {
            Fidelity::Full => {}
            Fidelity::Degraded { reasons } => {
                return Err(ChiknError::ReadOnly(describe_reasons(&reasons)));
            }
        }

        // Catch an in-process tree replacement that raced the probe.
        self.ensure_epoch_fresh()?;
        let permit = WritePermit { token: self };
        operation(&permit)
    }

    fn ensure_epoch_fresh(&self) -> Result<(), ChiknError> {
        if self.is_stale() {
            return Err(ChiknError::ReadOnly(format!(
                "the project at {} changed on disk after this session started; reopen it to continue",
                self.root.display()
            )));
        }
        Ok(())
    }

    fn ensure_valid_root(&self, project_path: &Path) -> Result<(), ChiknError> {
        let canonical = project_path.canonicalize().map_err(|e| {
            ChiknError::ReadOnly(format!(
                "cannot resolve project path {}: {e}",
                project_path.display()
            ))
        })?;
        if canonical != self.root {
            return Err(ChiknError::ReadOnly(format!(
                "write access was granted for {}, not {}",
                self.root.display(),
                canonical.display()
            )));
        }
        Ok(())
    }
}

/// Fresh, operation-scoped write authority.
///
/// Only [`WriteToken::with_write_permit`] can construct this type. Mutating
/// engine APIs accept a permit rather than a session token, which makes a
/// current fidelity check a structural precondition of every logical write.
#[derive(Debug)]
pub struct WritePermit<'token> {
    token: &'token WriteToken,
}

impl WritePermit<'_> {
    /// Canonical project root authorized for this operation.
    pub fn root(&self) -> &Path {
        self.token.root()
    }

    /// Refuse unless the operation still targets its authorized root and no
    /// in-process tree replacement has invalidated the underlying token.
    pub(crate) fn ensure_valid_for(&self, project_path: &Path) -> Result<(), ChiknError> {
        self.token.ensure_valid_root(project_path)?;
        self.ensure_fresh()
    }

    /// Cheap nested-step check. Fidelity was probed once when the operation
    /// permit was issued; repeating it here would reject valid intermediate
    /// states inside composite mutations.
    pub(crate) fn ensure_fresh(&self) -> Result<(), ChiknError> {
        self.token.ensure_epoch_fresh()
    }

    /// Re-probe at a deliberate sub-boundary that has not changed project
    /// content (for example after a network fetch, before a merge).
    pub(crate) fn revalidate_fidelity(&self) -> Result<(), ChiknError> {
        self.ensure_fresh()?;
        match probe_project_fidelity(self.root())? {
            Fidelity::Full => {}
            Fidelity::Degraded { reasons } => {
                return Err(ChiknError::ReadOnly(describe_reasons(&reasons)));
            }
        }
        self.ensure_fresh()
    }

    /// Arm the epoch bump at a tree-replacing operation's point of no
    /// return — immediately before its first ref, HEAD, or working-tree
    /// mutation. The bump fires when the returned guard drops, on success
    /// *and* on error, so a failure after the first mutation still leaves
    /// every outstanding token stale (refused until a re-probe). Failures
    /// before the guard is armed bump nothing.
    pub(crate) fn arm_epoch_bump(&self) -> EpochBumpGuard {
        EpochBumpGuard {
            root: self.root().to_path_buf(),
        }
    }
}

/// Drop-scope epoch bump for tree-replacing operations. See
/// [`WritePermit::arm_epoch_bump`]. The permit that armed it stays valid
/// for the operation's own nested steps; the bump lands only when the
/// operation scope exits.
#[derive(Debug)]
pub(crate) struct EpochBumpGuard {
    root: PathBuf,
}

impl Drop for EpochBumpGuard {
    fn drop(&mut self) {
        bump_write_epoch(&self.root);
    }
}

/// Probe fidelity and issue a write capability when (and only when) the
/// project probes [`Fidelity::Full`]. Degraded projects yield
/// [`ChiknError::ReadOnly`] carrying plain-English reasons.
pub fn acquire_write_token(project_path: &Path) -> Result<WriteToken, ChiknError> {
    match probe_project_fidelity(project_path)? {
        Fidelity::Full => {
            let root = project_path.canonicalize()?;
            let epoch = current_epoch(&root);
            Ok(WriteToken { root, epoch })
        }
        Fidelity::Degraded { reasons } => Err(ChiknError::ReadOnly(describe_reasons(&reasons))),
    }
}

// ── Probe ────────────────────────────────────────────────────────────────────

/// Side-effect-free preflight classification of a project.
///
/// Pure read-only scan: no folder creation, no quarantine renames, no
/// writes of any kind. Returns `Err` only for projects that cannot be
/// opened at all (missing root, no `project.yaml`, unparsable
/// `project.yaml`, unsafe paths) — conditions where the normal reader
/// fails identically and nothing can be written anyway.
///
/// Degraded reasons (see [`DegradedReason`]):
/// - hierarchy document paths that are not `.md`;
/// - hierarchy documents that can never resolve to loaded content
///   (missing, unreadable, outside the loaded folders, or nonzero bytes
///   yielding empty content — zero-byte files are VALID);
/// - content-threatening or incomplete-read conditions: corrupt sidecars,
///   orphan documents (read adopts them only in memory), conflicting
///   identities;
/// - a `format_version` newer than, or unintelligible to, this engine.
///
/// Missing standard folders are NOT Degraded: recreating an empty
/// `research/` touches no content and remains normal self-heal. An absent
/// `format_version` on an otherwise-modern project is also NOT Degraded.
pub fn probe_project_fidelity(project_path: &Path) -> Result<Fidelity, ChiknError> {
    validate_probe_root(project_path)?;

    let content = fs::read_to_string(project_path.join(PROJECT_FILE)).map_err(ChiknError::Io)?;
    let metadata: ProjectMetadata =
        serde_yaml::from_str(&content).map_err(ChiknError::Serialization)?;

    let mut reasons = Vec::new();

    check_format_version(metadata.format_version.as_deref(), &mut reasons);

    let hierarchy_docs = collect_probe_hierarchy_documents(&metadata.hierarchy, &mut reasons)?;

    // Resolution check for every hierarchy document.
    for doc in &hierarchy_docs {
        check_hierarchy_document_resolves(project_path, doc, &mut reasons);
    }

    // On-disk scan of the folders the reader loads: corrupt sidecars,
    // orphans, identity conflicts.
    scan_loaded_documents(project_path, &hierarchy_docs, &mut reasons)?;

    if reasons.is_empty() {
        Ok(Fidelity::Full)
    } else {
        Ok(Fidelity::Degraded { reasons })
    }
}

/// Same non-recoverable root checks as the reader (`validate_project_root`),
/// performed read-only.
fn validate_probe_root(path: &Path) -> Result<(), ChiknError> {
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
    let project_file = path.join(PROJECT_FILE);
    if !project_file.exists() {
        return Err(ChiknError::InvalidFormat(format!(
            "Missing required file: {PROJECT_FILE}"
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

fn parse_version(version: &str) -> Option<Vec<u64>> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.is_empty() {
        return None;
    }
    parts
        .iter()
        .map(|p| p.parse::<u64>().ok())
        .collect::<Option<Vec<u64>>>()
}

fn check_format_version(found: Option<&str>, reasons: &mut Vec<DegradedReason>) {
    // Absent format_version is fine: projects written before v1.2 locked
    // the format carry no marker, and legacy damage (if any) is caught by
    // the document checks, not the version marker.
    let Some(found) = found else { return };
    let Some(found_parts) = parse_version(found) else {
        reasons.push(DegradedReason::UnsupportedFormatVersion {
            found: found.to_string(),
        });
        return;
    };
    let engine_parts =
        parse_version(FORMAT_VERSION).expect("engine FORMAT_VERSION must be numeric");
    if found_parts > engine_parts {
        reasons.push(DegradedReason::NewerFormatVersion {
            found: found.to_string(),
        });
    }
}

struct ProbeHierarchyDoc {
    id: String,
    /// Normalized relative path (forward slashes).
    path: String,
}

/// Walk the hierarchy collecting document entries; duplicate ids or paths
/// become `ConflictingIdentity` reasons. Structurally unsafe paths
/// (absolute, `..`) propagate as errors exactly like the reader.
fn collect_probe_hierarchy_documents(
    hierarchy: &[TreeNode],
    reasons: &mut Vec<DegradedReason>,
) -> Result<Vec<ProbeHierarchyDoc>, ChiknError> {
    let mut docs = Vec::new();
    let mut ids = HashSet::new();
    let mut paths = HashSet::new();
    collect_probe_hierarchy_documents_inner(hierarchy, &mut docs, &mut ids, &mut paths, reasons)?;
    Ok(docs)
}

fn collect_probe_hierarchy_documents_inner(
    hierarchy: &[TreeNode],
    docs: &mut Vec<ProbeHierarchyDoc>,
    ids: &mut HashSet<String>,
    paths: &mut HashSet<String>,
    reasons: &mut Vec<DegradedReason>,
) -> Result<(), ChiknError> {
    for node in hierarchy {
        match node {
            TreeNode::Document { id, path, .. } => {
                let normalized = super::reader::normalized_relative_document_path(path)?;
                if !ids.insert(id.clone()) {
                    reasons.push(DegradedReason::ConflictingIdentity {
                        detail: format!("duplicate document id {id}"),
                    });
                }
                if !paths.insert(normalized.clone()) {
                    reasons.push(DegradedReason::ConflictingIdentity {
                        detail: format!("duplicate document path {normalized}"),
                    });
                }
                docs.push(ProbeHierarchyDoc {
                    id: id.clone(),
                    path: normalized,
                });
            }
            TreeNode::Folder { children, .. } => {
                collect_probe_hierarchy_documents_inner(children, docs, ids, paths, reasons)?;
            }
        }
    }
    Ok(())
}

fn check_hierarchy_document_resolves(
    project_path: &Path,
    doc: &ProbeHierarchyDoc,
    reasons: &mut Vec<DegradedReason>,
) {
    // Extension gate first: a non-.md path can never load as text in this
    // engine. Known binary asset types (imported research: PDFs, images,
    // audio) are legitimate binder references — the engine never loads or
    // writes their content, so they only need to exist. Any other non-.md
    // extension (e.g. the April-era .html text documents) is a text format
    // this engine cannot read and marks the project Degraded.
    let ext = Path::new(&doc.path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());
    let is_md = ext.as_deref() == Some(DOCUMENT_EXTENSION);
    if !is_md {
        let is_asset = ext
            .as_deref()
            .is_some_and(|e| OPAQUE_ASSET_EXTENSIONS.contains(&e));
        if is_asset {
            let full = project_path.join(&doc.path);
            match fs::symlink_metadata(&full) {
                Ok(m) if m.is_file() && !m.file_type().is_symlink() => {}
                _ => {
                    reasons.push(DegradedReason::UnresolvedDocument {
                        path: doc.path.clone(),
                        detail: "the asset file is missing".to_string(),
                    });
                }
            }
        } else {
            reasons.push(DegradedReason::LegacyDocumentPath {
                path: doc.path.clone(),
            });
        }
        return;
    }

    // The reader only loads documents from these roots; anywhere else the
    // entry exists in the binder but its content never loads.
    let under_loaded_root = LOADED_DOCUMENT_ROOTS
        .iter()
        .any(|root| doc.path.starts_with(&format!("{root}/")));
    if !under_loaded_root {
        reasons.push(DegradedReason::UnresolvedDocument {
            path: doc.path.clone(),
            detail: "it is outside the folders this app reads".to_string(),
        });
        return;
    }

    let full = project_path.join(&doc.path);
    let metadata = match fs::symlink_metadata(&full) {
        Ok(m) => m,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            reasons.push(DegradedReason::UnresolvedDocument {
                path: doc.path.clone(),
                detail: "the file is missing".to_string(),
            });
            return;
        }
        Err(e) => {
            reasons.push(DegradedReason::UnresolvedDocument {
                path: doc.path.clone(),
                detail: format!("the file cannot be checked: {e}"),
            });
            return;
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        reasons.push(DegradedReason::UnresolvedDocument {
            path: doc.path.clone(),
            detail: "the path is not a regular file".to_string(),
        });
        return;
    }

    // Zero-byte documents are VALID — the app creates them deliberately.
    // Only nonzero bytes that cannot become content are unresolved.
    match fs::read(&full) {
        Ok(bytes) => {
            if !bytes.is_empty() {
                match String::from_utf8(bytes) {
                    Ok(text) => {
                        if text.is_empty() {
                            reasons.push(DegradedReason::UnresolvedDocument {
                                path: doc.path.clone(),
                                detail: "the file has bytes but yields no content".to_string(),
                            });
                        }
                    }
                    Err(_) => {
                        reasons.push(DegradedReason::UnresolvedDocument {
                            path: doc.path.clone(),
                            detail: "the file is not valid text".to_string(),
                        });
                    }
                }
            }
        }
        Err(e) => {
            reasons.push(DegradedReason::UnresolvedDocument {
                path: doc.path.clone(),
                detail: format!("the file cannot be read: {e}"),
            });
        }
    }
}

/// Scan the folders the reader loads for corrupt sidecars, orphan
/// documents, and identity conflicts — read-only.
fn scan_loaded_documents(
    project_path: &Path,
    hierarchy_docs: &[ProbeHierarchyDoc],
    reasons: &mut Vec<DegradedReason>,
) -> Result<(), ChiknError> {
    let hierarchy_paths: HashSet<&str> = hierarchy_docs.iter().map(|d| d.path.as_str()).collect();
    let hierarchy_by_path: HashMap<&str, &str> = hierarchy_docs
        .iter()
        .map(|d| (d.path.as_str(), d.id.as_str()))
        .collect();
    let hierarchy_by_id: HashMap<&str, &str> = hierarchy_docs
        .iter()
        .map(|d| (d.id.as_str(), d.path.as_str()))
        .collect();

    let mut seen_ids: HashMap<String, String> = HashMap::new(); // id -> path

    for root in LOADED_DOCUMENT_ROOTS {
        let folder = project_path.join(root);
        match fs::symlink_metadata(&folder) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    // The reader rejects these projects outright; probe
                    // classifies the load as impossible-to-trust.
                    reasons.push(DegradedReason::UnresolvedDocument {
                        path: root.to_string(),
                        detail: "a project folder is not a regular folder".to_string(),
                    });
                    continue;
                }
            }
            // Missing standard folders are benign self-heal, NOT Degraded.
            Err(e) if e.kind() == ErrorKind::NotFound => continue,
            Err(e) => return Err(e.into()),
        }
        scan_documents_folder(
            project_path,
            root,
            &hierarchy_paths,
            &hierarchy_by_path,
            &hierarchy_by_id,
            &mut seen_ids,
            reasons,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn scan_documents_folder(
    project_path: &Path,
    relative_dir: &str,
    hierarchy_paths: &HashSet<&str>,
    hierarchy_by_path: &HashMap<&str, &str>,
    hierarchy_by_id: &HashMap<&str, &str>,
    seen_ids: &mut HashMap<String, String>,
    reasons: &mut Vec<DegradedReason>,
) -> Result<(), ChiknError> {
    let dir = project_path.join(relative_dir);
    let mut entries: Vec<_> = fs::read_dir(&dir)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let relative = format!("{relative_dir}/{name}");

        if metadata.file_type().is_symlink() {
            // The reader refuses to load projects containing symlinked
            // entries; nothing can be trusted about the content.
            reasons.push(DegradedReason::UnresolvedDocument {
                path: relative,
                detail: "the path is a link, not a real file".to_string(),
            });
            continue;
        }

        if metadata.is_dir() {
            scan_documents_folder(
                project_path,
                &relative,
                hierarchy_paths,
                hierarchy_by_path,
                hierarchy_by_id,
                seen_ids,
                reasons,
            )?;
            continue;
        }

        if !metadata.is_file() {
            continue;
        }
        let is_md = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e == DOCUMENT_EXTENSION)
            .unwrap_or(false);
        if !is_md {
            continue;
        }

        // Orphan check: entities under characters/ and locations/ live
        // outside the hierarchy by design.
        let is_entity = relative.starts_with(&format!("{CHARACTERS_FOLDER}/"))
            || relative.starts_with(&format!("{LOCATIONS_FOLDER}/"));
        if !is_entity && !hierarchy_paths.contains(relative.as_str()) {
            reasons.push(DegradedReason::OrphanDocument {
                path: relative.clone(),
            });
        }

        // Sidecar check.
        let meta_path = path.with_extension(super::format::METADATA_EXTENSION);
        let meta_relative = meta_path
            .strip_prefix(project_path)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| meta_path.to_string_lossy().into_owned());
        let doc_id: Option<String> = match fs::symlink_metadata(&meta_path) {
            Ok(meta_md) => {
                if meta_md.file_type().is_symlink() || !meta_md.is_file() {
                    reasons.push(DegradedReason::CorruptSidecar {
                        path: meta_relative.clone(),
                    });
                    None
                } else {
                    match fs::read_to_string(&meta_path) {
                        Ok(text) => match serde_yaml::from_str::<DocumentMetadata>(&text) {
                            Ok(_) => explicit_sidecar_id(&text),
                            Err(_) => {
                                reasons.push(DegradedReason::CorruptSidecar {
                                    path: meta_relative.clone(),
                                });
                                None
                            }
                        },
                        Err(_) => {
                            reasons.push(DegradedReason::CorruptSidecar {
                                path: meta_relative.clone(),
                            });
                            None
                        }
                    }
                }
            }
            Err(e) if e.kind() == ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };

        // Identity checks — mirror the reader, which refuses to load a
        // project whose sidecar ids collide or disagree with the
        // hierarchy. A sidecar without an explicit id inherits the
        // hierarchy identity for a referenced path (like the reader's
        // fallback), so only explicit ids can conflict.
        let effective_id = doc_id.or_else(|| {
            hierarchy_by_path
                .get(relative.as_str())
                .map(|id| id.to_string())
        });
        if let Some(id) = effective_id {
            if let Some(previous_path) = seen_ids.get(&id) {
                reasons.push(DegradedReason::ConflictingIdentity {
                    detail: format!(
                        "documents '{previous_path}' and '{relative}' share the id {id}"
                    ),
                });
            } else {
                seen_ids.insert(id.clone(), relative.clone());
            }
            match (
                hierarchy_by_id.get(id.as_str()),
                hierarchy_by_path.get(relative.as_str()),
            ) {
                // The hierarchy knows this id but places it elsewhere.
                (Some(expected_path), _) if *expected_path != relative => {
                    reasons.push(DegradedReason::ConflictingIdentity {
                        detail: format!(
                            "document id {id} is listed at '{expected_path}' but found at '{relative}'"
                        ),
                    });
                }
                // The hierarchy lists this path under a different id.
                (None, Some(expected_id)) if *expected_id != id => {
                    reasons.push(DegradedReason::ConflictingIdentity {
                        detail: format!(
                            "'{relative}' is listed as {expected_id} but its notes file says {id}"
                        ),
                    });
                }
                _ => {}
            }
        }
    }
    Ok(())
}

/// The reader generates a random id for sidecars that omit `id:` — which
/// can never match the hierarchy. Only an explicit id participates in
/// identity checks; an omitted one falls back to the hierarchy identity.
fn explicit_sidecar_id(meta_text: &str) -> Option<String> {
    serde_yaml::from_str::<serde_yaml::Value>(meta_text)
        .ok()
        .and_then(|v| v.get("id").and_then(|id| id.as_str().map(String::from)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::project::format::{
        MANUSCRIPT_FOLDER, PROJECT_FILE, RESEARCH_FOLDER, SETTINGS_FOLDER, TEMPLATES_FOLDER,
    };
    use std::collections::BTreeMap;
    use tempfile::TempDir;

    /// Byte-exact snapshot of every file under `root` (relative path →
    /// contents). Equality of two snapshots proves nothing was written.
    pub(crate) fn tree_snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
        let mut out = BTreeMap::new();
        snapshot_inner(root, root, &mut out);
        out
    }

    fn snapshot_inner(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let metadata = fs::symlink_metadata(&path).unwrap();
            if metadata.is_dir() {
                out.insert(format!("{rel}/"), Vec::new());
                snapshot_inner(root, &path, out);
            } else if metadata.is_file() {
                out.insert(rel, fs::read(&path).unwrap());
            }
        }
    }

    fn write_meta(dir: &Path, stem: &str, id: &str) {
        fs::write(
            dir.join(format!("{stem}.meta")),
            format!("id: \"{id}\"\ncreated: \"2025-01-01T00:00:00Z\"\nmodified: \"2025-01-01T00:00:00Z\"\n"),
        )
        .unwrap();
    }

    /// Minimal valid project: one referenced document with sidecar.
    fn create_probe_project() -> (TempDir, PathBuf) {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("Probe.chikn");
        fs::create_dir(&root).unwrap();
        for folder in [
            MANUSCRIPT_FOLDER,
            RESEARCH_FOLDER,
            TEMPLATES_FOLDER,
            SETTINGS_FOLDER,
        ] {
            fs::create_dir(root.join(folder)).unwrap();
        }
        fs::write(
            root.join(PROJECT_FILE),
            r#"format_version: '1.2'
id: "prj"
name: "Probe"
created: "2025-01-01T00:00:00Z"
modified: "2025-01-01T00:00:00Z"
hierarchy:
  - type: Document
    id: "doc1"
    name: "Chapter 1"
    path: "manuscript/chapter-01.md"
"#,
        )
        .unwrap();
        let ms = root.join(MANUSCRIPT_FOLDER);
        fs::write(ms.join("chapter-01.md"), "# Chapter 1\n").unwrap();
        write_meta(&ms, "chapter-01", "doc1");
        (temp, root)
    }

    fn probe(root: &Path) -> Fidelity {
        probe_project_fidelity(root).expect("probe must succeed")
    }

    #[test]
    fn full_on_intact_project() {
        let (_t, root) = create_probe_project();
        assert_eq!(probe(&root), Fidelity::Full);
    }

    #[test]
    fn full_when_standard_folders_missing() {
        // Missing research/templates/settings is benign self-heal, not
        // Degraded.
        let (_t, root) = create_probe_project();
        fs::remove_dir(root.join(RESEARCH_FOLDER)).unwrap();
        fs::remove_dir(root.join(TEMPLATES_FOLDER)).unwrap();
        fs::remove_dir(root.join(SETTINGS_FOLDER)).unwrap();
        assert_eq!(probe(&root), Fidelity::Full);
    }

    #[test]
    fn full_when_format_version_absent() {
        let (_t, root) = create_probe_project();
        let stripped: String = fs::read_to_string(root.join(PROJECT_FILE))
            .unwrap()
            .lines()
            .filter(|l| !l.starts_with("format_version:"))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(root.join(PROJECT_FILE), format!("{stripped}\n")).unwrap();
        assert_eq!(probe(&root), Fidelity::Full);
    }

    #[test]
    fn full_with_zero_byte_document() {
        let (_t, root) = create_probe_project();
        let ms = root.join(MANUSCRIPT_FOLDER);
        fs::write(ms.join("empty.md"), "").unwrap();
        write_meta(&ms, "empty", "doc-empty");
        let yaml = fs::read_to_string(root.join(PROJECT_FILE)).unwrap();
        fs::write(
            root.join(PROJECT_FILE),
            format!(
                "{}  - type: Document\n    id: \"doc-empty\"\n    name: \"Empty\"\n    path: \"manuscript/empty.md\"\n",
                yaml
            ),
        )
        .unwrap();
        assert_eq!(probe(&root), Fidelity::Full);
    }

    #[test]
    fn degraded_on_legacy_html_hierarchy_path() {
        let (_t, root) = create_probe_project();
        fs::write(
            root.join(PROJECT_FILE),
            r#"id: "prj"
name: "Probe"
created: "2025-01-01T00:00:00Z"
modified: "2025-01-01T00:00:00Z"
hierarchy:
  - type: Document
    id: "doc1"
    name: "Untitled"
    path: "manuscript/untitled.html"
"#,
        )
        .unwrap();
        fs::write(root.join("manuscript/untitled.html"), "<p>real work</p>").unwrap();
        // The old .md doc is now an orphan too, but the legacy reason must
        // be present regardless.
        match probe(&root) {
            Fidelity::Degraded { reasons } => {
                assert!(
                    reasons
                        .iter()
                        .any(|r| matches!(r, DegradedReason::LegacyDocumentPath { path } if path == "manuscript/untitled.html")),
                    "expected LegacyDocumentPath, got {reasons:?}"
                );
            }
            other => panic!("expected Degraded, got {other:?}"),
        }
    }

    #[test]
    fn degraded_on_missing_referenced_document() {
        let (_t, root) = create_probe_project();
        fs::remove_file(root.join("manuscript/chapter-01.md")).unwrap();
        fs::remove_file(root.join("manuscript/chapter-01.meta")).unwrap();
        match probe(&root) {
            Fidelity::Degraded { reasons } => {
                assert!(
                    reasons.iter().any(|r| matches!(
                        r,
                        DegradedReason::UnresolvedDocument { path, .. } if path == "manuscript/chapter-01.md"
                    )),
                    "expected UnresolvedDocument, got {reasons:?}"
                );
            }
            other => panic!("expected Degraded, got {other:?}"),
        }
    }

    #[test]
    fn degraded_on_corrupt_sidecar() {
        let (_t, root) = create_probe_project();
        fs::write(root.join("manuscript/chapter-01.meta"), "id: [").unwrap();
        match probe(&root) {
            Fidelity::Degraded { reasons } => {
                assert!(
                    reasons.iter().any(|r| matches!(
                        r,
                        DegradedReason::CorruptSidecar { path } if path == "manuscript/chapter-01.meta"
                    )),
                    "expected CorruptSidecar, got {reasons:?}"
                );
            }
            other => panic!("expected Degraded, got {other:?}"),
        }
    }

    #[test]
    fn degraded_on_orphan_document() {
        let (_t, root) = create_probe_project();
        let ms = root.join(MANUSCRIPT_FOLDER);
        fs::write(ms.join("orphan.md"), "restored but unlisted").unwrap();
        write_meta(&ms, "orphan", "orphan-1");
        match probe(&root) {
            Fidelity::Degraded { reasons } => {
                assert!(
                    reasons.iter().any(|r| matches!(
                        r,
                        DegradedReason::OrphanDocument { path } if path == "manuscript/orphan.md"
                    )),
                    "expected OrphanDocument, got {reasons:?}"
                );
            }
            other => panic!("expected Degraded, got {other:?}"),
        }
    }

    #[test]
    fn degraded_on_newer_format_version() {
        let (_t, root) = create_probe_project();
        let yaml = fs::read_to_string(root.join(PROJECT_FILE))
            .unwrap()
            .replace("format_version: '1.2'", "format_version: '9.9'");
        fs::write(root.join(PROJECT_FILE), yaml).unwrap();
        match probe(&root) {
            Fidelity::Degraded { reasons } => {
                assert_eq!(
                    reasons,
                    vec![DegradedReason::NewerFormatVersion {
                        found: "9.9".to_string()
                    }]
                );
            }
            other => panic!("expected Degraded, got {other:?}"),
        }
    }

    #[test]
    fn degraded_on_unparsable_format_version() {
        let (_t, root) = create_probe_project();
        let yaml = fs::read_to_string(root.join(PROJECT_FILE))
            .unwrap()
            .replace("format_version: '1.2'", "format_version: 'bananas'");
        fs::write(root.join(PROJECT_FILE), yaml).unwrap();
        match probe(&root) {
            Fidelity::Degraded { reasons } => {
                assert_eq!(
                    reasons,
                    vec![DegradedReason::UnsupportedFormatVersion {
                        found: "bananas".to_string()
                    }]
                );
            }
            other => panic!("expected Degraded, got {other:?}"),
        }
    }

    #[test]
    fn degraded_on_conflicting_identity() {
        let (_t, root) = create_probe_project();
        // Sidecar disagrees with the hierarchy about the document's id.
        write_meta(&root.join(MANUSCRIPT_FOLDER), "chapter-01", "other-id");
        match probe(&root) {
            Fidelity::Degraded { reasons } => {
                assert!(
                    reasons
                        .iter()
                        .any(|r| matches!(r, DegradedReason::ConflictingIdentity { .. })),
                    "expected ConflictingIdentity, got {reasons:?}"
                );
            }
            other => panic!("expected Degraded, got {other:?}"),
        }
    }

    #[test]
    fn probe_is_side_effect_free() {
        // Degraded fixture with a corrupt sidecar AND missing standard
        // folders: the probe must not quarantine, create folders, or write
        // anything at all.
        let (_t, root) = create_probe_project();
        fs::write(root.join("manuscript/chapter-01.meta"), "id: [").unwrap();
        fs::remove_dir(root.join(RESEARCH_FOLDER)).unwrap();
        fs::remove_dir(root.join(TEMPLATES_FOLDER)).unwrap();
        fs::remove_dir(root.join(SETTINGS_FOLDER)).unwrap();

        let before = tree_snapshot(&root);
        let fidelity = probe(&root);
        assert!(matches!(fidelity, Fidelity::Degraded { .. }));
        assert_eq!(
            before,
            tree_snapshot(&root),
            "probe must be byte-for-byte side-effect-free"
        );
    }

    #[test]
    fn acquire_token_on_full_project() {
        let (_t, root) = create_probe_project();
        let token = acquire_write_token(&root).expect("full project must yield a token");
        assert_eq!(token.root(), root.canonicalize().unwrap());
        assert!(!token.is_stale());
    }

    #[test]
    fn acquire_refused_on_degraded_project() {
        let (_t, root) = create_probe_project();
        fs::write(root.join("manuscript/chapter-01.meta"), "id: [").unwrap();
        let result = acquire_write_token(&root);
        assert!(
            matches!(result, Err(ChiknError::ReadOnly(_))),
            "degraded project must refuse a write token: {result:?}"
        );
    }

    #[test]
    fn token_rejected_for_other_project() {
        let (_ta, root_a) = create_probe_project();
        let (_tb, root_b) = create_probe_project();
        let token_a = acquire_write_token(&root_a).unwrap();
        assert!(token_a.with_write_permit(&root_a, |_| Ok(())).is_ok());
        let result = token_a.with_write_permit(&root_b, |_| Ok(()));
        assert!(
            matches!(result, Err(ChiknError::ReadOnly(_))),
            "token for project A must not validate against project B: {result:?}"
        );
    }

    #[test]
    fn token_stale_after_epoch_bump() {
        let (_t, root) = create_probe_project();
        let token = acquire_write_token(&root).unwrap();
        token
            .with_write_permit(&root, |permit| {
                permit.ensure_fresh()?;
                drop(permit.arm_epoch_bump());
                Ok(())
            })
            .unwrap();
        assert!(token.is_stale());
        assert!(matches!(
            token.with_write_permit(&root, |_| Ok(())),
            Err(ChiknError::ReadOnly(_))
        ));
        // Re-acquiring after the bump yields a fresh, valid token.
        let fresh = acquire_write_token(&root).unwrap();
        assert!(fresh.with_write_permit(&root, |_| Ok(())).is_ok());
    }

    #[test]
    fn fresh_fidelity_old_session_cannot_begin_operation() {
        let (_t, root) = create_probe_project();
        let token = acquire_write_token(&root).unwrap();

        let yaml = fs::read_to_string(root.join(PROJECT_FILE))
            .unwrap()
            .replace("format_version: '1.2'", "format_version: '9.9'");
        fs::write(root.join(PROJECT_FILE), yaml).unwrap();
        let before = tree_snapshot(&root);

        assert!(
            !token.is_stale(),
            "external edits do not change the in-process epoch"
        );
        let result = token.with_write_permit(&root, |_| Ok(()));
        assert!(matches!(result, Err(ChiknError::ReadOnly(_))));
        assert_eq!(before, tree_snapshot(&root));
    }
}
