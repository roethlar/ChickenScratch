use chicken_scratch::Project;

/// Runtime state for the GTK editor.
#[derive(Default)]
pub struct AppState {
    /// Currently opened .chikn project (if any).
    pub project: Option<Project>,
    /// Active document ID.
    pub current_document_id: Option<String>,
    /// Whether the editor buffer has unsaved changes.
    pub dirty: bool,
    /// Guards against reacting to programmatic buffer updates.
    pub suspend_buffer_events: bool,
}

impl AppState {
    pub fn clear(&mut self) {
        self.project = None;
        self.current_document_id = None;
        self.dirty = false;
        self.suspend_buffer_events = false;
    }
}
