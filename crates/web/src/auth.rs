//! Bearer-token check for every `/api/*` request.
//!
//! The token travels in the `Authorization` header, never in a cookie, so a
//! page on another origin cannot ride the browser's credentials into this
//! server (no CORS headers are sent either). Comparison is constant-time.

use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use subtle::ConstantTimeEq;

use crate::state::AppState;

pub fn token_matches(expected: &str, presented: &str) -> bool {
    expected.as_bytes().ct_eq(presented.as_bytes()).into()
}

fn presented_token(req: &Request) -> Option<&str> {
    req.headers()
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::trim)
}

pub async fn require_token(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Response {
    match presented_token(&req) {
        Some(token) if token_matches(&state.token, token) => next.run(req).await,
        _ => (StatusCode::UNAUTHORIZED, Json("Unauthorized")).into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::token_matches;

    #[test]
    fn token_comparison_is_exact() {
        assert!(token_matches("abc", "abc"));
        assert!(!token_matches("abc", "abd"));
        assert!(!token_matches("abc", "ab"));
        assert!(!token_matches("abc", ""));
    }
}
