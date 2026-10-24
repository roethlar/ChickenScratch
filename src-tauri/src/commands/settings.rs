//! Thin Tauri wrappers over `chickenscratch_services::settings` — transport only.
use chickenscratch_services::settings as svc;
#[allow(unused_imports)]
use chickenscratch_services::settings::*;
use chickenscratch_services::ChiknError;

#[tauri::command]
pub fn get_app_settings() -> AppSettings {
    svc::get_app_settings()
}

#[tauri::command]
pub fn save_app_settings(settings: AppSettings) -> Result<(), ChiknError> {
    svc::save_app_settings(settings)
}

#[tauri::command]
pub fn get_recent_projects() -> Vec<RecentProject> {
    svc::get_recent_projects()
}

#[tauri::command]
pub fn add_recent_project(name: String, path: String) -> Result<(), ChiknError> {
    svc::add_recent_project(name, path)
}

#[tauri::command]
pub fn check_pandoc() -> Result<String, ChiknError> {
    svc::check_pandoc()
}
