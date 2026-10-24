//! `GET /api/events`: the browser's event channel (server-sent events).
//!
//! The desktop shell delivers streamed AI output as Tauri events; here the
//! same three events (`ai:chunk`, `ai:done`, `ai:error`, same JSON payloads)
//! go out over one long-lived SSE response per connected page. A broadcast
//! channel fans them out; a page that is not listening simply misses them,
//! which matches Tauri's fire-and-forget events.

use std::convert::Infallible;
use std::sync::Arc;

use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
};
use chickenscratch_services::AiStreamSink;
use serde::Serialize;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};

use crate::state::AppState;

/// One event as the UI's transport expects it: `event:` name plus JSON data.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SseEvent {
    pub event: String,
    pub data: serde_json::Value,
}

/// Room for a burst of chunks while a slow page catches up.
pub const CHANNEL_CAPACITY: usize = 1024;

pub fn channel() -> broadcast::Sender<SseEvent> {
    broadcast::channel(CHANNEL_CAPACITY).0
}

/// Delivers streamed AI output to every connected page.
pub struct WebSink(pub broadcast::Sender<SseEvent>);

impl WebSink {
    fn send(&self, event: &str, data: serde_json::Value) {
        // No receivers connected is not an error: nobody is listening.
        let _ = self.0.send(SseEvent {
            event: event.to_string(),
            data,
        });
    }
}

impl AiStreamSink for WebSink {
    fn chunk(&self, request_id: &str, delta: &str) {
        self.send(
            "ai:chunk",
            serde_json::json!({ "id": request_id, "delta": delta }),
        );
    }
    fn done(&self, request_id: &str) {
        self.send("ai:done", serde_json::json!({ "id": request_id }));
    }
    fn error(&self, request_id: &str, message: &str) {
        self.send(
            "ai:error",
            serde_json::json!({ "id": request_id, "message": message }),
        );
    }
}

pub fn to_sse(event: &SseEvent) -> Event {
    Event::default()
        .event(event.event.clone())
        .data(event.data.to_string())
}

pub async fn stream(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let receiver = state.events.subscribe();
    let stream = BroadcastStream::new(receiver).filter_map(|item| match item {
        Ok(event) => Some(Ok(to_sse(&event))),
        // Lagged: this page fell behind and lost some events; keep going.
        Err(_) => None,
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_sink_broadcasts_the_three_ai_events_with_tauri_payloads() {
        let sender = channel();
        let mut receiver = sender.subscribe();
        let sink = WebSink(sender);

        sink.chunk("r1", "Hel");
        sink.done("r1");
        sink.error("r2", "boom");

        assert_eq!(
            receiver.try_recv().unwrap(),
            SseEvent {
                event: "ai:chunk".into(),
                data: serde_json::json!({ "id": "r1", "delta": "Hel" })
            }
        );
        assert_eq!(
            receiver.try_recv().unwrap(),
            SseEvent {
                event: "ai:done".into(),
                data: serde_json::json!({ "id": "r1" })
            }
        );
        assert_eq!(
            receiver.try_recv().unwrap(),
            SseEvent {
                event: "ai:error".into(),
                data: serde_json::json!({ "id": "r2", "message": "boom" })
            }
        );
    }

    #[test]
    fn sending_with_no_listener_is_not_an_error() {
        let sink = WebSink(channel());
        sink.chunk("r1", "x"); // must not panic
    }
}
