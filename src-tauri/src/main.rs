#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use commands::{ai, convert, document, git, io, project, search, settings, templates};

fn main() {
    // Work around WebKitGTK DMA-BUF renderer crash on Wayland
    // https://github.com/tauri-apps/tauri/issues/10702
    #[cfg(target_os = "linux")]
    unsafe {
        std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            project::create_project,
            project::load_project,
            project::save_project,
            project::import_scrivener,
            project::update_project_metadata,
            project::pick_scriv_folder,
            document::get_document,
            document::update_document_content,
            document::update_document_metadata,
            document::add_comment,
            document::update_comment,
            document::delete_comment,
            document::rename_node,
            document::link_documents,
            document::create_document,
            document::create_folder,
            document::delete_node,
            document::move_node,
            ai::get_ai_settings,
            ai::save_ai_settings,
            ai::ai_summarize,
            ai::ai_transform,
            git::save_revision,
            git::list_revisions,
            git::restore_revision,
            git::create_draft,
            git::list_drafts,
            git::switch_draft,
            git::merge_draft,
            git::push_backup,
            git::sync_push,
            git::sync_fetch,
            git::sync_status,
            git::has_changes,
            git::backup_on_close,
            io::compile_project,
            io::get_compile_formats,
            io::import_file,
            io::import_markdown_folder,
            settings::get_app_settings,
            settings::save_app_settings,
            settings::get_recent_projects,
            settings::add_recent_project,
            settings::check_pandoc,
            search::search_project,
            templates::list_templates,
            templates::create_from_template,
            templates::save_as_template,
            git::revision_diff,
            git::word_diff,
            git::compare_drafts,
            git::word_diff_drafts,
            convert::markdown_to_html,
            convert::html_to_markdown,
            io::get_project_stats,
            io::get_writing_history,
            io::record_daily_words,
        ])
        .run(tauri::generate_context!())
        .expect("error while running ChickenScratch");
}
