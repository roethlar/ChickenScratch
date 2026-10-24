use std::sync::Arc;

use axum::{
    http::StatusCode,
    middleware,
    routing::{get, post},
    Json, Router,
};
use tower_http::trace::TraceLayer;

use crate::{auth, commands, projects, state::AppState, static_ui};

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
        .route("/events", get(no_events))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_token,
        ));

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
