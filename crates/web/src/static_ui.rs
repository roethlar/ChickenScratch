//! Serves the built React UI with the same posture as the Tauri webview's
//! CSP: same-origin everything, inline styles allowed (TipTap and Tailwind
//! need them), no frames, no objects.
//!
//! The UI comes either from a folder (`--ui-dir`, default `ui/dist`) or, when
//! the crate is built with `--features embed-ui`, from assets compiled into
//! the binary so a single file can be copied to the host machine.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::http::{header, HeaderName, HeaderValue};
use axum::Router;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::state::AppState;

pub const CSP: &str = "default-src 'self'; connect-src 'self'; img-src 'self' data:; \
font-src 'self' data:; style-src 'self' 'unsafe-inline'; script-src 'self'; \
object-src 'none'; base-uri 'self'; frame-ancestors 'none'";

/// Where the page and its assets come from.
#[derive(Debug, Clone)]
pub enum UiSource {
    /// A folder holding `index.html` and `assets/`.
    Dir(PathBuf),
    /// Assets compiled into the binary (`--features embed-ui`).
    #[cfg(feature = "embed-ui")]
    Embedded,
}

impl fmt::Display for UiSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UiSource::Dir(dir) => write!(f, "{}", dir.display()),
            #[cfg(feature = "embed-ui")]
            UiSource::Embedded => f.write_str("embedded in the binary"),
        }
    }
}

/// Static files with `index.html` as the fallback (the app has no
/// client-side routes; `/` and any stray path land on the page).
pub fn service(ui_dir: &Path) -> ServeDir<ServeFile> {
    ServeDir::new(ui_dir).fallback(ServeFile::new(ui_dir.join("index.html")))
}

/// Attach the UI as the router's fallback, from whichever source is configured.
pub fn mount(router: Router<Arc<AppState>>, ui: &UiSource) -> Router<Arc<AppState>> {
    match ui {
        UiSource::Dir(dir) => router.fallback_service(service(dir)),
        #[cfg(feature = "embed-ui")]
        UiSource::Embedded => router.fallback(embedded::serve),
    }
}

pub fn security_headers() -> [SetResponseHeaderLayer<HeaderValue>; 3] {
    [
        SetResponseHeaderLayer::overriding(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(CSP),
        ),
        SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ),
        SetResponseHeaderLayer::overriding(
            HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("no-referrer"),
        ),
    ]
}

#[cfg(feature = "embed-ui")]
mod embedded {
    use axum::{
        http::{header, HeaderValue, StatusCode, Uri},
        response::{IntoResponse, Response},
    };
    use rust_embed::RustEmbed;

    /// `ui/dist` at build time. Build the UI first (`cd ui && npm run build`).
    #[derive(RustEmbed)]
    #[folder = "../../ui/dist"]
    struct Assets;

    /// Content types for what Vite emits; anything else downloads as bytes.
    fn mime_for(path: &str) -> &'static str {
        match path.rsplit('.').next() {
            Some("html") => "text/html; charset=utf-8",
            Some("js") => "text/javascript; charset=utf-8",
            Some("css") => "text/css; charset=utf-8",
            Some("json") | Some("map") => "application/json",
            Some("webmanifest") => "application/manifest+json",
            Some("svg") => "image/svg+xml",
            Some("png") => "image/png",
            Some("ico") => "image/x-icon",
            Some("woff2") => "font/woff2",
            Some("woff") => "font/woff",
            Some("txt") => "text/plain; charset=utf-8",
            _ => "application/octet-stream",
        }
    }

    fn asset_response(path: &str) -> Option<Response> {
        let file = Assets::get(path)?;
        let mime = mime_for(path);
        Some(
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, HeaderValue::from_static(mime))],
                file.data.into_owned(),
            )
                .into_response(),
        )
    }

    pub async fn serve(uri: Uri) -> Response {
        let path = uri.path().trim_start_matches('/');
        let path = if path.is_empty() { "index.html" } else { path };
        asset_response(path)
            .or_else(|| asset_response("index.html"))
            .unwrap_or_else(|| {
                (StatusCode::NOT_FOUND, "UI assets were not embedded").into_response()
            })
    }
}
