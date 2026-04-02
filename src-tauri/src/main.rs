#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use commands::{document, project};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            project::create_project,
            project::load_project,
            project::save_project,
            project::import_scrivener,
            document::get_document,
            document::update_document_content,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Chicken Scratch");
}
