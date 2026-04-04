#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use commands::{ai, document, project};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            project::create_project,
            project::load_project,
            project::save_project,
            project::import_scrivener,
            project::pick_scriv_folder,
            document::get_document,
            document::update_document_content,
            document::update_document_metadata,
            document::create_document,
            document::create_folder,
            document::delete_node,
            document::move_node,
            ai::get_ai_settings,
            ai::save_ai_settings,
            ai::ai_summarize,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Chicken Scratch");
}
