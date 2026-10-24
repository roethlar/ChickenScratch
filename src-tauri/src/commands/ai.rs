//! Thin Tauri wrappers over `chickenscratch_services::ai` — transport only.
use chickenscratch_services::ai as svc;
#[allow(unused_imports)]
use chickenscratch_services::ai::*;
use chickenscratch_services::settings::AiSettings;
use chickenscratch_services::ChiknError;
use std::sync::Arc;
use tauri::Emitter;

pub use chickenscratch_services::ai::AiStreamRegistry;

/// Delivers streamed AI output to the webview as Tauri events.
struct TauriSink(tauri::AppHandle);

impl chickenscratch_services::AiStreamSink for TauriSink {
    fn chunk(&self, request_id: &str, delta: &str) {
        let _ = self.0.emit(
            "ai:chunk",
            serde_json::json!({ "id": request_id, "delta": delta }),
        );
    }
    fn done(&self, request_id: &str) {
        let _ = self
            .0
            .emit("ai:done", serde_json::json!({ "id": request_id }));
    }
    fn error(&self, request_id: &str, message: &str) {
        let _ = self.0.emit(
            "ai:error",
            serde_json::json!({ "id": request_id, "message": message }),
        );
    }
}

#[tauri::command]
pub fn get_ai_settings() -> AiSettings {
    svc::get_ai_settings()
}

#[tauri::command]
pub fn save_ai_settings(ai: AiSettings) -> Result<(), ChiknError> {
    svc::save_ai_settings(ai)
}

#[tauri::command]
pub fn ai_summarize(content: String) -> Result<String, ChiknError> {
    svc::ai_summarize(content)
}

#[tauri::command]
pub fn ai_transform_stream(
    app: tauri::AppHandle,
    registry: tauri::State<'_, AiStreamRegistry>,
    content: String,
    operation: String,
    request_id: String,
) -> Result<(), ChiknError> {
    svc::ai_transform_stream(
        Arc::new(TauriSink(app)),
        &registry,
        content,
        operation,
        request_id,
    )
}

#[tauri::command]
pub fn cancel_ai_transform_stream(
    registry: tauri::State<'_, AiStreamRegistry>,
    request_id: String,
) -> Result<(), ChiknError> {
    svc::cancel_ai_transform_stream(&registry, request_id)
}

#[tauri::command]
pub fn ai_transform(content: String, operation: String) -> Result<String, ChiknError> {
    svc::ai_transform(content, operation)
}
