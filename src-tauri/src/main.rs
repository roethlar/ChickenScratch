// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;
mod core;
mod models;
mod utils;

use api::project_commands;
use api::document_commands;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            // Project commands
            project_commands::create_project,
            project_commands::load_project,
            project_commands::save_project,

            // Document commands
            document_commands::create_document,
            document_commands::update_document,
            document_commands::delete_document,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
