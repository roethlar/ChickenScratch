//! Single-writer lease per project (plan slice 7).
//!
//! The UI assumes one instance per project: whole-document saves and the
//! operation barrier are per tab. When the same project is open in two
//! browser tabs or on two devices, the second writer would silently overwrite
//! the first. So every project-mutating command carries the tab's session id
//! (`X-CS-Session`, random per tab, kept in `sessionStorage`); the first
//! session to write holds the lease, other sessions get `409` with a
//! take-over offer, and `POST /api/lease/take` hands the lease over
//! explicitly. A lease nobody has used for [`LEASE_TTL`] is free again.
//!
//! Requests without the header (scripts, curl) are not arbitrated: the lease
//! is a UX safety net for the browser, not an access control.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::{
    body::{to_bytes, Body},
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;

use crate::app::MAX_BODY_BYTES;
use crate::commands::CmdError;
use crate::gate::PathArg;
use crate::state::AppState;

pub const SESSION_HEADER: &str = "x-cs-session";
pub const LEASE_TTL: Duration = Duration::from_secs(120);
pub const LEASE_HELD_MESSAGE: &str =
    "This project is open in another browser tab or device. Take over?";

#[derive(Debug, Clone)]
struct Lease {
    session: String,
    last_seen: Instant,
}

#[derive(Default)]
pub struct Leases {
    inner: Mutex<HashMap<PathBuf, Lease>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct LeaseHeld;

impl Leases {
    /// Claim or refresh the lease for `session`; `Err` when another live
    /// session holds it.
    pub fn claim(&self, project: &Path, session: &str, now: Instant) -> Result<(), LeaseHeld> {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        match map.get_mut(project) {
            Some(lease)
                if lease.session != session && now.duration_since(lease.last_seen) < LEASE_TTL =>
            {
                Err(LeaseHeld)
            }
            Some(lease) => {
                lease.session = session.to_string();
                lease.last_seen = now;
                Ok(())
            }
            None => {
                map.insert(
                    project.to_path_buf(),
                    Lease {
                        session: session.to_string(),
                        last_seen: now,
                    },
                );
                Ok(())
            }
        }
    }

    /// Hand the lease to `session` regardless of who holds it.
    pub fn take_over(&self, project: &Path, session: &str, now: Instant) {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        map.insert(
            project.to_path_buf(),
            Lease {
                session: session.to_string(),
                last_seen: now,
            },
        );
    }

    /// Give the project up at once when `session` holds it (the Projects
    /// button, plan PLAN_RELOAD_SAFETY_MOBILE slice 1), so another tab or
    /// device can write without waiting out [`LEASE_TTL`]. A session that
    /// does not hold the lease changes nothing. Returns whether it was held.
    pub fn release(&self, project: &Path, session: &str) -> bool {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        match map.get(project) {
            Some(lease) if lease.session == session => {
                map.remove(project);
                true
            }
            _ => false,
        }
    }

    pub fn holder(&self, project: &Path) -> Option<String> {
        let map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        map.get(project).map(|l| l.session.clone())
    }
}

/// Which body field names the project for each arbitrated command.
/// Commands that create a new project, pure reads, and settings are not
/// arbitrated.
pub fn project_field(command: &str) -> Option<&'static [&'static str]> {
    const PROJECT_PATH: &[&str] = &["projectPath"];
    match command {
        "load_project" => Some(&["path"]),
        "save_project" => Some(&["project", "path"]),
        "update_document_content"
        | "add_comment"
        | "update_comment"
        | "delete_comment"
        | "update_document_metadata"
        | "rename_node"
        | "link_documents"
        | "create_document"
        | "create_entity"
        | "create_folder"
        | "delete_node"
        | "move_node"
        | "update_project_metadata"
        | "save_revision"
        | "restore_revision"
        | "create_draft"
        | "switch_draft"
        | "merge_draft"
        | "push_backup"
        | "manual_backup"
        | "sync_push"
        | "sync_fetch"
        | "sync_pull"
        | "sync_abort_pull"
        | "complete_merge"
        | "force_resolve_merge"
        | "sync_pull_force"
        | "restore_document"
        | "backup_on_close"
        | "import_file"
        | "record_daily_words"
        | "create_from_template"
        | "create_thread"
        | "update_thread"
        | "delete_thread" => Some(PROJECT_PATH),
        _ => None,
    }
}

fn session_of(headers: &HeaderMap) -> Option<String> {
    headers
        .get(SESSION_HEADER)?
        .to_str()
        .ok()
        .map(str::trim)
        .filter(|s| !s.is_empty() && s.len() <= 128)
        .map(str::to_string)
}

fn lookup<'a>(value: &'a serde_json::Value, path: &[&str]) -> Option<&'a str> {
    let mut cur = value;
    for key in path {
        cur = cur.get(key)?;
    }
    cur.as_str()
}

/// Middleware on `/api/cmd/{name}`: arbitrate project-mutating commands that
/// carry a session id. The body is buffered to read the project path and
/// handed on untouched.
pub async fn guard(State(state): State<Arc<AppState>>, req: Request, next: Next) -> Response {
    let Some(session) = session_of(req.headers()) else {
        return next.run(req).await;
    };
    let Some(command) = req
        .uri()
        .path()
        .rsplit_once("/cmd/")
        .map(|(_, name)| name.to_string())
    else {
        return next.run(req).await;
    };
    let Some(field) = project_field(&command) else {
        return next.run(req).await;
    };

    let (parts, body) = req.into_parts();
    let bytes = match to_bytes(body, MAX_BODY_BYTES).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return (StatusCode::PAYLOAD_TOO_LARGE, Json("Request too large")).into_response()
        }
    };
    let raw_path = serde_json::from_slice::<serde_json::Value>(&bytes)
        .ok()
        .and_then(|v| lookup(&v, field).map(str::to_string));
    if let Some(raw) = raw_path {
        // The command's own gate reports bad paths; only arbitrate real ones.
        if let Ok(project) = state.gate.existing(&raw) {
            if state
                .leases
                .claim(&project, &session, Instant::now())
                .is_err()
            {
                return (StatusCode::CONFLICT, Json(LEASE_HELD_MESSAGE)).into_response();
            }
        }
    }
    next.run(Request::from_parts(parts, Body::from(bytes)))
        .await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TakeArgs {
    project_path: PathArg,
}

/// `POST /api/lease/take`: this session becomes the writer for the project.
pub async fn take(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(args): Json<TakeArgs>,
) -> Result<Json<()>, CmdError> {
    let session = session_of(&headers)
        .ok_or_else(|| CmdError::BadRequest("Missing session id".to_string()))?;
    let project = state
        .gate
        .existing(&args.project_path.existing(&state.gate)?)?;
    state.leases.take_over(&project, &session, Instant::now());
    Ok(Json(()))
}

/// `POST /api/lease/release`: this session stops being the writer for the
/// project (a no-op when it is not). Same body as `take`.
pub async fn release(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(args): Json<TakeArgs>,
) -> Result<Json<()>, CmdError> {
    let session = session_of(&headers)
        .ok_or_else(|| CmdError::BadRequest("Missing session id".to_string()))?;
    let project = state
        .gate
        .existing(&args.project_path.existing(&state.gate)?)?;
    state.leases.release(&project, &session);
    Ok(Json(()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_writer_holds_until_expiry_or_take_over() {
        let leases = Leases::default();
        let p = Path::new("/srv/root/Notes.chikn");
        let t0 = Instant::now();
        assert_eq!(leases.claim(p, "a", t0), Ok(()));
        assert_eq!(leases.claim(p, "a", t0 + Duration::from_secs(10)), Ok(()));
        assert_eq!(
            leases.claim(p, "b", t0 + Duration::from_secs(20)),
            Err(LeaseHeld)
        );
        // a's last use was t0+10; b may claim once the TTL has passed.
        assert_eq!(
            leases.claim(p, "b", t0 + Duration::from_secs(10) + LEASE_TTL),
            Ok(())
        );
        assert_eq!(leases.holder(p), Some("b".to_string()));
        // Take-over is unconditional and flips the holder.
        leases.take_over(p, "a", t0 + Duration::from_secs(10) + LEASE_TTL);
        assert_eq!(leases.holder(p), Some("a".to_string()));
        assert_eq!(
            leases.claim(p, "b", t0 + Duration::from_secs(11) + LEASE_TTL),
            Err(LeaseHeld)
        );
    }

    #[test]
    fn release_frees_the_project_only_for_its_holder() {
        let leases = Leases::default();
        let p = Path::new("/srv/root/Notes.chikn");
        let t0 = Instant::now();
        assert_eq!(leases.claim(p, "a", t0), Ok(()));
        // A non-holder's release changes nothing: a is still the writer.
        assert!(!leases.release(p, "b"));
        assert_eq!(leases.holder(p), Some("a".to_string()));
        assert_eq!(
            leases.claim(p, "b", t0 + Duration::from_secs(1)),
            Err(LeaseHeld)
        );
        // The holder's release frees it at once, no TTL wait.
        assert!(leases.release(p, "a"));
        assert_eq!(leases.holder(p), None);
        assert_eq!(leases.claim(p, "b", t0 + Duration::from_secs(2)), Ok(()));
        // Releasing a project nobody holds is harmless.
        assert!(!leases.release(Path::new("/srv/root/Other.chikn"), "a"));
    }

    #[test]
    fn only_project_mutating_commands_are_arbitrated() {
        assert_eq!(
            project_field("update_document_content"),
            Some(&["projectPath"][..])
        );
        assert_eq!(project_field("load_project"), Some(&["path"][..]));
        assert_eq!(
            project_field("save_project"),
            Some(&["project", "path"][..])
        );
        assert_eq!(project_field("get_document"), None);
        assert_eq!(project_field("create_project"), None);
        assert_eq!(project_field("get_app_settings"), None);
    }
}
