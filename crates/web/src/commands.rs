//! The command table: one entry per Tauri command, same name, same camelCase
//! argument keys, same result and error shapes, dispatched to
//! `chickenscratch_services`.
//!
//! `POST /api/cmd/{name}` with a JSON object body. Responses:
//! - `200` + JSON result (`null` for unit commands, as Tauri resolves them);
//! - `400` + JSON string for malformed arguments or a relative path;
//! - `403` + JSON string when a path resolves outside the projects root;
//! - `404` + JSON string for an unknown command name;
//! - `422` + JSON string carrying the `ChiknError` display text — the UI
//!   renders this string exactly as it renders a Tauri rejection;
//! - `500` + JSON string if the worker panicked.
//!
//! Every command body blocks (git2, Pandoc, network), so each runs on the
//! blocking pool.

use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chickenscratch_services as svc;
use chickenscratch_services::document::FieldUpdates;
use chickenscratch_services::settings::AiSettings;
use chickenscratch_services::{AppSettings, ChiknError, Project, SessionTarget};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::events::WebSink;
use crate::gate::{GateError, PathArg};
use crate::state::AppState;

#[derive(Debug)]
pub enum CmdError {
    Chikn(ChiknError),
    Gate(GateError),
    BadRequest(String),
    Unknown(String),
    Internal,
}

impl From<ChiknError> for CmdError {
    fn from(e: ChiknError) -> Self {
        CmdError::Chikn(e)
    }
}

impl From<GateError> for CmdError {
    fn from(e: GateError) -> Self {
        CmdError::Gate(e)
    }
}

impl IntoResponse for CmdError {
    fn into_response(self) -> Response {
        match self {
            // Serialize through ChiknError's own impl (its Display string).
            CmdError::Chikn(e) => (StatusCode::UNPROCESSABLE_ENTITY, Json(e)).into_response(),
            CmdError::Gate(e) => (e.status(), Json(e.to_string())).into_response(),
            CmdError::BadRequest(m) => (StatusCode::BAD_REQUEST, Json(m)).into_response(),
            CmdError::Unknown(name) => (
                StatusCode::NOT_FOUND,
                Json(format!("Unknown command: {name}")),
            )
                .into_response(),
            CmdError::Internal => {
                (StatusCode::INTERNAL_SERVER_ERROR, Json("Internal error")).into_response()
            }
        }
    }
}

fn not_in_web(what: &str) -> CmdError {
    CmdError::Chikn(ChiknError::Unknown(format!(
        "{what} is not available in the web version yet"
    )))
}

/// Deserialize the camelCase args, run the body on the blocking pool, JSON the result.
async fn run<A, T, F>(state: Arc<AppState>, body: Bytes, f: F) -> Result<Response, CmdError>
where
    A: DeserializeOwned + Send + 'static,
    T: Serialize + Send + 'static,
    F: FnOnce(A, &AppState) -> Result<T, CmdError> + Send + 'static,
{
    let raw: &[u8] = if body.is_empty() { b"{}" } else { &body };
    let args: A = serde_json::from_slice(raw)
        .map_err(|e| CmdError::BadRequest(format!("Invalid arguments: {e}")))?;
    let out = tokio::task::spawn_blocking(move || f(args, &state))
        .await
        .map_err(|_| CmdError::Internal)??;
    Ok(Json(out).into_response())
}

macro_rules! args {
    ($( $name:ident { $( $field:ident : $ty:ty ),* $(,)? } )*) => {
        $(
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct $name { $( $field: $ty, )* }
        )*
    };
}

args! {
    NoArgs {}
    // project
    CreateProject { name: String, path: PathArg }
    LoadProject { path: PathArg }
    SaveProject { project: Project }
    ImportScrivener { scriv_path: PathArg, output_path: PathArg }
    UpdateProjectMetadata {
        project_path: PathArg, title: Option<String>, author: Option<String>,
        project_type: Option<String>, genre: Option<String>, theme: Option<String>,
        summary: Option<String>, session_target: Option<SessionTarget>,
    }
    // document
    GetDocument { project_path: PathArg, doc_id: String }
    UpdateDocumentContent { project_path: PathArg, doc_id: String, content: String }
    AddComment {
        project_path: PathArg, doc_id: String, comment_id: String, body: String,
        new_content: String,
    }
    UpdateComment {
        project_path: PathArg, doc_id: String, comment_id: String,
        body: Option<String>, resolved: Option<bool>,
    }
    DeleteComment { project_path: PathArg, doc_id: String, comment_id: String, new_content: String }
    UpdateDocumentMetadata {
        project_path: PathArg, doc_id: String, synopsis: Option<String>,
        label: Option<String>, status: Option<String>, keywords: Option<Vec<String>>,
        include_in_compile: Option<bool>, word_count_target: Option<u32>,
        compile_order: Option<i32>, fields: Option<FieldUpdates>,
    }
    RenameNode { project_path: PathArg, node_id: String, new_name: String }
    LinkDocuments { project_path: PathArg, doc_id_a: String, doc_id_b: String }
    CreateDocument { project_path: PathArg, name: String, parent_id: Option<String> }
    CreateEntity { project_path: PathArg, name: String, kind: String }
    CreateFolder { project_path: PathArg, name: String, parent_id: Option<String> }
    DeleteNode { project_path: PathArg, node_id: String }
    MoveNode {
        project_path: PathArg, node_id: String, new_parent_id: Option<String>,
        new_index: Option<usize>,
    }
    // ai
    SaveAiSettings { ai: AiSettings }
    Content { content: String }
    Transform { content: String, operation: String }
    RequestId { request_id: String }
    AiStream { content: String, operation: String, request_id: String }
    // git
    ProjectPath { project_path: PathArg }
    ProjectMessage { project_path: PathArg, message: String }
    ProjectCommit { project_path: PathArg, commit_id: String }
    ProjectName { project_path: PathArg, name: String }
    ProjectBackupDir { project_path: PathArg, backup_dir: String }
    ProjectDocPath { project_path: PathArg, doc_path: String }
    RestoreDocument { project_path: PathArg, doc_path: String, commit_id: String }
    ForceResolveMerge { project_path: PathArg, attestation: String }
    WordDiff { project_path: PathArg, commit_id: String, doc_path: String }
    CompareDrafts { project_path: PathArg, draft_a: String, draft_b: String }
    WordDiffDrafts { project_path: PathArg, draft_a: String, draft_b: String, doc_path: String }
    ProjectDraft { project_path: PathArg, draft: String }
    ReadDraftDocument { project_path: PathArg, draft: String, doc_path: String }
    // io
    CompileProject {
        project_path: PathArg, output_path: PathArg, format: String,
        title: Option<String>, author: Option<String>, section_separator: Option<String>,
        include_title_page: Option<bool>, manuscript_format: Option<bool>,
    }
    ImportFile { project_path: PathArg, file_path: PathArg, parent_id: Option<String> }
    ImportMarkdownFolder { folder_path: PathArg, output_path: PathArg }
    RecordDailyWords { project_path: PathArg, words: usize }
    // settings
    SaveAppSettings { settings: AppSettings }
    AddRecentProject { name: String, path: String }
    // search / templates / threads
    SearchProject { project_path: PathArg, query: String }
    CreateFromTemplate {
        project_path: PathArg, template_id: String, name: String, parent_id: Option<String>,
    }
    ProjectDocId { project_path: PathArg, doc_id: String }
    CreateThread {
        project_path: PathArg, name: String, color: Option<String>, description: Option<String>,
    }
    UpdateThread {
        project_path: PathArg, id: String, name: Option<String>, color: Option<String>,
        description: Option<String>,
    }
    DeleteThread { project_path: PathArg, id: String }
}

pub async fn handle(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    body: Bytes,
) -> Response {
    match dispatch(state, &name, body).await {
        Ok(response) => response,
        Err(e) => e.into_response(),
    }
}

async fn dispatch(st: Arc<AppState>, name: &str, body: Bytes) -> Result<Response, CmdError> {
    use svc::{ai, document, git, io, project, search, settings, templates, threads};
    let response = match name {
        // ── project ──────────────────────────────────────────────────
        "create_project" => {
            run(st, body, |a: CreateProject, s| {
                Ok(project::create_project(
                    a.name,
                    a.path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                )?)
            })
            .await?
        }
        "load_project" => {
            run(st, body, |a: LoadProject, s| {
                Ok(project::load_project(
                    a.path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                )?)
            })
            .await?
        }
        "save_project" => {
            run(st, body, |mut a: SaveProject, s| {
                // The engine writes wherever `project.path` says: gate it too.
                a.project.path = s
                    .gate
                    .existing(&a.project.path)?
                    .to_string_lossy()
                    .into_owned();
                Ok(project::save_project(a.project, &s.locks, &s.tokens)?)
            })
            .await?
        }
        "import_scrivener" => {
            run(st, body, |a: ImportScrivener, s| {
                Ok(project::import_scrivener(
                    a.scriv_path.existing(&s.gate)?,
                    a.output_path.creatable(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                )?)
            })
            .await?
        }
        "update_project_metadata" => {
            run(st, body, |a: UpdateProjectMetadata, s| {
                Ok(project::update_project_metadata(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                    a.title,
                    a.author,
                    a.project_type,
                    a.genre,
                    a.theme,
                    a.summary,
                    a.session_target,
                )?)
            })
            .await?
        }
        "pick_scriv_folder" => return Err(not_in_web("The Scrivener folder picker")),
        // The desktop clears its open-project state for the `chikn-asset`
        // scheme; the web shell gates every asset request by project path
        // instead, so there is nothing to clear (slice 3).
        "close_project" => run(st, body, |_: NoArgs, _| Ok(())).await?,

        // ── document ─────────────────────────────────────────────────
        "get_document" => {
            run(st, body, |a: GetDocument, s| {
                Ok(document::get_document(
                    a.project_path.existing(&s.gate)?,
                    a.doc_id,
                )?)
            })
            .await?
        }
        "update_document_content" => {
            run(st, body, |a: UpdateDocumentContent, s| {
                Ok(document::update_document_content(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                    a.doc_id,
                    a.content,
                )?)
            })
            .await?
        }
        "add_comment" => {
            run(st, body, |a: AddComment, s| {
                Ok(document::add_comment(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                    a.doc_id,
                    a.comment_id,
                    a.body,
                    a.new_content,
                )?)
            })
            .await?
        }
        "update_comment" => {
            run(st, body, |a: UpdateComment, s| {
                Ok(document::update_comment(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                    a.doc_id,
                    a.comment_id,
                    a.body,
                    a.resolved,
                )?)
            })
            .await?
        }
        "delete_comment" => {
            run(st, body, |a: DeleteComment, s| {
                Ok(document::delete_comment(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                    a.doc_id,
                    a.comment_id,
                    a.new_content,
                )?)
            })
            .await?
        }
        "update_document_metadata" => {
            run(st, body, |a: UpdateDocumentMetadata, s| {
                Ok(document::update_document_metadata(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                    a.doc_id,
                    a.synopsis,
                    a.label,
                    a.status,
                    a.keywords,
                    a.include_in_compile,
                    a.word_count_target,
                    a.compile_order,
                    a.fields,
                )?)
            })
            .await?
        }
        "rename_node" => {
            run(st, body, |a: RenameNode, s| {
                Ok(document::rename_node(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                    a.node_id,
                    a.new_name,
                )?)
            })
            .await?
        }
        "link_documents" => {
            run(st, body, |a: LinkDocuments, s| {
                Ok(document::link_documents(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                    a.doc_id_a,
                    a.doc_id_b,
                )?)
            })
            .await?
        }
        "create_document" => {
            run(st, body, |a: CreateDocument, s| {
                Ok(document::create_document(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                    a.name,
                    a.parent_id,
                )?)
            })
            .await?
        }
        "create_entity" => {
            run(st, body, |a: CreateEntity, s| {
                Ok(document::create_entity(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                    a.name,
                    a.kind,
                )?)
            })
            .await?
        }
        "create_folder" => {
            run(st, body, |a: CreateFolder, s| {
                Ok(document::create_folder(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                    a.name,
                    a.parent_id,
                )?)
            })
            .await?
        }
        "delete_node" => {
            run(st, body, |a: DeleteNode, s| {
                Ok(document::delete_node(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                    a.node_id,
                )?)
            })
            .await?
        }
        "move_node" => {
            run(st, body, |a: MoveNode, s| {
                Ok(document::move_node(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                    a.node_id,
                    a.new_parent_id,
                    a.new_index,
                )?)
            })
            .await?
        }

        // ── ai ───────────────────────────────────────────────────────
        "get_ai_settings" => run(st, body, |_: NoArgs, _| Ok(ai::get_ai_settings())).await?,
        "save_ai_settings" => {
            run(st, body, |a: SaveAiSettings, _| {
                Ok(ai::save_ai_settings(a.ai)?)
            })
            .await?
        }
        "ai_summarize" => run(st, body, |a: Content, _| Ok(ai::ai_summarize(a.content)?)).await?,
        "ai_transform" => {
            run(st, body, |a: Transform, _| {
                Ok(ai::ai_transform(a.content, a.operation)?)
            })
            .await?
        }
        "ai_transform_stream" => {
            run(st, body, |a: AiStream, s| {
                Ok(ai::ai_transform_stream(
                    Arc::new(WebSink(s.events.clone())),
                    &s.ai_registry,
                    a.content,
                    a.operation,
                    a.request_id,
                )?)
            })
            .await?
        }
        "cancel_ai_transform_stream" => {
            run(st, body, |a: RequestId, s| {
                Ok(ai::cancel_ai_transform_stream(
                    &s.ai_registry,
                    a.request_id,
                )?)
            })
            .await?
        }

        // ── git ──────────────────────────────────────────────────────
        "save_revision" => {
            run(st, body, |a: ProjectMessage, s| {
                Ok(git::save_revision(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                    a.message,
                )?)
            })
            .await?
        }
        "list_revisions" => {
            run(st, body, |a: ProjectPath, s| {
                Ok(git::list_revisions(a.project_path.existing(&s.gate)?)?)
            })
            .await?
        }
        "restore_revision" => {
            run(st, body, |a: ProjectCommit, s| {
                Ok(git::restore_revision(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                    a.commit_id,
                )?)
            })
            .await?
        }
        "create_draft" => {
            run(st, body, |a: ProjectName, s| {
                Ok(git::create_draft(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                    a.name,
                )?)
            })
            .await?
        }
        "list_drafts" => {
            run(st, body, |a: ProjectPath, s| {
                Ok(git::list_drafts(a.project_path.existing(&s.gate)?)?)
            })
            .await?
        }
        "switch_draft" => {
            run(st, body, |a: ProjectName, s| {
                Ok(git::switch_draft(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                    a.name,
                )?)
            })
            .await?
        }
        "merge_draft" => {
            run(st, body, |a: ProjectName, s| {
                Ok(git::merge_draft(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                    a.name,
                )?)
            })
            .await?
        }
        "push_backup" => {
            run(st, body, |a: ProjectBackupDir, s| {
                Ok(git::push_backup(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                    a.backup_dir,
                )?)
            })
            .await?
        }
        "manual_backup" => {
            run(st, body, |a: ProjectBackupDir, s| {
                Ok(git::manual_backup(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                    a.backup_dir,
                )?)
            })
            .await?
        }
        "sync_push" => {
            run(st, body, |a: ProjectPath, s| {
                Ok(git::sync_push(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                )?)
            })
            .await?
        }
        "sync_fetch" => {
            run(st, body, |a: ProjectPath, s| {
                Ok(git::sync_fetch(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                )?)
            })
            .await?
        }
        "sync_status" => {
            run(st, body, |a: ProjectPath, s| {
                Ok(git::sync_status(a.project_path.existing(&s.gate)?)?)
            })
            .await?
        }
        "document_history" => {
            run(st, body, |a: ProjectDocPath, s| {
                Ok(git::document_history(
                    a.project_path.existing(&s.gate)?,
                    a.doc_path,
                )?)
            })
            .await?
        }
        "restore_document" => {
            run(st, body, |a: RestoreDocument, s| {
                Ok(git::restore_document(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                    a.doc_path,
                    a.commit_id,
                )?)
            })
            .await?
        }
        "sync_pull" => {
            run(st, body, |a: ProjectPath, s| {
                Ok(git::sync_pull(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                )?)
            })
            .await?
        }
        "sync_abort_pull" => {
            run(st, body, |a: ProjectPath, s| {
                Ok(git::sync_abort_pull(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                )?)
            })
            .await?
        }
        "merge_state" => {
            run(st, body, |a: ProjectPath, s| {
                Ok(git::merge_state(a.project_path.existing(&s.gate)?)?)
            })
            .await?
        }
        "complete_merge" => {
            run(st, body, |a: ProjectMessage, s| {
                Ok(git::complete_merge(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                    a.message,
                )?)
            })
            .await?
        }
        "force_resolve_merge" => {
            run(st, body, |a: ForceResolveMerge, s| {
                Ok(git::force_resolve_merge(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                    a.attestation,
                )?)
            })
            .await?
        }
        "sync_pull_force" => {
            run(st, body, |a: ProjectPath, s| {
                Ok(git::sync_pull_force(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                )?)
            })
            .await?
        }
        "revision_diff" => {
            run(st, body, |a: ProjectCommit, s| {
                Ok(git::revision_diff(
                    a.project_path.existing(&s.gate)?,
                    a.commit_id,
                )?)
            })
            .await?
        }
        "word_diff" => {
            run(st, body, |a: WordDiff, s| {
                Ok(git::word_diff(
                    a.project_path.existing(&s.gate)?,
                    a.commit_id,
                    a.doc_path,
                )?)
            })
            .await?
        }
        "compare_drafts" => {
            run(st, body, |a: CompareDrafts, s| {
                Ok(git::compare_drafts(
                    a.project_path.existing(&s.gate)?,
                    a.draft_a,
                    a.draft_b,
                )?)
            })
            .await?
        }
        "word_diff_drafts" => {
            run(st, body, |a: WordDiffDrafts, s| {
                Ok(git::word_diff_drafts(
                    a.project_path.existing(&s.gate)?,
                    a.draft_a,
                    a.draft_b,
                    a.doc_path,
                )?)
            })
            .await?
        }
        "list_draft_documents" => {
            run(st, body, |a: ProjectDraft, s| {
                Ok(git::list_draft_documents(
                    a.project_path.existing(&s.gate)?,
                    a.draft,
                )?)
            })
            .await?
        }
        "read_draft_document" => {
            run(st, body, |a: ReadDraftDocument, s| {
                Ok(git::read_draft_document(
                    a.project_path.existing(&s.gate)?,
                    a.draft,
                    a.doc_path,
                )?)
            })
            .await?
        }
        "has_changes" => {
            run(st, body, |a: ProjectPath, s| {
                Ok(git::has_changes(a.project_path.existing(&s.gate)?)?)
            })
            .await?
        }
        "backup_on_close" => {
            run(st, body, |a: ProjectPath, s| {
                Ok(git::backup_on_close(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                )?)
            })
            .await?
        }

        // ── io ───────────────────────────────────────────────────────
        "compile_project" => {
            run(st, body, |a: CompileProject, s| {
                Ok(io::compile_project(
                    a.project_path.existing(&s.gate)?,
                    a.output_path.creatable(&s.gate)?,
                    a.format,
                    a.title,
                    a.author,
                    a.section_separator,
                    a.include_title_page,
                    a.manuscript_format,
                )?)
            })
            .await?
        }
        "get_compile_formats" => {
            run(st, body, |_: NoArgs, _| Ok(io::get_compile_formats())).await?
        }
        "import_file" => {
            run(st, body, |a: ImportFile, s| {
                Ok(io::import_file(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                    a.file_path.existing(&s.gate)?,
                    a.parent_id,
                )?)
            })
            .await?
        }
        "import_markdown_folder" => {
            run(st, body, |a: ImportMarkdownFolder, s| {
                Ok(io::import_markdown_folder(
                    a.folder_path.existing(&s.gate)?,
                    a.output_path.creatable(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                )?)
            })
            .await?
        }
        "get_writing_history" => {
            run(st, body, |a: ProjectPath, s| {
                Ok(io::get_writing_history(a.project_path.existing(&s.gate)?)?)
            })
            .await?
        }
        "record_daily_words" => {
            run(st, body, |a: RecordDailyWords, s| {
                Ok(io::record_daily_words(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                    a.words,
                )?)
            })
            .await?
        }
        "get_session_progress" => {
            run(st, body, |a: ProjectPath, s| {
                Ok(io::get_session_progress(a.project_path.existing(&s.gate)?)?)
            })
            .await?
        }
        "get_project_stats" => {
            run(st, body, |a: ProjectPath, s| {
                Ok(io::get_project_stats(a.project_path.existing(&s.gate)?)?)
            })
            .await?
        }

        // ── settings ─────────────────────────────────────────────────
        "get_app_settings" => {
            run(st, body, |_: NoArgs, _| Ok(settings::get_app_settings())).await?
        }
        "save_app_settings" => {
            run(st, body, |a: SaveAppSettings, _| {
                Ok(settings::save_app_settings(a.settings)?)
            })
            .await?
        }
        "get_recent_projects" => {
            run(st, body, |_: NoArgs, _| Ok(settings::get_recent_projects())).await?
        }
        "add_recent_project" => {
            run(st, body, |a: AddRecentProject, _| {
                Ok(settings::add_recent_project(a.name, a.path)?)
            })
            .await?
        }
        "check_pandoc" => run(st, body, |_: NoArgs, _| Ok(settings::check_pandoc()?)).await?,

        // ── search / templates / threads ─────────────────────────────
        "search_project" => {
            run(st, body, |a: SearchProject, s| {
                Ok(search::search_project(
                    a.project_path.existing(&s.gate)?,
                    a.query,
                )?)
            })
            .await?
        }
        "list_templates" => run(st, body, |_: NoArgs, _| Ok(templates::list_templates())).await?,
        "create_from_template" => {
            run(st, body, |a: CreateFromTemplate, s| {
                Ok(templates::create_from_template(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                    a.template_id,
                    a.name,
                    a.parent_id,
                )?)
            })
            .await?
        }
        "save_as_template" => {
            run(st, body, |a: ProjectDocId, s| {
                Ok(templates::save_as_template(
                    a.project_path.existing(&s.gate)?,
                    a.doc_id,
                )?)
            })
            .await?
        }
        "validate_references" => {
            run(st, body, |a: ProjectPath, s| {
                Ok(threads::validate_references(
                    a.project_path.existing(&s.gate)?,
                )?)
            })
            .await?
        }
        "list_threads" => {
            run(st, body, |a: ProjectPath, s| {
                Ok(threads::list_threads(a.project_path.existing(&s.gate)?)?)
            })
            .await?
        }
        "create_thread" => {
            run(st, body, |a: CreateThread, s| {
                Ok(threads::create_thread(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                    a.name,
                    a.color,
                    a.description,
                )?)
            })
            .await?
        }
        "update_thread" => {
            run(st, body, |a: UpdateThread, s| {
                Ok(threads::update_thread(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                    a.id,
                    a.name,
                    a.color,
                    a.description,
                )?)
            })
            .await?
        }
        "delete_thread" => {
            run(st, body, |a: DeleteThread, s| {
                Ok(threads::delete_thread(
                    a.project_path.existing(&s.gate)?,
                    &s.locks,
                    &s.tokens,
                    a.id,
                )?)
            })
            .await?
        }

        other => return Err(CmdError::Unknown(other.to_string())),
    };
    Ok(response)
}

/// Every Tauri command name, for the table-completeness test.
pub const COMMAND_NAMES: &[&str] = &[
    "create_project",
    "load_project",
    "save_project",
    "import_scrivener",
    "update_project_metadata",
    "pick_scriv_folder",
    "close_project",
    "get_document",
    "update_document_content",
    "update_document_metadata",
    "add_comment",
    "update_comment",
    "delete_comment",
    "rename_node",
    "link_documents",
    "create_document",
    "create_entity",
    "create_folder",
    "delete_node",
    "move_node",
    "get_ai_settings",
    "save_ai_settings",
    "ai_transform_stream",
    "cancel_ai_transform_stream",
    "ai_summarize",
    "ai_transform",
    "save_revision",
    "list_revisions",
    "restore_revision",
    "create_draft",
    "list_drafts",
    "switch_draft",
    "merge_draft",
    "push_backup",
    "manual_backup",
    "sync_push",
    "sync_fetch",
    "sync_status",
    "sync_pull",
    "sync_abort_pull",
    "sync_pull_force",
    "merge_state",
    "complete_merge",
    "force_resolve_merge",
    "document_history",
    "restore_document",
    "has_changes",
    "backup_on_close",
    "compile_project",
    "get_compile_formats",
    "import_file",
    "import_markdown_folder",
    "get_app_settings",
    "save_app_settings",
    "get_recent_projects",
    "add_recent_project",
    "check_pandoc",
    "search_project",
    "list_templates",
    "create_from_template",
    "save_as_template",
    "list_threads",
    "create_thread",
    "update_thread",
    "delete_thread",
    "validate_references",
    "revision_diff",
    "word_diff",
    "compare_drafts",
    "word_diff_drafts",
    "list_draft_documents",
    "read_draft_document",
    "get_project_stats",
    "get_session_progress",
    "get_writing_history",
    "record_daily_words",
];
