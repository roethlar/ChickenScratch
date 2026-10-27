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

    let (bytes, file_name, warnings) = tokio::task::spawn_blocking(move || {
        let dir = tempfile::tempdir().map_err(chickenscratch_services::ChiknError::from)?;
        let out = dir.path().join(&file_name);
        let warnings = io::compile_project(
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
        Ok::<_, CmdError>((bytes, file_name, warnings))
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
            (
                // The body is the file, so compile's warnings travel in a
                // header the UI reads after the download (review finding
                // s3-5).
                WARNINGS_HEADER,
                HeaderValue::from_str(&warnings_header(&warnings))
                    .unwrap_or_else(|_| HeaderValue::from_static("%5B%5D")),
            ),
        ],
        Body::from(bytes),
    )
        .into_response())
}

/// Compile's warnings, as a JSON array percent-encoded so any text fits a
/// header's visible-ASCII rule. The UI decodes it with `decodeURIComponent`.
pub const WARNINGS_HEADER: header::HeaderName =
    header::HeaderName::from_static("x-cs-compile-warnings");

pub fn warnings_header(warnings: &[String]) -> String {
    let json = serde_json::to_string(warnings).unwrap_or_else(|_| "[]".to_string());
    let mut out = String::with_capacity(json.len());
    for byte in json.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{download_name, warnings_header};

    /// Any warning text, non-ASCII included, becomes a visible-ASCII header
    /// value that decodes back to the JSON array (s3-5).
    #[test]
    fn warnings_travel_as_a_percent_encoded_json_array() {
        assert_eq!(warnings_header(&[]), "%5B%5D");
        let value = warnings_header(&["b.md#café was not found; the link became text".to_string()]);
        assert!(value.bytes().all(|b| b.is_ascii_graphic()), "{value}");
        assert!(value.contains("%C3%A9"), "{value}");
        assert!(value.starts_with("%5B%22b.md%23caf"), "{value}");
    }

    #[test]
    fn download_names_are_header_safe() {
        assert_eq!(download_name("My Novel", "docx"), "My Novel.docx");
        assert_eq!(download_name("Über: Draft/2", "pdf"), "ber- Draft-2.pdf");
        assert_eq!(download_name("   ", "html"), "Manuscript.html");
        assert_eq!(download_name("\"quoted\"", "odt"), "quoted.odt");
    }
}
