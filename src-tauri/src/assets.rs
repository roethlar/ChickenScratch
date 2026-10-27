//! The desktop's `chikn-asset` scheme: the picture a document shows
//! (`![alt](research/figure.png)`) is served from the project that is open
//! right now (PLAN_SCRIVENER_UPLOAD slice 3).
//!
//! Tauri's built-in asset protocol cannot do this safely: its scope is
//! additive, so a project closed and reopened in one session would either
//! lose its pictures or keep access after closing (review finding plan-4
//! R5-6). This handler instead consults [`OpenProject`], which the project
//! commands set on load and clear on close, at every request; the file
//! checks themselves are the services layer's, shared with the web shell.
//!
//! The UI builds URLs with `convertFileSrc(path, "chikn-asset")`, so the
//! request path is the percent-encoded project-relative path.

use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Mutex;

use chickenscratch_services::assets::{serve_project_asset, AssetRefusal};
use tauri::http::{header, Request, Response, StatusCode};

/// The project the desktop considers open: set by `load_project` and
/// `create_project`, cleared by `close_project`.
#[derive(Default)]
pub struct OpenProject(Mutex<Option<PathBuf>>);

impl OpenProject {
    pub fn set(&self, path: Option<PathBuf>) {
        if let Ok(mut slot) = self.0.lock() {
            *slot = path;
        }
    }

    pub fn current(&self) -> Option<PathBuf> {
        self.0.lock().ok().and_then(|slot| slot.clone())
    }
}

/// Answer one `chikn-asset://localhost/<encoded relative path>` request
/// (`http://chikn-asset.localhost/…` on Windows) against the open project.
pub fn respond(open: Option<PathBuf>, request: &Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let relative = percent_decode(request.uri().path().trim_start_matches('/'));
    let Some(root) = open else {
        return refusal(StatusCode::FORBIDDEN, "No project is open");
    };
    match serve_project_asset(&root, &relative) {
        Ok(asset) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, asset.content_type)
            .header(header::CACHE_CONTROL, "no-store")
            .body(Cow::Owned(asset.bytes))
            .unwrap_or_else(|_| refusal(StatusCode::INTERNAL_SERVER_ERROR, "response")),
        Err(AssetRefusal::BadPath) => refusal(
            StatusCode::BAD_REQUEST,
            "Not a file path inside the project",
        ),
        Err(AssetRefusal::Outside) => {
            refusal(StatusCode::FORBIDDEN, "That file is outside the project")
        }
        Err(AssetRefusal::NotFound) => {
            refusal(StatusCode::NOT_FOUND, "No such file in the project")
        }
        Err(AssetRefusal::TooLarge) => refusal(
            StatusCode::PAYLOAD_TOO_LARGE,
            "That file is too large to show",
        ),
    }
}

fn refusal(status: StatusCode, text: &'static str) -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Cow::Borrowed(text.as_bytes()))
        .expect("a static refusal response builds")
}

/// `%XX` sequences decoded; anything malformed is kept as written.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| s.to_string())
}

fn hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn project() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path().join("Novel.chikn");
        fs::create_dir_all(root.join("research")).unwrap();
        fs::write(root.join("research").join("figure.png"), b"\x89PNG").unwrap();
        (dir, root)
    }

    fn request(path: &str) -> Request<Vec<u8>> {
        Request::builder()
            .uri(format!("chikn-asset://localhost/{path}"))
            .body(Vec::new())
            .unwrap()
    }

    /// Open, close, reopen and switch: the handler follows the shared state
    /// at every request (plan-4 R5-6).
    #[test]
    fn follows_the_open_project_at_request_time() {
        let (dir, a) = project();
        let b = dir.path().join("Other.chikn");
        fs::create_dir_all(b.join("research")).unwrap();
        fs::write(b.join("research").join("figure.png"), b"B").unwrap();
        let state = OpenProject::default();

        assert_eq!(
            respond(state.current(), &request("research%2Ffigure.png")).status(),
            StatusCode::FORBIDDEN
        );
        state.set(Some(a.clone()));
        let res = respond(state.current(), &request("research%2Ffigure.png"));
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.body().as_ref(), b"\x89PNG");
        assert_eq!(res.headers()[header::CONTENT_TYPE], "image/png");
        assert_eq!(
            respond(state.current(), &request("..%2Fsecret.txt")).status(),
            StatusCode::BAD_REQUEST
        );
        state.set(None);
        assert_eq!(
            respond(state.current(), &request("research%2Ffigure.png")).status(),
            StatusCode::FORBIDDEN
        );
        state.set(Some(b));
        assert_eq!(
            respond(state.current(), &request("research%2Ffigure.png"))
                .body()
                .as_ref(),
            b"B"
        );
        state.set(Some(a));
        assert_eq!(
            respond(state.current(), &request("research%2Ffigure.png"))
                .body()
                .as_ref(),
            b"\x89PNG"
        );
        assert_eq!(
            respond(state.current(), &request("research%2Fmissing.png")).status(),
            StatusCode::NOT_FOUND
        );
    }
}
