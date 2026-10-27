//! `GET /api/asset`: a project's own file for the pictures documents show
//! (PLAN_SCRIVENER_UPLOAD slice 3). The project path is gated like every
//! other; the relative path is refused when it would leave the project.

use std::fs;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

use chickenscratch_web::static_ui::UiSource;
use chickenscratch_web::{build_router, AppState};

struct Harness {
    _tmp: tempfile::TempDir,
    project: std::path::PathBuf,
    outside: std::path::PathBuf,
    app: Router,
}

fn harness() -> Harness {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path().join("root");
    let outside = tmp.path().join("outside");
    let ui = tmp.path().join("ui");
    fs::create_dir_all(root.join("Novel.chikn").join("research")).unwrap();
    fs::create_dir_all(&outside).unwrap();
    fs::create_dir_all(&ui).unwrap();
    fs::write(ui.join("index.html"), "<!doctype html><title>stub</title>").unwrap();
    fs::write(
        root.join("Novel.chikn").join("research").join("figure.png"),
        b"\x89PNG-bytes",
    )
    .unwrap();
    fs::write(outside.join("secret.txt"), b"outside").unwrap();
    let state = AppState::new(&root, None, UiSource::Dir(ui)).unwrap();
    Harness {
        project: state.root.join("Novel.chikn"),
        outside: outside.canonicalize().unwrap(),
        app: build_router(Arc::new(state)),
        _tmp: tmp,
    }
}

async fn get(app: &Router, project: &str, path: &str) -> (StatusCode, Vec<u8>, Option<String>) {
    let uri = format!(
        "/api/asset?project={}&path={}",
        urlencode(project),
        urlencode(path)
    );
    let res = app
        .clone()
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let ctype = res
        .headers()
        .get(header::CONTENT_TYPE)
        .map(|v| v.to_str().unwrap().to_string());
    let bytes = res.into_body().collect().await.unwrap().to_bytes().to_vec();
    (status, bytes, ctype)
}

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[tokio::test]
async fn serves_a_file_inside_the_project_with_its_type() {
    let h = harness();
    let (status, bytes, ctype) =
        get(&h.app, &h.project.to_string_lossy(), "research/figure.png").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(bytes, b"\x89PNG-bytes");
    assert_eq!(ctype.as_deref(), Some("image/png"));
}

#[tokio::test]
async fn refuses_paths_that_leave_the_project_or_the_root() {
    let h = harness();
    let project = h.project.to_string_lossy().into_owned();
    let (status, _, _) = get(&h.app, &project, "../../outside/secret.txt").await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "traversal is not a file path"
    );
    let (status, _, _) = get(&h.app, &project, "/etc/passwd").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _, _) = get(&h.app, &project, "research/missing.png").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    // A project path outside the projects root never gets as far as the file.
    let (status, bytes, _) = get(&h.app, &h.outside.to_string_lossy(), "secret.txt").await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "{}",
        String::from_utf8_lossy(&bytes)
    );
    // A relative project path is refused by the gate too.
    let (status, _, _) = get(&h.app, "Novel.chikn", "research/figure.png").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn close_project_is_accepted_as_a_no_op() {
    let h = harness();
    let res = h
        .app
        .clone()
        .oneshot(
            Request::post("/api/cmd/close_project")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}
