// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;
mod core;
mod models;
mod utils;

use api::{project_commands, document_commands, scrivener_commands};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            // Project commands
            project_commands::create_project,
            project_commands::load_project,
            project_commands::save_project,
            project_commands::add_to_hierarchy,
            project_commands::add_to_folder,
            project_commands::remove_from_hierarchy,
            project_commands::move_node,
            project_commands::reorder_node,

            // Document commands
            document_commands::create_document,
            document_commands::update_document,
            document_commands::delete_document,
            document_commands::get_document,

            // Scrivener commands
            scrivener_commands::import_scrivener_project,
            scrivener_commands::export_to_scrivener,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
