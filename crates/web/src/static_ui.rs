//! Serves the built React UI (`ui/dist`) with the same posture as the Tauri
//! webview's CSP: same-origin everything, inline styles allowed (TipTap and
//! Tailwind need them), no frames, no objects.

use std::path::Path;

use axum::http::{header, HeaderName, HeaderValue};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;

pub const CSP: &str = "default-src 'self'; connect-src 'self'; img-src 'self' data:; \
font-src 'self' data:; style-src 'self' 'unsafe-inline'; script-src 'self'; \
object-src 'none'; base-uri 'self'; frame-ancestors 'none'";

/// Static files with `index.html` as the not-found fallback (the app has no
/// client-side routes; `/` and any stray path land on the page).
pub fn service(ui_dir: &Path) -> ServeDir<ServeFile> {
    ServeDir::new(ui_dir).fallback(ServeFile::new(ui_dir.join("index.html")))
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
