//! HTTP contract tests for `chikn-web` (plan slice 3a): the
//! projects-root gate, the Tauri-identical command contract, static UI.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

use chickenscratch_web::static_ui::UiSource;
use chickenscratch_web::{build_router, AppState};

struct Harness {
    _tmp: tempfile::TempDir,
    root: std::path::PathBuf,
    outside: std::path::PathBuf,
    app: Router,
}

fn harness() -> Harness {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path().join("root");
    let outside = tmp.path().join("outside");
    let ui = tmp.path().join("ui");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&outside).unwrap();
    fs::create_dir_all(&ui).unwrap();
    fs::write(ui.join("index.html"), "<!doctype html><title>stub</title>").unwrap();
    let state = AppState::new(&root, None, UiSource::Dir(ui)).unwrap();
    Harness {
        root: state.root.clone(),
        outside: outside.canonicalize().unwrap(),
        app: build_router(Arc::new(state)),
        _tmp: tmp,
    }
}

/// Like `harness()`, with a backup folder beside the root (not inside it).
fn harness_with_backup() -> (Harness, std::path::PathBuf) {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path().join("root");
    let outside = tmp.path().join("outside");
    let backup = tmp.path().join("backups");
    let ui = tmp.path().join("ui");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&outside).unwrap();
    fs::create_dir_all(&ui).unwrap();
    fs::write(ui.join("index.html"), "<!doctype html><title>stub</title>").unwrap();
    // `backup` does not exist yet: the server creates it.
    let state = AppState::new(&root, Some(&backup), UiSource::Dir(ui)).unwrap();
    let backup = state.backup.clone().unwrap();
    (
        Harness {
            root: state.root.clone(),
            outside: outside.canonicalize().unwrap(),
            app: build_router(Arc::new(state)),
            _tmp: tmp,
        },
        backup,
    )
}

async fn call(app: &Router, req: Request<Body>) -> (StatusCode, Value, axum::http::HeaderMap) {
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()))
    };
    (status, body, headers)
}

fn cmd(name: &str, args: Value) -> Request<Body> {
    Request::post(format!("/api/cmd/{name}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(args.to_string()))
        .unwrap()
}

async fn ok(app: &Router, name: &str, args: Value) -> Value {
    let (status, body, _) = call(app, cmd(name, args)).await;
    assert_eq!(status, StatusCode::OK, "{name} failed: {body}");
    body
}

fn path_str(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

#[tokio::test]
async fn unknown_command_is_404_with_a_plain_message() {
    let h = harness();
    let (status, body, _) = call(&h.app, cmd("no_such_command", json!({}))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, json!("Unknown command: no_such_command"));
}

#[tokio::test]
async fn create_load_edit_round_trip_matches_the_tauri_contract() {
    let h = harness();

    // create_project(name, path) — `path` is the parent folder.
    let project = ok(
        &h.app,
        "create_project",
        json!({ "name": "Scratch", "path": path_str(&h.root) }),
    )
    .await;
    let project_path = project["path"].as_str().unwrap().to_string();
    assert!(Path::new(&project_path).join("project.yaml").is_file());
    assert!(project_path.starts_with(&path_str(&h.root)));

    let loaded = ok(&h.app, "load_project", json!({ "path": project_path })).await;
    assert_eq!(loaded["read_only"], json!(false));
    assert_eq!(loaded["project"]["name"], json!("Scratch"));

    let with_doc = ok(
        &h.app,
        "create_document",
        json!({ "projectPath": project_path, "name": "First note", "parentId": null }),
    )
    .await;
    let doc_id = with_doc["documents"]
        .as_object()
        .unwrap()
        .values()
        .find(|d| d["name"] == json!("First note"))
        .map(|d| d["id"].as_str().unwrap().to_string())
        .expect("new document present");

    // Unit command resolves to null, like Tauri.
    let unit = ok(&h.app, "update_document_content", json!({ "projectPath": project_path, "docId": doc_id, "content": "Hello from the browser" })).await;
    assert_eq!(unit, Value::Null);

    let doc = ok(
        &h.app,
        "get_document",
        json!({ "projectPath": project_path, "docId": doc_id }),
    )
    .await;
    assert_eq!(doc["content"], json!("Hello from the browser"));

    // Unknown document id: Option<Document> → null, not an error.
    let missing = ok(
        &h.app,
        "get_document",
        json!({ "projectPath": project_path, "docId": "nope" }),
    )
    .await;
    assert_eq!(missing, Value::Null);

    // A ChiknError travels as 422 + its Display string, byte for byte.
    let (status, body, _) = call(
        &h.app,
        cmd(
            "update_document_content",
            json!({ "projectPath": project_path, "docId": "nope", "content": "x" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let expected =
        chickenscratch_services::ChiknError::NotFound("Document not found: nope".to_string())
            .to_string();
    assert_eq!(body, json!(expected));

    // Camel-case keys are mandatory; a missing required arg is a 400.
    let (status, body, _) = call(
        &h.app,
        cmd("get_document", json!({ "project_path": project_path })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body.as_str().unwrap().starts_with("Invalid arguments"),
        "{body}"
    );

    // Revisions work through the same door.
    let rev = ok(
        &h.app,
        "save_revision",
        json!({ "projectPath": project_path, "message": "First save" }),
    )
    .await;
    assert_eq!(rev["message"], json!("First save"));
    let list = ok(
        &h.app,
        "list_revisions",
        json!({ "projectPath": project_path }),
    )
    .await;
    assert!(list.as_array().unwrap().len() >= 2, "{list}");

    // The project list sees it; a plain folder is ignored.
    fs::create_dir_all(h.root.join("plain")).unwrap();
    let (status, projects, _) = call(
        &h.app,
        Request::get("/api/projects").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        projects,
        json!([{ "name": "Scratch", "path": project_path }])
    );
}

#[tokio::test]
async fn paths_outside_the_root_are_refused_and_nothing_is_created() {
    let h = harness();
    let before = fs::read_dir(&h.outside).unwrap().count();

    // Absolute path elsewhere.
    let (status, body, _) = call(
        &h.app,
        cmd(
            "create_project",
            json!({ "name": "Evil", "path": path_str(&h.outside) }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body, json!("Path is outside the projects folder"));

    // `..` climbing out of the root.
    let dotdot = h.root.join("..").join("outside");
    let (status, _, _) = call(
        &h.app,
        cmd(
            "create_project",
            json!({ "name": "Evil", "path": path_str(&dotdot) }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Relative path.
    let (status, _, _) = call(
        &h.app,
        cmd("load_project", json!({ "path": "relative/Notes.chikn" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // A symlink inside the root that points outside.
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&h.outside, h.root.join("escape")).unwrap();
        let (status, _, _) = call(
            &h.app,
            cmd(
                "create_project",
                json!({ "name": "Evil", "path": path_str(&h.root.join("escape")) }),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    assert_eq!(
        fs::read_dir(&h.outside).unwrap().count(),
        before,
        "nothing may be created outside the root"
    );
    assert!(!h.outside.join("Evil.chikn").exists());
}

#[tokio::test]
async fn save_project_cannot_be_pointed_outside_the_root() {
    let h = harness();
    let project = ok(
        &h.app,
        "create_project",
        json!({ "name": "Scratch", "path": path_str(&h.root) }),
    )
    .await;
    let mut hijacked = project.clone();
    hijacked["path"] = json!(path_str(&h.outside));
    let (status, _, _) = call(&h.app, cmd("save_project", json!({ "project": hijacked }))).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(!h.outside.join("project.yaml").exists());
}

#[tokio::test]
async fn desktop_only_commands_say_so() {
    let h = harness();
    let (status, body, _) = call(&h.app, cmd("pick_scriv_folder", json!({}))).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        body.as_str()
            .unwrap()
            .contains("not available in the web version"),
        "{body}"
    );
    // AI streaming is wired through the SSE sink: never "unknown command".
    // With AI disabled or unconfigured it refuses with the settings message.
    let (status, body, _) = call(
        &h.app,
        cmd(
            "ai_transform_stream",
            json!({ "content": "x", "operation": "polish", "requestId": "r1" }),
        ),
    )
    .await;
    assert_ne!(status, StatusCode::NOT_FOUND);
    assert!(
        status == StatusCode::OK || body.as_str().is_some_and(|s| s.contains("AI")),
        "{status} {body}"
    );
}

#[tokio::test]
async fn every_tauri_command_name_is_in_the_table() {
    let h = harness();
    for name in chickenscratch_web::commands::COMMAND_NAMES {
        let (status, _, _) = call(&h.app, cmd(name, json!({}))).await;
        assert_ne!(
            status,
            StatusCode::NOT_FOUND,
            "{name} is missing from the command table"
        );
    }
    assert_eq!(chickenscratch_web::commands::COMMAND_NAMES.len(), 75);
}

#[tokio::test]
async fn ui_is_served_with_security_headers_and_info_reports_the_root() {
    let h = harness();
    let (status, body, headers) =
        call(&h.app, Request::get("/").body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.as_str().unwrap().contains("<title>stub</title>"));
    assert!(headers[header::CONTENT_SECURITY_POLICY]
        .to_str()
        .unwrap()
        .contains("frame-ancestors 'none'"));
    assert_eq!(headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
    assert!(headers.get(header::ACCESS_CONTROL_ALLOW_ORIGIN).is_none());

    // Unknown non-API paths fall back to the page too.
    let (status, _, _) = call(
        &h.app,
        Request::get("/some/where").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, info, _) = call(
        &h.app,
        Request::get("/api/info").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(info["root"], json!(path_str(&h.root)));
    assert_eq!(info["events"], json!(true));

    // The event channel answers as a live SSE stream (the body never ends,
    // so only status and headers are checked).
    let res = h
        .app
        .clone()
        .oneshot(Request::get("/api/events").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(res.headers()[header::CONTENT_TYPE]
        .to_str()
        .unwrap()
        .starts_with("text/event-stream"));
}

#[tokio::test]
async fn backup_folder_is_a_second_browsable_root_but_not_a_project_root() {
    let (h, backup) = harness_with_backup();
    assert!(backup.is_dir(), "server creates the backup folder");
    fs::create_dir_all(backup.join("nightly")).unwrap();

    let (status, info, _) = call(
        &h.app,
        Request::get("/api/info").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(info["roots"][0]["label"], json!("Projects"));
    assert_eq!(info["roots"][1]["label"], json!("Backups"));
    assert_eq!(info["roots"][1]["path"], json!(path_str(&backup)));

    // Browsing inside the backup folder works and tops out at that root.
    let (status, listing, _) = call(
        &h.app,
        Request::get(format!("/api/fs/dirs?path={}", path_str(&backup)))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(listing["parent"], Value::Null);
    assert_eq!(listing["dirs"][0]["name"], json!("nightly"));
    assert_eq!(listing["roots"][1]["label"], json!("Backups"));
    let (status, nightly, _) = call(
        &h.app,
        Request::get(format!(
            "/api/fs/dirs?path={}",
            path_str(&backup.join("nightly"))
        ))
        .body(Body::empty())
        .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(nightly["parent"], json!(path_str(&backup)));

    // Still nothing outside either root.
    let (status, _, _) = call(
        &h.app,
        Request::get(format!("/api/fs/dirs?path={}", path_str(&h.outside)))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // And the backup folder is not a place projects may be created.
    let (status, _, _) = call(
        &h.app,
        cmd(
            "create_project",
            json!({ "name": "Sneaky", "path": path_str(&backup) }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(!backup.join("Sneaky.chikn").exists());
}

#[tokio::test]
async fn export_validates_format_and_reports_a_missing_pandoc_plainly() {
    let h = harness();
    let project = ok(
        &h.app,
        "create_project",
        json!({ "name": "Scratch", "path": path_str(&h.root) }),
    )
    .await;
    let project_path = project["path"].as_str().unwrap().to_string();
    // Something to compile: one document inside the Manuscript folder.
    let manuscript_id = project["hierarchy"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["name"] == json!("Manuscript"))
        .and_then(|n| n["id"].as_str())
        .expect("manuscript folder")
        .to_string();
    let with_doc = ok(
        &h.app,
        "create_document",
        json!({ "projectPath": project_path, "name": "Opening", "parentId": manuscript_id }),
    )
    .await;
    let doc_id = with_doc["documents"]
        .as_object()
        .unwrap()
        .values()
        .find(|d| d["name"] == json!("Opening"))
        .map(|d| d["id"].as_str().unwrap().to_string())
        .unwrap();
    ok(
        &h.app,
        "update_document_content",
        json!({ "projectPath": project_path, "docId": doc_id, "content": "Export me." }),
    )
    .await;

    let (status, body, _) = call(
        &h.app,
        Request::post("/api/export")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({ "projectPath": project_path, "format": "exe" }).to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body, json!("Unknown export format: exe"));

    // Outside the root: gated before anything is compiled.
    let (status, _, _) = call(
        &h.app,
        Request::post("/api/export")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({ "projectPath": path_str(&h.outside), "format": "html" }).to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let res = h.app.clone().oneshot(Request::post("/api/export").header(header::CONTENT_TYPE, "application/json").body(Body::from(json!({ "projectPath": project_path, "format": "html", "title": "Scratch Notes" }).to_string())).unwrap()).await.unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    if status == StatusCode::OK {
        // Pandoc is installed on this machine: a real download comes back.
        assert_eq!(
            headers[header::CONTENT_DISPOSITION],
            "attachment; filename=\"Scratch Notes.html\""
        );
        assert!(headers[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("text/html"));
        assert!(!bytes.is_empty());
    } else {
        // No Pandoc: the same plain ChiknError string the desktop shows.
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            String::from_utf8_lossy(&bytes)
        );
        let msg: String = serde_json::from_slice(&bytes).unwrap();
        assert!(msg.contains("Pandoc"), "{msg}");
    }
    // Nothing was written next to or inside the project.
    assert!(!h.root.join("Scratch Notes.html").exists());
    assert!(!Path::new(&project_path).join("Scratch Notes.html").exists());
}

fn multipart(fields: &[(&str, Option<&str>, &[u8])]) -> (String, Vec<u8>) {
    let boundary = "----chiknboundary7MA4YWxkTrZu0gW";
    let mut body = Vec::new();
    for (name, file_name, data) in fields {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        match file_name {
            Some(f) => body.extend_from_slice(
                format!("Content-Disposition: form-data; name=\"{name}\"; filename=\"{f}\"\r\nContent-Type: application/octet-stream\r\n\r\n").as_bytes(),
            ),
            None => body.extend_from_slice(format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes()),
        }
        body.extend_from_slice(data);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}

#[tokio::test]
async fn import_upload_adds_the_file_as_a_document_and_leaves_no_copy_behind() {
    let h = harness();
    let project = ok(
        &h.app,
        "create_project",
        json!({ "name": "Scratch", "path": path_str(&h.root) }),
    )
    .await;
    let project_path = project["path"].as_str().unwrap().to_string();

    let (ct, body) = multipart(&[
        ("projectPath", None, project_path.as_bytes()),
        ("parentId", None, b""),
        (
            "file",
            Some("../../Shopping list.md"),
            b"# Shopping\n\n- eggs\n",
        ),
    ]);
    let (status, result, _) = call(
        &h.app,
        Request::post("/api/import-upload")
            .header(header::CONTENT_TYPE, ct)
            .body(Body::from(body))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{result}");
    let doc = result["documents"]
        .as_object()
        .unwrap()
        .values()
        .find(|d| d["name"] == json!("Shopping list"))
        .expect("imported document");
    assert_eq!(doc["content"], json!("# Shopping\n\n- eggs\n"));
    assert!(doc["path"].as_str().unwrap().starts_with("manuscript/"));
    // The upload was staged outside the root and is gone.
    assert!(!h.root.join("Shopping list.md").exists());
    assert!(!Path::new(&project_path).join("Shopping list.md").exists());

    // Outside the root: refused, nothing imported.
    let (ct, body) = multipart(&[
        ("projectPath", None, path_str(&h.outside).as_bytes()),
        ("file", Some("x.md"), b"x"),
    ]);
    let (status, _, _) = call(
        &h.app,
        Request::post("/api/import-upload")
            .header(header::CONTENT_TYPE, ct)
            .body(Body::from(body))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Missing file: a plain 400.
    let (ct, body) = multipart(&[("projectPath", None, project_path.as_bytes())]);
    let (status, msg, _) = call(
        &h.app,
        Request::post("/api/import-upload")
            .header(header::CONTENT_TYPE, ct)
            .body(Body::from(body))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(msg, json!("Invalid upload: missing file"));
}

#[tokio::test]
async fn folder_browser_stays_inside_the_root() {
    let h = harness();
    fs::create_dir_all(h.root.join("Research").join("Draft.scriv")).unwrap();
    fs::create_dir_all(h.root.join(".git")).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&h.outside, h.root.join("escape")).unwrap();

    let (status, listing, _) = call(
        &h.app,
        Request::get("/api/fs/dirs").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(listing["path"], json!(path_str(&h.root)));
    assert_eq!(listing["parent"], Value::Null);
    assert_eq!(
        listing["dirs"],
        json!([{ "name": "Research", "path": path_str(&h.root.join("Research")) }])
    );

    let (status, listing, _) = call(
        &h.app,
        Request::get(format!(
            "/api/fs/dirs?path={}",
            urlencode(&path_str(&h.root.join("Research")))
        ))
        .body(Body::empty())
        .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(listing["parent"], json!(path_str(&h.root)));
    assert_eq!(listing["dirs"][0]["name"], json!("Draft.scriv"));

    let (status, _, _) = call(
        &h.app,
        Request::get(format!(
            "/api/fs/dirs?path={}",
            urlencode(&path_str(&h.outside))
        ))
        .body(Body::empty())
        .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    #[cfg(unix)]
    {
        let (status, _, _) = call(
            &h.app,
            Request::get(format!(
                "/api/fs/dirs?path={}",
                urlencode(&path_str(&h.root.join("escape")))
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }
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

fn cmd_as(name: &str, args: Value, session: &str) -> Request<Body> {
    Request::post(format!("/api/cmd/{name}"))
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-cs-session", session)
        .body(Body::from(args.to_string()))
        .unwrap()
}

#[tokio::test]
async fn second_session_is_refused_until_it_takes_over() {
    let h = harness();
    let project = ok(
        &h.app,
        "create_project",
        json!({ "name": "Scratch", "path": path_str(&h.root) }),
    )
    .await;
    let project_path = project["path"].as_str().unwrap().to_string();
    let held = json!("This project is open in another browser tab or device. Take over?");

    // Tab A opens and writes.
    let (status, _, _) = call(
        &h.app,
        cmd_as("load_project", json!({ "path": project_path }), "tab-a"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) = call(
        &h.app,
        cmd_as(
            "save_revision",
            json!({ "projectPath": project_path, "message": "a" }),
            "tab-a",
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Tab B may read but not open-for-writing or write.
    let (status, _, _) = call(
        &h.app,
        cmd_as(
            "list_revisions",
            json!({ "projectPath": project_path }),
            "tab-b",
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, body, _) = call(
        &h.app,
        cmd_as("load_project", json!({ "path": project_path }), "tab-b"),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body, held);
    let (status, _, _) = call(
        &h.app,
        cmd_as(
            "save_revision",
            json!({ "projectPath": project_path, "message": "b" }),
            "tab-b",
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // A still writes; a client without a session id is not arbitrated.
    let (status, _, _) = call(
        &h.app,
        cmd_as(
            "save_revision",
            json!({ "projectPath": project_path, "message": "a2" }),
            "tab-a",
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) = call(
        &h.app,
        cmd("has_changes", json!({ "projectPath": project_path })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Another project is independent.
    let other = ok(
        &h.app,
        "create_project",
        json!({ "name": "Other", "path": path_str(&h.root) }),
    )
    .await;
    let (status, _, _) = call(
        &h.app,
        cmd_as("load_project", json!({ "path": other["path"] }), "tab-b"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // B takes over; now A is the one refused.
    let (status, _, _) = call(
        &h.app,
        Request::post("/api/lease/take")
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-cs-session", "tab-b")
            .body(Body::from(
                json!({ "projectPath": project_path }).to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) = call(
        &h.app,
        cmd_as(
            "save_revision",
            json!({ "projectPath": project_path, "message": "b2" }),
            "tab-b",
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, body, _) = call(
        &h.app,
        cmd_as(
            "update_document_content",
            json!({ "projectPath": project_path, "docId": "x", "content": "y" }),
            "tab-a",
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body, held);

    // Take-over needs a session id and stays inside the root.
    let (status, _, _) = call(
        &h.app,
        Request::post("/api/lease/take")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({ "projectPath": project_path }).to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _, _) = call(
        &h.app,
        Request::post("/api/lease/take")
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-cs-session", "tab-b")
            .body(Body::from(
                json!({ "projectPath": path_str(&h.outside) }).to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

fn lease_request(route: &str, project_path: &str, session: &str) -> Request<Body> {
    Request::post(route)
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-cs-session", session)
        .body(Body::from(
            json!({ "projectPath": project_path }).to_string(),
        ))
        .unwrap()
}

/// The Projects button hands the lease back (PLAN_RELOAD_SAFETY_MOBILE
/// slice 1): the next device writes at once instead of waiting out the
/// idle expiry, and a tab that does not hold the project cannot release it.
#[tokio::test]
async fn releasing_the_lease_frees_the_project_for_the_next_session() {
    let h = harness();
    let project = ok(
        &h.app,
        "create_project",
        json!({ "name": "Scratch", "path": path_str(&h.root) }),
    )
    .await;
    let project_path = project["path"].as_str().unwrap().to_string();

    // A holds the project; B is refused.
    let (status, _, _) = call(
        &h.app,
        cmd_as("load_project", json!({ "path": project_path }), "tab-a"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) = call(
        &h.app,
        cmd_as("load_project", json!({ "path": project_path }), "tab-b"),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // B cannot release what A holds.
    let (status, _, _) = call(
        &h.app,
        lease_request("/api/lease/release", &project_path, "tab-b"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) = call(
        &h.app,
        cmd_as("load_project", json!({ "path": project_path }), "tab-b"),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // A leaves; B opens immediately.
    let (status, _, _) = call(
        &h.app,
        lease_request("/api/lease/release", &project_path, "tab-a"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) = call(
        &h.app,
        cmd_as("load_project", json!({ "path": project_path }), "tab-b"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Release needs a session id and stays inside the root.
    let (status, _, _) = call(
        &h.app,
        Request::post("/api/lease/release")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({ "projectPath": project_path }).to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _, _) = call(
        &h.app,
        lease_request("/api/lease/release", &path_str(&h.outside), "tab-a"),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
