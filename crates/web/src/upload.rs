//! `POST /api/import-upload`: import a file the browser sends, instead of a
//! server path the desktop's open dialog would have produced.
//!
//! Multipart fields: `projectPath` (text), optional `parentId` (text), `file`.
//! The upload lands in a temporary folder outside the projects root, the
//! ordinary `import_file` service runs on it, and the folder is deleted when
//! the request ends — so no gate widening and nothing lingers on disk.

use std::path::Path;
use std::sync::Arc;

use axum::{
    extract::{Multipart, State},
    response::{IntoResponse, Response},
    Json,
};
use chickenscratch_services::{io, ChiknError};

use crate::commands::CmdError;
use crate::state::AppState;

/// Just the final name component, with path separators and control
/// characters replaced; the extension is kept because it picks the importer.
pub fn safe_upload_name(raw: Option<&str>) -> String {
    let base = raw
        .map(|r| r.replace('\\', "/"))
        .and_then(|r| {
            Path::new(&r)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
        })
        .unwrap_or_default();
    let cleaned: String = base
        .chars()
        .map(|c| if c.is_control() || c == '/' { '-' } else { c })
        .collect();
    let cleaned = cleaned.trim().trim_start_matches('.').to_string();
    if cleaned.is_empty() {
        "upload.md".to_string()
    } else {
        cleaned
    }
}

fn bad(e: impl std::fmt::Display) -> CmdError {
    CmdError::BadRequest(format!("Invalid upload: {e}"))
}

pub async fn import_upload(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Response, CmdError> {
    let mut project_path: Option<String> = None;
    let mut parent_id: Option<String> = None;
    let mut file: Option<(String, Vec<u8>)> = None;

    while let Some(field) = multipart.next_field().await.map_err(bad)? {
        match field.name() {
            Some("projectPath") => project_path = Some(field.text().await.map_err(bad)?),
            Some("parentId") => {
                let text = field.text().await.map_err(bad)?;
                if !text.is_empty() {
                    parent_id = Some(text);
                }
            }
            Some("file") => {
                let name = safe_upload_name(field.file_name());
                let bytes = field.bytes().await.map_err(bad)?;
                file = Some((name, bytes.to_vec()));
            }
            _ => {}
        }
    }

    let project_path = project_path.ok_or_else(|| bad("missing projectPath"))?;
    let project_path = state
        .gate
        .existing(&project_path)?
        .to_string_lossy()
        .into_owned();
    let (name, bytes) = file.ok_or_else(|| bad("missing file"))?;

    let project = tokio::task::spawn_blocking(move || {
        let dir = tempfile::tempdir().map_err(ChiknError::from)?;
        let staged = dir.path().join(&name);
        std::fs::write(&staged, &bytes).map_err(ChiknError::from)?;
        let project = io::import_file(
            project_path,
            &state.locks,
            &state.tokens,
            staged.to_string_lossy().into_owned(),
            parent_id,
        )?;
        Ok::<_, CmdError>(project)
    })
    .await
    .map_err(|_| CmdError::Internal)??;

    Ok(Json(project).into_response())
}

#[cfg(test)]
mod tests {
    use super::safe_upload_name;

    #[test]
    fn upload_names_keep_the_extension_and_drop_paths() {
        assert_eq!(safe_upload_name(Some("note.md")), "note.md");
        assert_eq!(safe_upload_name(Some("../../etc/passwd")), "passwd");
        assert_eq!(
            safe_upload_name(Some("C:\\Users\\me\\Draft.docx")),
            "Draft.docx"
        );
        assert_eq!(safe_upload_name(Some(".hidden")), "hidden");
        assert_eq!(safe_upload_name(Some("")), "upload.md");
        assert_eq!(safe_upload_name(None), "upload.md");
    }
}
