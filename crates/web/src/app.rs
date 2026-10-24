use std::sync::Arc;

use axum::{
    extract::DefaultBodyLimit,
    http::StatusCode,
    middleware,
    routing::{get, post},
    Json, Router,
};
use tower_http::trace::TraceLayer;

use crate::{auth, commands, export, projects, state::AppState, static_ui, upload};

/// 50 MiB: the same cap the desktop's bounded Pandoc runs use.
pub const MAX_BODY_BYTES: usize = 50 * 1024 * 1024;

async fn no_events() -> (StatusCode, Json<&'static str>) {
    (
        StatusCode::NOT_FOUND,
        Json("This server has no event channel yet"),
    )
}

/// The whole server as one router: `/api/*` behind the token check, the UI
/// everywhere else.
pub fn build_router(state: Arc<AppState>) -> Router {
    let api = Router::new()
        .route("/cmd/{name}", post(commands::handle))
        .route("/projects", get(projects::list))
        .route("/info", get(projects::info))
        .route("/export", post(export::export))
        .route("/import-upload", post(upload::import_upload))
        .route("/events", get(no_events))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_token,
        ))
        // Whole projects travel in one body (save_project) and uploads
        // can be large documents; axum's 2 MiB default is too small.
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES));

    let [csp, nosniff, referrer] = static_ui::security_headers();

    Router::new()
        .nest("/api", api)
        .fallback_service(static_ui::service(&state.ui_dir))
        .layer(csp)
        .layer(nosniff)
        .layer(referrer)
        // Method, path and status only — never the query string or a body.
        .layer(
            TraceLayer::new_for_http().make_span_with(|req: &axum::extract::Request| {
                tracing::info_span!("request", method = %req.method(), path = %req.uri().path())
            }),
        )
        .with_state(state)
}
