//! `POST /api/export`: compile the manuscript and hand the file back as a
//! download. The desktop app writes wherever the save dialog pointed; a
//! browser has no such dialog, so the server compiles into a temporary folder
//! (deleted after the response is built) and never writes under the projects
//! root.

use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chickenscratch_services::io;
use serde::Deserialize;

use crate::commands::CmdError;
use crate::gate::PathArg;
use crate::state::AppState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportArgs {
    project_path: PathArg,
    format: String,
    title: Option<String>,
    author: Option<String>,
    section_separator: Option<String>,
    include_title_page: Option<bool>,
    manuscript_format: Option<bool>,
}

fn content_type(ext: &str) -> &'static str {
    match ext {
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "pdf" => "application/pdf",
        "epub" => "application/epub+zip",
        "html" => "text/html; charset=utf-8",
        "odt" => "application/vnd.oasis.opendocument.text",
        _ => "application/octet-stream",
    }
}

/// A download name safe for a `Content-Disposition` header: ASCII letters,
/// digits, space, dash, underscore, dot; anything else becomes a dash.
pub fn download_name(title: &str, ext: &str) -> String {
    let mut stem: String = title
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_') {
                c
            } else {
                '-'
            }
        })
        .collect();
    stem = stem.trim().trim_matches('-').trim().to_string();
    if stem.is_empty() {
        stem = "Manuscript".to_string();
    }
    format!("{stem}.{ext}")
}

pub async fn export(
    State(state): State<Arc<AppState>>,
    Json(args): Json<ExportArgs>,
) -> Result<Response, CmdError> {
    let known = io::get_compile_formats();
    if !known.iter().any(|(ext, _)| *ext == args.format) {
        return Err(CmdError::BadRequest(format!(
            "Unknown export format: {}",
            args.format
        )));
    }
    let project_path = args.project_path.existing(&state.gate)?;
    let format = args.format.clone();
    let title_for_name = args
        .title
        .clone()
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| {
            std::path::Path::new(&project_path)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Manuscript".to_string())
        });
    let file_name = download_name(&title_for_name, &format);

    let (bytes, file_name) = tokio::task::spawn_blocking(move || {
        let dir = tempfile::tempdir().map_err(chickenscratch_services::ChiknError::from)?;
        let out = dir.path().join(&file_name);
        io::compile_project(
            project_path,
            out.to_string_lossy().into_owned(),
            format,
            args.title,
            args.author,
            args.section_separator,
            args.include_title_page,
            args.manuscript_format,
        )?;
        let bytes = std::fs::read(&out).map_err(chickenscratch_services::ChiknError::from)?;
        Ok::<_, CmdError>((bytes, file_name))
    })
    .await
    .map_err(|_| CmdError::Internal)??;

    let disposition = format!("attachment; filename=\"{file_name}\"");
    Ok((
        StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static(content_type(&args.format)),
            ),
            (
                header::CONTENT_DISPOSITION,
                HeaderValue::from_str(&disposition)
                    .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
            ),
        ],
        Body::from(bytes),
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use super::download_name;

    #[test]
    fn download_names_are_header_safe() {
        assert_eq!(download_name("My Novel", "docx"), "My Novel.docx");
        assert_eq!(download_name("Über: Draft/2", "pdf"), "ber- Draft-2.pdf");
        assert_eq!(download_name("   ", "html"), "Manuscript.html");
        assert_eq!(download_name("\"quoted\"", "odt"), "quoted.odt");
    }
}
