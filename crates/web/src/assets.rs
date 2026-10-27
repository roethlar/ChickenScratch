//! `GET /api/asset?project=…&path=…`: a project's own file, for the picture
//! a document shows (PLAN_SCRIVENER_UPLOAD slice 3). The project path goes
//! through the projects-root gate like every other path the browser sends;
//! the relative path is validated and read by the services layer, which
//! refuses traversal, hidden entries and symlinks.

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use chickenscratch_services::assets::{serve_project_asset, AssetRefusal};
use serde::Deserialize;

use crate::gate::PathArg;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct AssetQuery {
    project: PathArg,
    path: String,
}

pub async fn asset(State(state): State<Arc<AppState>>, Query(q): Query<AssetQuery>) -> Response {
    let project = match q.project.existing(&state.gate) {
        Ok(p) => p,
        Err(e) => return (e.status(), e.to_string()).into_response(),
    };
    let relative = q.path;
    let served = tokio::task::spawn_blocking(move || {
        serve_project_asset(std::path::Path::new(&project), &relative)
    })
    .await;
    match served {
        Ok(Ok(asset)) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, asset.content_type)
            .header(header::CACHE_CONTROL, "no-store")
            .body(Body::from(asset.bytes))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response()),
        Ok(Err(AssetRefusal::BadPath)) => (
            StatusCode::BAD_REQUEST,
            "Not a file path inside the project",
        )
            .into_response(),
        Ok(Err(AssetRefusal::Outside)) => {
            (StatusCode::FORBIDDEN, "That file is outside the project").into_response()
        }
        Ok(Err(AssetRefusal::NotFound)) => {
            (StatusCode::NOT_FOUND, "No such file in the project").into_response()
        }
        Ok(Err(AssetRefusal::TooLarge)) => (
            StatusCode::PAYLOAD_TOO_LARGE,
            "That file is too large to show",
        )
            .into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
