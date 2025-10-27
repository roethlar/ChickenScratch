use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::{Rc, Weak};

use anyhow::{bail, Context, Result};
use chrono::Utc;
use gtk::prelude::*;
use gtk::{
    gio, glib, FileChooserAction, FileChooserNative, ResponseType, TextBuffer, TextIter, TextTag,
    TreeIter,
};

use chicken_scratch::TreeNode;

use crate::markdown;
use crate::project;
use crate::state::AppState;
use crate::ui::{AppUi, Tags, TreeColumns, BULLET_PREFIX};

const CONTROLLER_KEY: &str = "gtk-editor-controller";

pub fn run() -> Result<()> {
    let app = gtk::Application::builder()
        .application_id("com.chickenscratch.GtkEditor")
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();

    app.connect_activate(|app| {
        let controller = EditorController::new(app);
        app.set_data(CONTROLLER_KEY, controller.clone());
        controller.present();

        let open_action = gio::SimpleAction::new("open", None);
        open_action.connect_activate(clone!(@weak controller => move |_, _| {
            controller.open_project_dialog();
        }));
        app.add_action(&open_action);
        app.set_accels_for_action("app.open", &["<Primary>O"]);

        let save_action = gio::SimpleAction::new("save", None);
        save_action.connect_activate(clone!(@weak controller => move |_, _| {
            if let Err(err) = controller.save_current_document() {
                controller.show_error("Save failed", &format!("{err:#}"));
            }
        }));
        app.add_action(&save_action);
        app.set_accels_for_action("app.save", &["<Primary>S"]);
    });

    app.connect_open(|app, files, _| {
        if let Some(controller) = app.data::<Rc<EditorController>>(CONTROLLER_KEY) {
            if let Some(file) = files.first() {
                if let Some(path) = file.path() {
                    if let Err(err) = controller.load_project(path) {
                        controller.show_error("Unable to open project", &format!("{err:#}"));
                    }
                }
            }
        }
    });

    app.run();

    Ok(())
}

struct EditorController {
    ui: AppUi,
    state: RefCell<AppState>,
}

impl EditorController {
    fn new(app: &gtk::Application) -> Rc<Self> {
        let ui = AppUi::new(app);
        let controller = Rc::new(Self {
            ui,
            state: RefCell::new(AppState::default()),
        });
        controller.setup_handlers();
        controller
    }

    fn present(&self) {
        self.ui.window.present();
    }

    fn setup_handlers(self: &Rc<Self>) {
        self.ui
            .open_button
            .connect_clicked(clone!(@weak self as controller => move |_| {
                controller.open_project_dialog();
            }));

        self.ui
            .save_button
            .connect_clicked(clone!(@weak self as controller => move |_| {
                if let Err(err) = controller.save_current_document() {
                    controller.show_error("Save failed", &format!("{err:#}"));
                }
            }));

        self.ui.tree_selection.connect_changed(
            clone!(@weak self as controller => move |selection| {
                if let Some((model, iter)) = selection.selected() {
                    let is_folder: bool = model
                        .value(&iter, TreeColumns::IS_FOLDER)
                        .get()
                        .unwrap_or(false);
                    if is_folder {
                        return;
                    }
                    if let Ok(doc_id) = model
                        .value(&iter, TreeColumns::ID)
                        .get::<String>()
                    {
                        if let Err(err) = controller.load_document_if_needed(doc_id) {
                            controller.show_error("Unable to load document", &format!("{err:#}"));
                        }
                    }
                }
            }),
        );

        self.ui
            .text_buffer
            .connect_changed(clone!(@weak self as controller => move |_| {
                controller.on_buffer_changed();
            }));

        self.ui
            .bold_button
            .connect_clicked(clone!(@weak self as controller => move |_| {
                controller.toggle_inline_tag(Tags::BOLD);
            }));
        self.ui
            .italic_button
            .connect_clicked(clone!(@weak self as controller => move |_| {
                controller.toggle_inline_tag(Tags::ITALIC);
            }));
        self.ui
            .strike_button
            .connect_clicked(clone!(@weak self as controller => move |_| {
                controller.toggle_inline_tag(Tags::STRIKE);
            }));
        self.ui
            .code_button
            .connect_clicked(clone!(@weak self as controller => move |_| {
                controller.toggle_inline_tag(Tags::CODE);
            }));
        self.ui
            .heading1_button
            .connect_clicked(clone!(@weak self as controller => move |_| {
                controller.set_heading(Some(1));
            }));
        self.ui
            .heading2_button
            .connect_clicked(clone!(@weak self as controller => move |_| {
                controller.set_heading(Some(2));
            }));
        self.ui
            .bullet_button
            .connect_clicked(clone!(@weak self as controller => move |_| {
                controller.toggle_list_item();
            }));

        self.ui.window.connect_close_request(clone!(@weak self as controller => @default-return glib::Propagation::Proceed, move |_| {
            if let Err(err) = controller.save_if_dirty() {
                controller.show_error("Save failed", &format!("{err:#}"));
            }
            glib::Propagation::Proceed
        }));
    }

    fn on_buffer_changed(&self) {
        let mut state = self.state.borrow_mut();
        if state.suspend_buffer_events {
            return;
        }
        state.dirty = true;
        drop(state);
        self.update_status();
    }

    fn load_project(self: &Rc<Self>, path: PathBuf) -> Result<()> {
        if let Some(ext) = path.extension() {
            if ext != "chikn" {
                bail!("Selected folder is not a .chikn project");
            }
        }

        let project = project::open_project(path.as_path())?;

        {
            let mut state = self.state.borrow_mut();
            state.project = Some(project);
            state.current_document_id = None;
            state.dirty = false;
            state.suspend_buffer_events = true;
        }

        self.ui.tree_store.clear();
        if let Some(project) = self.state.borrow().project.as_ref() {
            for node in &project.hierarchy {
                self.insert_tree_node(None, node);
            }
        }
        self.ui.tree_view.expand_all();

        {
            let mut state = self.state.borrow_mut();
            state.suspend_buffer_events = false;
        }

        self.update_title();
        self.update_status();

        if let Some(doc_id) = self.first_document_id() {
            self.select_document_in_tree(&doc_id);
            self.load_document(doc_id)?;
        } else {
            self.clear_editor();
        }

        Ok(())
    }

    fn open_project_dialog(self: &Rc<Self>) {
        let dialog = FileChooserNative::new(
            Some("Open .chikn Project"),
            Some(&self.ui.window),
            FileChooserAction::SelectFolder,
            Some("Open"),
            Some("Cancel"),
        );

        let weak = Rc::downgrade(self);
        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(path) = file.path() {
                        if let Some(controller) = weak.upgrade() {
                            if let Err(err) = controller.load_project(path) {
                                controller
                                    .show_error("Unable to open project", &format!("{err:#}"));
                            }
                        }
                    }
                }
            }
            dialog.destroy();
        });

        dialog.show();
    }

    fn load_document_if_needed(self: &Rc<Self>, doc_id: String) -> Result<()> {
        let current = self.state.borrow().current_document_id.clone();
        if current.as_deref() == Some(doc_id.as_str()) {
            return Ok(());
        }
        self.load_document(doc_id)
    }

    fn load_document(self: &Rc<Self>, doc_id: String) -> Result<()> {
        self.save_if_dirty()?;

        let content = {
            let state = self.state.borrow();
            let project = state.project.as_ref().context("No project loaded")?;
            let document = project
                .documents
                .get(&doc_id)
                .context("Document not found in project")?;
            document.content.clone()
        };

        {
            let mut state = self.state.borrow_mut();
            state.suspend_buffer_events = true;
        }
        markdown::apply_to_buffer(&self.ui.text_buffer, &content)?;
        {
            let mut state = self.state.borrow_mut();
            state.current_document_id = Some(doc_id);
            state.dirty = false;
            state.suspend_buffer_events = false;
        }

        self.update_title();
        self.update_status();
        self.ui
            .text_buffer
            .place_cursor(&self.ui.text_buffer.start_iter());

        Ok(())
    }

    fn save_current_document(self: &Rc<Self>) -> Result<()> {
        let markdown = markdown::buffer_to_markdown(&self.ui.text_buffer);

        {
            let mut state = self.state.borrow_mut();
            let project = match state.project.as_mut() {
                Some(project) => project,
                None => return Ok(()),
            };

            let doc_id = match state.current_document_id.clone() {
                Some(id) => id,
                None => return Ok(()),
            };

            if let Some(document) = project.documents.get_mut(&doc_id) {
                document.content = markdown;
                document.modified = Utc::now().to_rfc3339();
            }

            project::save_project(project)?;
            state.dirty = false;
        }

        self.update_status();
        Ok(())
    }

    fn save_if_dirty(&self) -> Result<()> {
        let dirty = {
            let state = self.state.borrow();
            state.dirty
        };
        if dirty {
            self.save_current_document()?;
        }
        Ok(())
    }

    fn clear_editor(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.suspend_buffer_events = true;
        }
        self.ui.text_buffer.set_text("");
        {
            let mut state = self.state.borrow_mut();
            state.current_document_id = None;
            state.dirty = false;
            state.suspend_buffer_events = false;
        }
        self.update_title();
        self.update_status();
    }

    fn update_status(&self) {
        let word_count = markdown::word_count(&self.ui.text_buffer);
        let (doc_name, dirty) = {
            let state = self.state.borrow();
            let doc_name = if let (Some(project), Some(doc_id)) =
                (&state.project, state.current_document_id.as_ref())
            {
                project.documents.get(doc_id).map(|d| d.name.clone())
            } else {
                None
            };
            (doc_name, state.dirty)
        };

        let status = if let Some(name) = doc_name {
            if dirty {
                format!("{name} • {word_count} words • Unsaved changes")
            } else {
                format!("{name} • {word_count} words")
            }
        } else if dirty {
            format!("{word_count} words • Unsaved changes")
        } else {
            "Ready".to_string()
        };

        self.ui.status_label.set_text(&status);
    }

    fn update_title(&self) {
        let (project_name, doc_name) = {
            let state = self.state.borrow();
            if let Some(project) = &state.project {
                let doc_name = state
                    .current_document_id
                    .as_ref()
                    .and_then(|id| project.documents.get(id).map(|d| d.name.clone()));
                (Some(project.name.clone()), doc_name)
            } else {
                (None, None)
            }
        };

        let title = match (project_name, doc_name) {
            (Some(project), Some(doc)) => format!("{doc} — {project} — Chicken Scratch GTK"),
            (Some(project), None) => format!("{project} — Chicken Scratch GTK"),
            _ => "Chicken Scratch GTK Editor".to_string(),
        };

        self.ui.window.set_title(Some(&title));
    }

    fn show_error(&self, title: &str, message: &str) {
        let dialog = gtk::MessageDialog::builder()
            .transient_for(&self.ui.window)
            .modal(true)
            .message_type(gtk::MessageType::Error)
            .buttons(gtk::ButtonsType::Ok)
            .text(title)
            .secondary_text(message)
            .build();
        dialog.connect_response(|dialog, _| dialog.destroy());
        dialog.show();
    }

    fn toggle_inline_tag(&self, tag_name: &str) {
        let buffer = &self.ui.text_buffer;
        if let Some((mut start, mut end)) = buffer.selection_bounds() {
            if start.equal(&end) {
                return;
            }

            if let Some(tag) = buffer.tag_table().lookup(tag_name) {
                if range_has_tag(buffer, &tag, &start, &end) {
                    buffer.remove_tag(&tag, &mut start, &mut end);
                } else {
                    buffer.apply_tag(&tag, &mut start, &mut end);
                }
            }
        }
    }

    fn set_heading(&self, level: Option<u8>) {
        let buffer = &self.ui.text_buffer;
        let Some((mut line_start, mut line_end)) = line_bounds_at_cursor(buffer) else {
            return;
        };

        if let Some(tag) = buffer.tag_table().lookup(Tags::HEADING1) {
            buffer.remove_tag(&tag, &mut line_start.clone(), &mut line_end.clone());
        }
        if let Some(tag) = buffer.tag_table().lookup(Tags::HEADING2) {
            buffer.remove_tag(&tag, &mut line_start.clone(), &mut line_end.clone());
        }

        self.ensure_no_list_prefix(&mut line_start, &mut line_end);

        match level {
            Some(1) => {
                if let Some(tag) = buffer.tag_table().lookup(Tags::HEADING1) {
                    buffer.apply_tag(&tag, &mut line_start, &mut line_end);
                }
            }
            Some(2) => {
                if let Some(tag) = buffer.tag_table().lookup(Tags::HEADING2) {
                    buffer.apply_tag(&tag, &mut line_start, &mut line_end);
                }
            }
            _ => {}
        }
    }

    fn toggle_list_item(&self) {
        let buffer = &self.ui.text_buffer;
        let Some((mut line_start, mut line_end)) = line_bounds_at_cursor(buffer) else {
            return;
        };

        let line_text = buffer.text(&line_start, &line_end, false).to_string();
        let has_bullet = line_text.starts_with(BULLET_PREFIX);

        if has_bullet {
            let prefix_len = BULLET_PREFIX.chars().count() as i32;
            let mut remove_start = line_start.clone();
            let mut remove_end = line_start.clone();
            remove_end.forward_chars(prefix_len);
            buffer.delete(&mut remove_start, &mut remove_end);
            line_end = buffer.iter_at_offset(line_end.offset() - prefix_len);

            if let Some(tag) = buffer.tag_table().lookup(Tags::LIST_ITEM) {
                buffer.remove_tag(&tag, &mut line_start, &mut line_end);
            }
        } else {
            let mut insert_pos = line_start.clone();
            buffer.insert(&mut insert_pos, BULLET_PREFIX);

            let new_end_offset = line_end.offset() + BULLET_PREFIX.chars().count() as i32;
            line_end = buffer.iter_at_offset(new_end_offset);

            if let Some(tag) = buffer.tag_table().lookup(Tags::LIST_ITEM) {
                buffer.apply_tag(&tag, &mut line_start, &mut line_end);
            }

            if let Some(tag) = buffer.tag_table().lookup(Tags::HEADING1) {
                buffer.remove_tag(&tag, &mut line_start.clone(), &mut line_end.clone());
            }
            if let Some(tag) = buffer.tag_table().lookup(Tags::HEADING2) {
                buffer.remove_tag(&tag, &mut line_start, &mut line_end);
            }
        }
    }

    fn ensure_no_list_prefix(&self, line_start: &mut TextIter, line_end: &mut TextIter) {
        let buffer = &self.ui.text_buffer;
        let line_text = buffer.text(line_start, line_end, false).to_string();
        if line_text.starts_with(BULLET_PREFIX) {
            let count = BULLET_PREFIX.chars().count() as i32;
            let mut remove_start = line_start.clone();
            let mut remove_end = line_start.clone();
            remove_end.forward_chars(count);
            buffer.delete(&mut remove_start, &mut remove_end);
            *line_end = buffer.iter_at_offset(line_end.offset() - count);

            if let Some(tag) = buffer.tag_table().lookup(Tags::LIST_ITEM) {
                buffer.remove_tag(&tag, line_start, line_end);
            }
        }
    }

    fn insert_tree_node(&self, parent: Option<&TreeIter>, node: &TreeNode) {
        match node {
            TreeNode::Folder { id, name, children } => {
                let iter = self.ui.tree_store.append(parent);
                self.ui.tree_store.set(
                    &iter,
                    &[
                        (TreeColumns::NAME, name.as_str()),
                        (TreeColumns::ID, id.as_str()),
                        (TreeColumns::IS_FOLDER, true),
                    ],
                );
                for child in children {
                    self.insert_tree_node(Some(&iter), child);
                }
            }
            TreeNode::Document { id, name, .. } => {
                let iter = self.ui.tree_store.append(parent);
                self.ui.tree_store.set(
                    &iter,
                    &[
                        (TreeColumns::NAME, name.as_str()),
                        (TreeColumns::ID, id.as_str()),
                        (TreeColumns::IS_FOLDER, false),
                    ],
                );
            }
        }
    }

    fn first_document_id(&self) -> Option<String> {
        let state = self.state.borrow();
        let project = state.project.as_ref()?;
        find_first_document(&project.hierarchy)
    }

    fn select_document_in_tree(&self, doc_id: &str) {
        if let Some(iter) = find_iter_by_id(&self.ui.tree_store, doc_id) {
            self.ui.tree_selection.select_iter(&iter);
        }
    }
}

fn find_first_document(nodes: &[TreeNode]) -> Option<String> {
    for node in nodes {
        match node {
            TreeNode::Document { id, .. } => return Some(id.clone()),
            TreeNode::Folder { children, .. } => {
                if let Some(id) = find_first_document(children) {
                    return Some(id);
                }
            }
        }
    }
    None
}

fn find_iter_by_id(store: &gtk::TreeStore, target: &str) -> Option<TreeIter> {
    let mut iter = store.iter_first()?;
    loop {
        let id: Option<String> = store.value(&iter, TreeColumns::ID).get().ok();
        if id.as_deref() == Some(target) {
            return Some(iter);
        }

        if let Some(child) = store.iter_children(Some(&iter)) {
            if let Some(found) = find_iter_in_children(store, child, target) {
                return Some(found);
            }
        }

        if !store.iter_next(&iter) {
            break;
        }
    }

    None
}

fn find_iter_in_children(
    store: &gtk::TreeStore,
    mut iter: TreeIter,
    target: &str,
) -> Option<TreeIter> {
    loop {
        let id: Option<String> = store.value(&iter, TreeColumns::ID).get().ok();
        if id.as_deref() == Some(target) {
            return Some(iter);
        }

        if let Some(child) = store.iter_children(Some(&iter)) {
            if let Some(found) = find_iter_in_children(store, child, target) {
                return Some(found);
            }
        }

        if !store.iter_next(&iter) {
            break;
        }
    }
    None
}

fn range_has_tag(buffer: &TextBuffer, tag: &TextTag, start: &TextIter, end: &TextIter) -> bool {
    let mut iter = start.clone();
    while iter.compare(end) < 0 {
        if !iter.has_tag(tag) {
            return false;
        }
        let mut next = iter.clone();
        if !next.forward_to_tag_toggle(Some(tag)) {
            break;
        }
        if next.compare(end) <= 0 {
            iter = next;
        } else {
            break;
        }
    }
    true
}

fn line_bounds_at_cursor(buffer: &TextBuffer) -> Option<(TextIter, TextIter)> {
    let mut insert_iter = buffer.iter_at_mark(&buffer.insert_mark());
    let mut line_start = insert_iter.clone();
    line_start.backward_to_line_start();
    let mut line_end = line_start.clone();
    line_end.forward_to_line_end();
    Some((line_start, line_end))
}
