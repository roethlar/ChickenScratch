//! `POST /api/export`: compile the manuscript and hand the file back as a
//! download. The desktop app writes wherever the save dialog pointed; a
//! browser has no such dialog, so the server compiles into a temporary folder
//! (deleted after the response is built) and never writes under the projects
//! root.
//!
//! Compile's warnings (links between documents that became text, and why)
//! cannot ride in the download's body, and a header is the wrong place for an
//! unbounded list: a 200-warning report overflowed Node's 16 KiB header limit
//! behind the Vite proxy and turned a successful compile into a failed
//! download (repair review of finding s3-5). The headers therefore carry only
//! a count and a report id, and the UI fetches the report itself from
//! `GET /api/compile-report?id=<id>`.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use axum::{
    body::Body,
    extract::{Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
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
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(content_type(&args.format)),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&disposition)
            .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
    );
    headers.insert(WARNINGS_HEADER, HeaderValue::from(warnings.len()));
    if !warnings.is_empty() {
        let id = state.reports.store(warnings);
        headers.insert(REPORT_HEADER, HeaderValue::from(id));
    }
    Ok((StatusCode::OK, headers, Body::from(bytes)).into_response())
}

/// How many warnings the compile produced, as a decimal. Always present on a
/// successful export; the only thing about warnings the download's headers
/// say besides the report id.
pub const WARNINGS_HEADER: header::HeaderName =
    header::HeaderName::from_static("x-cs-compile-warnings");

/// The id of the stored report, present when the count is above zero.
pub const REPORT_HEADER: header::HeaderName =
    header::HeaderName::from_static("x-cs-compile-report");

/// Reports the newest `KEPT` exports left behind, in memory, for the UI to
/// fetch after its download. One per process, shared through `AppState`.
#[derive(Default)]
pub struct CompileReports {
    inner: Mutex<Reports>,
}

#[derive(Default)]
struct Reports {
    next_id: u64,
    kept: VecDeque<(u64, Vec<String>)>,
}

/// Enough for every tab of a small group to fetch its report before the
/// list rolls over; a report is a few kilobytes at most.
const KEPT: usize = 32;

impl CompileReports {
    /// Keep one export's warnings; the returned id names them.
    pub fn store(&self, warnings: Vec<String>) -> u64 {
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        inner.next_id += 1;
        let id = inner.next_id;
        inner.kept.push_back((id, warnings));
        while inner.kept.len() > KEPT {
            inner.kept.pop_front();
        }
        id
    }

    /// The warnings stored under `id`, if they have not rolled off yet.
    pub fn get(&self, id: u64) -> Option<Vec<String>> {
        let inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        inner
            .kept
            .iter()
            .find(|(kept_id, _)| *kept_id == id)
            .map(|(_, warnings)| warnings.clone())
    }
}

#[derive(Deserialize)]
pub struct ReportQuery {
    id: u64,
}

/// `GET /api/compile-report?id=<id>`: one recent export's warnings as a JSON
/// array, or 404 once it has rolled off.
pub async fn compile_report(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ReportQuery>,
) -> Response {
    match state.reports.get(q.id) {
        Some(warnings) => Json(warnings).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json("That compile report is no longer available; export again to see its warnings"),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::{download_name, CompileReports, KEPT};

    /// Reports are found by id until newer ones push them out (s3-5 repair).
    #[test]
    fn reports_are_kept_until_newer_ones_roll_them_off() {
        let reports = CompileReports::default();
        assert_eq!(reports.get(1), None);
        let first = reports.store(vec!["b.md#café was not found".to_string()]);
        assert_eq!(
            reports.get(first).as_deref(),
            Some(&["b.md#café was not found".to_string()][..])
        );
        let mut last = first;
        for n in 0..KEPT {
            last = reports.store(vec![format!("warning {n}")]);
        }
        assert_ne!(last, first);
        assert_eq!(reports.get(first), None, "the oldest report rolled off");
        assert_eq!(
            reports.get(last).unwrap(),
            vec![format!("warning {}", KEPT - 1)]
        );
        assert_eq!(reports.get(last + 1), None);
    }

    #[test]
    fn download_names_are_header_safe() {
        assert_eq!(download_name("My Novel", "docx"), "My Novel.docx");
        assert_eq!(download_name("Über: Draft/2", "pdf"), "ber- Draft-2.pdf");
        assert_eq!(download_name("   ", "html"), "Manuscript.html");
        assert_eq!(download_name("\"quoted\"", "odt"), "quoted.odt");
    }
}
