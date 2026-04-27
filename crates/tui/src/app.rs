use anyhow::{anyhow, Context, Result};
use chickenscratch_core::core::git;
use chickenscratch_core::core::project::{hierarchy, reader, writer};
use chickenscratch_core::utils::slug;
use chickenscratch_core::{Document, Project, TreeNode};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::Backend, Terminal};
use ratatui_textarea::{TextArea, WrapMode};
use std::path::PathBuf;
use std::time::Duration;

use crate::convert;
use crate::ui;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Focus {
    Binder,
    Editor,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Mode {
    Normal,
    RevisionPrompt,
    NewDocPrompt,
    NewFolderPrompt,
    Confirm,
    Comments,
    CommentEdit,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum ViewMode {
    Edit,    // editable markdown
    Preview, // read-only formatted rendering
}

impl ViewMode {
    pub fn label(self) -> &'static str {
        match self {
            ViewMode::Edit => "edit",
            ViewMode::Preview => "preview",
        }
    }

    pub fn next(self) -> ViewMode {
        match self {
            ViewMode::Edit => ViewMode::Preview,
            ViewMode::Preview => ViewMode::Edit,
        }
    }
}

pub struct BinderItem {
    pub depth: usize,
    pub id: String,
    pub name: String,
    pub is_folder: bool,
}

pub struct App<'a> {
    pub project: Project,
    pub project_path: PathBuf,

    pub focus: Focus,
    pub mode: Mode,

    pub binder_items: Vec<BinderItem>,
    pub binder_selected: usize,
    pub expanded: std::collections::HashSet<String>,

    pub active_doc_id: Option<String>,
    pub editor: TextArea<'a>,
    pub dirty: bool,
    pub view_mode: ViewMode,
    pub wrap: bool,

    pub status: String,
    pub prompt_input: String,
    pub should_quit: bool,
    pub new_item_parent_id: Option<String>,

    // Comments overlay state
    pub comments_selected: usize,
    pub comment_edit_id: Option<String>,
    /// Snapshot of editor selection while we prompt for an anchored-comment body.
    /// Tuple: (start_row, start_col, end_row, end_col).
    pub pending_selection: Option<(usize, usize, usize, usize)>,
}

impl<'a> App<'a> {
    pub fn new(project_path: PathBuf) -> Result<Self> {
        let project = reader::read_project(&project_path)
            .map_err(|e| anyhow!("Failed to read project: {:?}", e))?;

        let mut expanded = std::collections::HashSet::new();
        for node in &project.hierarchy {
            if let TreeNode::Folder { id, .. } = node {
                expanded.insert(id.clone());
            }
        }

        let mut app = Self {
            project,
            project_path,
            focus: Focus::Binder,
            mode: Mode::Normal,
            binder_items: Vec::new(),
            binder_selected: 0,
            expanded,
            active_doc_id: None,
            editor: TextArea::default(),
            dirty: false,
            view_mode: ViewMode::Edit,
            wrap: true,
            status: "Ready. ?=help  Tab=switch pane  q=quit".to_string(),
            prompt_input: String::new(),
            should_quit: false,
            new_item_parent_id: None,
            comments_selected: 0,
            comment_edit_id: None,
            pending_selection: None,
        };
        app.rebuild_binder();
        app.apply_editor_settings();
        Ok(app)
    }

    fn apply_editor_settings(&mut self) {
        self.editor.set_wrap_mode(if self.wrap {
            WrapMode::WordOrGlyph
        } else {
            WrapMode::None
        });
    }

    pub fn rebuild_binder(&mut self) {
        self.binder_items.clear();
        let nodes = self.project.hierarchy.clone();
        self.walk_hierarchy(&nodes, 0);
        if self.binder_selected >= self.binder_items.len() && !self.binder_items.is_empty() {
            self.binder_selected = self.binder_items.len() - 1;
        }
    }

    fn walk_hierarchy(&mut self, nodes: &[TreeNode], depth: usize) {
        for node in nodes {
            match node {
                TreeNode::Folder { id, name, children } => {
                    self.binder_items.push(BinderItem {
                        depth,
                        id: id.clone(),
                        name: name.clone(),
                        is_folder: true,
                    });
                    if self.expanded.contains(id) {
                        self.walk_hierarchy(children, depth + 1);
                    }
                }
                TreeNode::Document { id, name, .. } => {
                    self.binder_items.push(BinderItem {
                        depth,
                        id: id.clone(),
                        name: name.clone(),
                        is_folder: false,
                    });
                }
            }
        }
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()>
    where
        <B as Backend>::Error: Send + Sync + 'static,
    {
        while !self.should_quit {
            terminal.draw(|f| ui::render(f, self))?;

            if event::poll(Duration::from_millis(200))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind != crossterm::event::KeyEventKind::Press {
                        continue;
                    }
                    self.handle_key(key)?;
                }
            }
        }
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match self.mode {
            Mode::RevisionPrompt => self.handle_prompt_key(key),
            Mode::NewDocPrompt | Mode::NewFolderPrompt => self.handle_new_item_key(key),
            Mode::Confirm => self.handle_confirm_key(key),
            Mode::Normal => self.handle_normal_key(key),
            Mode::Comments => self.handle_comments_key(key),
            Mode::CommentEdit => self.handle_comment_edit_key(key),
        }
    }

    fn handle_comments_key(&mut self, key: KeyEvent) -> Result<()> {
        let comments = self.current_comments();
        let n = comments.len();
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = Mode::Normal;
            }
            KeyCode::Down | KeyCode::Char('j')
                if n > 0 && self.comments_selected + 1 < n => {
                    self.comments_selected += 1;
                }
            KeyCode::Up | KeyCode::Char('k')
                if self.comments_selected > 0 => {
                    self.comments_selected -= 1;
                }
            KeyCode::Char('n') | KeyCode::Char('a')
                // New document-level comment (no anchor)
                if self.active_doc_id.is_some() => {
                    self.comment_edit_id = Some(String::new()); // empty id = new comment
                    self.prompt_input.clear();
                    self.mode = Mode::CommentEdit;
                }
            KeyCode::Char('r') => {
                if let Some(id) = comments.get(self.comments_selected).map(|c| c.id.clone()) {
                    self.toggle_resolve_comment(&id);
                }
            }
            KeyCode::Char('d') => {
                if let Some(id) = comments.get(self.comments_selected).map(|c| c.id.clone()) {
                    self.delete_comment(&id)?;
                }
            }
            KeyCode::Char('e') | KeyCode::Enter => {
                if let Some(c) = comments.get(self.comments_selected) {
                    self.comment_edit_id = Some(c.id.clone());
                    self.prompt_input = c.body.clone();
                    self.mode = Mode::CommentEdit;
                } else if n == 0 && self.active_doc_id.is_some() {
                    // Empty list — Enter/e creates a new doc-level comment
                    self.comment_edit_id = Some(String::new());
                    self.prompt_input.clear();
                    self.mode = Mode::CommentEdit;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_comment_edit_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Comments;
                self.prompt_input.clear();
                self.comment_edit_id = None;
            }
            KeyCode::Enter => {
                if let Some(id) = self.comment_edit_id.take() {
                    let body = self.prompt_input.clone();
                    if id.is_empty() {
                        // New comment — anchored if we have a pending selection,
                        // otherwise document-level (orphan).
                        if let Some(sel) = self.pending_selection.take() {
                            self.add_anchored_comment(&body, sel)?;
                        } else {
                            self.add_orphan_comment(&body);
                        }
                    } else {
                        self.update_comment_body(&id, &body);
                    }
                }
                self.prompt_input.clear();
                // After anchored comment, return to Normal (focus stays on editor);
                // after orphan from overlay, return to Comments overlay.
                self.mode = if self.focus == Focus::Editor {
                    Mode::Normal
                } else {
                    Mode::Comments
                };
            }
            KeyCode::Backspace => {
                self.prompt_input.pop();
            }
            KeyCode::Char(c) => {
                self.prompt_input.push(c);
            }
            _ => {}
        }
        Ok(())
    }

    fn add_anchored_comment(
        &mut self,
        body: &str,
        selection: (usize, usize, usize, usize),
    ) -> Result<()> {
        if body.trim().is_empty() {
            self.status = "Empty body; comment not added".to_string();
            return Ok(());
        }
        let doc_id = match self.active_doc_id.as_ref() {
            Some(id) => id.clone(),
            None => return Ok(()),
        };

        let (sr, sc, er, ec) = normalize_selection(selection);
        let mut lines: Vec<String> = self.editor.lines().iter().map(|l| l.to_string()).collect();

        let comment_id = format!("c_{}", uuid::Uuid::new_v4().simple());
        let open_tag = format!(
            "<span class=\"comment\" data-comment-id=\"{}\">",
            comment_id
        );
        let close_tag = "</span>";

        if !wrap_selection_in_lines(&mut lines, sr, sc, er, ec, &open_tag, close_tag) {
            self.status = "Empty selection; nothing to anchor".to_string();
            return Ok(());
        }

        // Update editor with wrapped content
        self.editor = TextArea::new(lines.clone());
        self.apply_editor_settings();
        self.dirty = true;

        let md = lines.join("\n");
        let now = chrono::Utc::now().to_rfc3339();
        if let Some(doc) = self.project.documents.get_mut(&doc_id) {
            doc.content = md;
            doc.comments.push(chickenscratch_core::models::Comment {
                id: comment_id,
                body: body.to_string(),
                resolved: false,
                created: now.clone(),
                modified: now.clone(),
            });
            doc.modified = now;
        }

        writer::write_project(&mut self.project).map_err(|e| anyhow!("Write failed: {:?}", e))?;
        self.dirty = false;
        self.status = "Comment anchored to selection".to_string();
        Ok(())
    }

    fn add_orphan_comment(&mut self, body: &str) {
        if body.trim().is_empty() {
            return;
        }
        let doc_id = match self.active_doc_id.as_ref() {
            Some(id) => id.clone(),
            None => return,
        };
        let comment_id = format!("c_{}", uuid::Uuid::new_v4().simple());
        let now = chrono::Utc::now().to_rfc3339();
        let did_add = if let Some(doc) = self.project.documents.get_mut(&doc_id) {
            doc.comments.push(chickenscratch_core::models::Comment {
                id: comment_id,
                body: body.to_string(),
                resolved: false,
                created: now.clone(),
                modified: now.clone(),
            });
            doc.modified = now;
            true
        } else {
            false
        };
        if did_add {
            let _ = writer::write_project(&mut self.project);
            self.comments_selected = self.current_comments().len().saturating_sub(1);
            self.status = "Comment added".to_string();
        }
    }

    pub fn current_comments(&self) -> Vec<chickenscratch_core::models::Comment> {
        self.active_doc_id
            .as_ref()
            .and_then(|id| self.project.documents.get(id))
            .map(|d| d.comments.clone())
            .unwrap_or_default()
    }

    /// Extract the anchor text between `<span class="comment" data-comment-id="id">`
    /// and `</span>` from the document's HTML content.
    pub fn comment_anchor_text(&self, comment_id: &str) -> String {
        let doc = match self
            .active_doc_id
            .as_ref()
            .and_then(|id| self.project.documents.get(id))
        {
            Some(d) => d,
            None => return String::new(),
        };
        let needle = format!("data-comment-id=\"{}\"", comment_id);
        let html = &doc.content;
        if let Some(start) = html.find(&needle) {
            if let Some(close) = html[start..].find('>') {
                let content_start = start + close + 1;
                if let Some(end_rel) = html[content_start..].find("</span>") {
                    let text_html = &html[content_start..content_start + end_rel];
                    return strip_tags_inline(text_html);
                }
            }
        }
        String::new()
    }

    fn toggle_resolve_comment(&mut self, comment_id: &str) {
        let doc_id = match self.active_doc_id.as_ref() {
            Some(id) => id.clone(),
            None => return,
        };
        let resolved_after = {
            if let Some(doc) = self.project.documents.get_mut(&doc_id) {
                if let Some(c) = doc.comments.iter_mut().find(|c| c.id == comment_id) {
                    c.resolved = !c.resolved;
                    c.modified = chrono::Utc::now().to_rfc3339();
                    doc.modified = chrono::Utc::now().to_rfc3339();
                    Some(c.resolved)
                } else {
                    None
                }
            } else {
                None
            }
        };
        if let Some(r) = resolved_after {
            let _ = writer::write_project(&mut self.project);
            self.status = format!("Comment {}", if r { "resolved" } else { "reopened" });
        }
    }

    fn update_comment_body(&mut self, comment_id: &str, body: &str) {
        let doc_id = match self.active_doc_id.as_ref() {
            Some(id) => id.clone(),
            None => return,
        };
        let did_update = {
            if let Some(doc) = self.project.documents.get_mut(&doc_id) {
                if let Some(c) = doc.comments.iter_mut().find(|c| c.id == comment_id) {
                    c.body = body.to_string();
                    c.modified = chrono::Utc::now().to_rfc3339();
                    doc.modified = chrono::Utc::now().to_rfc3339();
                    true
                } else {
                    false
                }
            } else {
                false
            }
        };
        if did_update {
            let _ = writer::write_project(&mut self.project);
            self.status = "Comment updated".to_string();
        }
    }

    fn delete_comment(&mut self, comment_id: &str) -> Result<()> {
        let doc_id = match self.active_doc_id.as_ref() {
            Some(id) => id.clone(),
            None => return Ok(()),
        };
        let new_content = if let Some(doc) = self.project.documents.get_mut(&doc_id) {
            doc.comments.retain(|c| c.id != comment_id);
            doc.content = strip_comment_span(&doc.content, comment_id);
            doc.modified = chrono::Utc::now().to_rfc3339();
            Some(doc.content.clone())
        } else {
            None
        };

        writer::write_project(&mut self.project).map_err(|e| anyhow!("Write failed: {:?}", e))?;

        if let Some(md) = new_content {
            if self.view_mode == ViewMode::Edit {
                self.load_markdown(&md);
            }
        }

        let n = self.current_comments().len();
        if n > 0 && self.comments_selected >= n {
            self.comments_selected = n - 1;
        }
        self.status = "Comment deleted".to_string();
        Ok(())
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> Result<()> {
        // F2: toggle comments overlay (works regardless of focus)
        if key.code == KeyCode::F(2) {
            if self.active_doc_id.is_some() {
                self.mode = Mode::Comments;
                self.comments_selected = 0;
            } else {
                self.status = "Open a document first to see comments".to_string();
            }
            return Ok(());
        }

        // F3: add anchored comment on current editor selection
        if key.code == KeyCode::F(3) {
            if self.active_doc_id.is_some() && self.focus == Focus::Editor {
                if let Some((start, end)) = self.editor.selection_range() {
                    self.pending_selection = Some((start.0, start.1, end.0, end.1));
                    self.comment_edit_id = Some(String::new());
                    self.prompt_input.clear();
                    self.mode = Mode::CommentEdit;
                } else {
                    self.status =
                        "Select text first (Shift+arrows), then F3 to anchor a comment".to_string();
                }
            } else {
                self.status = "Focus the editor and select text first".to_string();
            }
            return Ok(());
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('s') => {
                    self.save_active_doc()?;
                    return Ok(());
                }
                KeyCode::Char('r') => {
                    self.prompt_input =
                        format!("Revision {}", chrono::Utc::now().format("%Y-%m-%d %H:%M"));
                    self.mode = Mode::RevisionPrompt;
                    return Ok(());
                }
                KeyCode::Char('t') => {
                    self.cycle_view_mode();
                    return Ok(());
                }
                KeyCode::Char('w') => {
                    self.wrap = !self.wrap;
                    self.apply_editor_settings();
                    self.status = format!("Wrap {}", if self.wrap { "on" } else { "off" });
                    return Ok(());
                }
                KeyCode::Char(';') => {
                    if self.active_doc_id.is_some() {
                        self.mode = Mode::Comments;
                        self.comments_selected = 0;
                    }
                    return Ok(());
                }
                KeyCode::Char('q') => {
                    self.try_quit();
                    return Ok(());
                }
                _ => {}
            }
        }

        if key.code == KeyCode::Tab {
            self.focus = match self.focus {
                Focus::Binder => Focus::Editor,
                Focus::Editor => Focus::Binder,
            };
            return Ok(());
        }

        match self.focus {
            Focus::Binder => self.handle_binder_key(key),
            Focus::Editor => self.handle_editor_key(key),
        }
    }

    fn handle_binder_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.try_quit(),
            KeyCode::Char('?') => {
                self.status = "Keys: ↑↓=nav Enter=open Tab=editor n=new doc N=new folder Ctrl+S=save Ctrl+R=rev Ctrl+T=view Ctrl+W=wrap F2=comments q=quit".to_string();
            }
            KeyCode::Char('n') => {
                self.new_item_parent_id = self.selected_folder_id();
                self.prompt_input.clear();
                self.mode = Mode::NewDocPrompt;
            }
            KeyCode::Char('N') => {
                self.new_item_parent_id = self.selected_folder_id();
                self.prompt_input.clear();
                self.mode = Mode::NewFolderPrompt;
            }
            KeyCode::Down | KeyCode::Char('j')
                if self.binder_selected + 1 < self.binder_items.len() =>
            {
                self.binder_selected += 1;
            }
            KeyCode::Up | KeyCode::Char('k') if self.binder_selected > 0 => {
                self.binder_selected -= 1;
            }
            KeyCode::Enter => {
                if let Some(item) = self.binder_items.get(self.binder_selected) {
                    if item.is_folder {
                        if self.expanded.contains(&item.id) {
                            self.expanded.remove(&item.id);
                        } else {
                            self.expanded.insert(item.id.clone());
                        }
                        self.rebuild_binder();
                    } else {
                        let id = item.id.clone();
                        self.open_document(&id);
                        self.focus = Focus::Editor;
                    }
                }
            }
            KeyCode::Char(' ') => {
                if let Some(item) = self.binder_items.get(self.binder_selected) {
                    if item.is_folder {
                        if self.expanded.contains(&item.id) {
                            self.expanded.remove(&item.id);
                        } else {
                            self.expanded.insert(item.id.clone());
                        }
                        self.rebuild_binder();
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_editor_key(&mut self, key: KeyEvent) -> Result<()> {
        if self.active_doc_id.is_none() {
            if key.code == KeyCode::Esc {
                self.focus = Focus::Binder;
            }
            return Ok(());
        }
        if key.code == KeyCode::Esc {
            self.focus = Focus::Binder;
            return Ok(());
        }
        if self.view_mode == ViewMode::Preview {
            // Preview is read-only — no keys do anything here
            return Ok(());
        }
        let changed = self.editor.input(key);
        if changed {
            self.dirty = true;
        }
        Ok(())
    }

    fn cycle_view_mode(&mut self) {
        self.view_mode = self.view_mode.next();
        self.status = format!("View: {}", self.view_mode.label());
    }

    fn load_markdown(&mut self, md: &str) {
        let lines: Vec<String> = if md.is_empty() {
            vec![String::new()]
        } else {
            md.lines().map(String::from).collect()
        };
        self.editor = TextArea::new(lines);
        self.apply_editor_settings();
    }

    pub fn editor_content_string(&self) -> String {
        self.editor.lines().join("\n")
    }

    fn handle_prompt_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.prompt_input.clear();
            }
            KeyCode::Enter => {
                let msg = self.prompt_input.trim().to_string();
                self.mode = Mode::Normal;
                self.prompt_input.clear();
                if !msg.is_empty() {
                    self.save_revision(&msg)?;
                }
            }
            KeyCode::Backspace => {
                self.prompt_input.pop();
            }
            KeyCode::Char(c) => {
                self.prompt_input.push(c);
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_new_item_key(&mut self, key: KeyEvent) -> Result<()> {
        let is_doc = self.mode == Mode::NewDocPrompt;
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.prompt_input.clear();
                self.new_item_parent_id = None;
            }
            KeyCode::Enter => {
                let name = self.prompt_input.trim().to_string();
                self.mode = Mode::Normal;
                self.prompt_input.clear();
                if !name.is_empty() {
                    let parent = self.new_item_parent_id.take();
                    if is_doc {
                        self.create_document_in_project(name, parent)?;
                    } else {
                        self.create_folder_in_project(name, parent)?;
                    }
                }
            }
            KeyCode::Backspace => {
                self.prompt_input.pop();
            }
            KeyCode::Char(c) => {
                self.prompt_input.push(c);
            }
            _ => {}
        }
        Ok(())
    }

    fn selected_folder_id(&self) -> Option<String> {
        self.binder_items
            .get(self.binder_selected)
            .filter(|item| item.is_folder)
            .map(|item| item.id.clone())
    }

    fn create_document_in_project(&mut self, name: String, parent_id: Option<String>) -> Result<()> {
        let s = slug::unique_slug(&name, "manuscript/", &self.project.documents);
        let doc_path = format!("manuscript/{}.md", s);
        let now = chrono::Utc::now().to_rfc3339();
        let doc_id = uuid::Uuid::new_v4().to_string();

        let document = Document {
            id: doc_id.clone(),
            name: name.clone(),
            path: doc_path.clone(),
            content: String::new(),
            parent_id: parent_id.clone(),
            created: now.clone(),
            modified: now,
            ..Default::default()
        };
        self.project.documents.insert(doc_id.clone(), document);

        let node = TreeNode::Document { id: doc_id, name: name.clone(), path: doc_path };
        match parent_id {
            Some(pid) => hierarchy::add_child_to_folder(&mut self.project.hierarchy, &pid, node)
                .map_err(|e| anyhow!("Add to folder failed: {:?}", e))?,
            None => hierarchy::add_document_to_hierarchy(&mut self.project.hierarchy, node),
        }

        writer::write_project(&mut self.project).map_err(|e| anyhow!("Write failed: {:?}", e))?;
        self.rebuild_binder();
        self.status = format!("Created document: {}", name);
        Ok(())
    }

    fn create_folder_in_project(&mut self, name: String, parent_id: Option<String>) -> Result<()> {
        let folder_id = uuid::Uuid::new_v4().to_string();
        let node = TreeNode::Folder { id: folder_id.clone(), name: name.clone(), children: Vec::new() };
        match parent_id {
            Some(pid) => hierarchy::add_child_to_folder(&mut self.project.hierarchy, &pid, node)
                .map_err(|e| anyhow!("Add to folder failed: {:?}", e))?,
            None => hierarchy::add_document_to_hierarchy(&mut self.project.hierarchy, node),
        }
        self.expanded.insert(folder_id);
        writer::write_project(&mut self.project).map_err(|e| anyhow!("Write failed: {:?}", e))?;
        self.rebuild_binder();
        self.status = format!("Created folder: {}", name);
        Ok(())
    }

    fn handle_confirm_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.mode = Mode::Normal;
                self.should_quit = true;
            }
            _ => {
                self.mode = Mode::Normal;
                self.status = "Canceled quit".to_string();
            }
        }
        Ok(())
    }

    fn try_quit(&mut self) {
        if self.dirty {
            self.mode = Mode::Confirm;
            self.status = "Unsaved changes. Quit anyway? (y/N)".to_string();
        } else {
            self.should_quit = true;
        }
    }

    fn open_document(&mut self, doc_id: &str) {
        if self.dirty {
            let _ = self.save_active_doc();
        }
        let (content, name) = match self.project.documents.get(doc_id) {
            Some(doc) => (doc.content.clone(), doc.name.clone()),
            None => return,
        };
        self.load_markdown(&content);
        self.active_doc_id = Some(doc_id.to_string());
        self.dirty = false;
        self.status = format!("Opened: {}", name);
    }

    fn save_active_doc(&mut self) -> Result<()> {
        let doc_id = match &self.active_doc_id {
            Some(id) => id.clone(),
            None => return Ok(()),
        };
        let md = self.editor_content_string();
        if let Some(doc) = self.project.documents.get_mut(&doc_id) {
            doc.content = md.clone();
            doc.modified = chrono::Utc::now().to_rfc3339();
        }
        writer::write_project(&mut self.project).map_err(|e| anyhow!("Write failed: {:?}", e))?;
        self.dirty = false;
        let word_count = convert::count_words(&md);
        self.status = format!("Saved. {} words.", word_count);
        Ok(())
    }

    fn save_revision(&mut self, message: &str) -> Result<()> {
        if self.dirty {
            self.save_active_doc()?;
        }
        match git::save_revision(&self.project_path, message) {
            Ok(rev) => {
                let short = rev.short_id.clone();
                // After a named revision: push to backup if configured.
                let backup_msg = match read_backup_directory() {
                    Some(dir) => match git::push_backup(&self.project_path, &dir) {
                        Ok(()) => " · backed up".to_string(),
                        Err(e) => format!(" · backup failed: {:?}", e),
                    },
                    None => String::new(),
                };
                self.status = format!("Revision saved: {} ({}){}", message, short, backup_msg);
            }
            Err(e) => {
                self.status = format!("Revision failed: {:?}", e);
            }
        }
        Ok(())
    }

    pub fn active_doc_name(&self) -> Option<String> {
        self.active_doc_id
            .as_ref()
            .and_then(|id| self.project.documents.get(id))
            .map(|d| d.name.clone())
    }

    pub fn word_count(&self) -> usize {
        if self.active_doc_id.is_none() {
            return 0;
        }
        let md = self.editor_content_string();
        convert::count_words(&md)
    }
}

impl App<'_> {
    #[allow(dead_code)]
    fn _ctx(&self) {
        let _: fn() = || {
            let _: Result<()> = Err(anyhow!("x")).context("y");
        };
    }
}

/// Read the backup directory from the shared settings file at
/// `~/.config/chickenscratch/settings.json`. Returns None if unset, file missing,
/// or parse fails.
fn read_backup_directory() -> Option<PathBuf> {
    let mut path = dirs::config_dir()?;
    path.push("chickenscratch");
    path.push("settings.json");
    let data = std::fs::read_to_string(&path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&data).ok()?;
    v.get("backup")?
        .get("backup_directory")?
        .as_str()
        .map(PathBuf::from)
}

/// Ensure start ≤ end in (row, col) order.
fn normalize_selection(sel: (usize, usize, usize, usize)) -> (usize, usize, usize, usize) {
    let (sr, sc, er, ec) = sel;
    if (sr, sc) <= (er, ec) {
        (sr, sc, er, ec)
    } else {
        (er, ec, sr, sc)
    }
}

/// Safely slice a string at a character boundary (col is a char index, not byte).
fn char_byte_index(s: &str, col: usize) -> usize {
    s.char_indices().nth(col).map(|(i, _)| i).unwrap_or(s.len())
}

/// Wrap the selected region of `lines` (from row sr, col sc to row er, col ec)
/// with `open_tag` at the start and `close_tag` at the end. Mutates `lines` in place.
/// Returns false if the selection is empty.
fn wrap_selection_in_lines(
    lines: &mut [String],
    sr: usize,
    sc: usize,
    er: usize,
    ec: usize,
    open_tag: &str,
    close_tag: &str,
) -> bool {
    if lines.is_empty() || sr >= lines.len() {
        return false;
    }
    if sr == er && sc == ec {
        return false;
    }

    // Single-line selection
    if sr == er {
        let line = lines[sr].clone();
        let start_b = char_byte_index(&line, sc);
        let end_b = char_byte_index(&line, ec);
        if start_b >= end_b {
            return false;
        }
        let new_line = format!(
            "{}{}{}{}{}",
            &line[..start_b],
            open_tag,
            &line[start_b..end_b],
            close_tag,
            &line[end_b..]
        );
        lines[sr] = new_line;
        return true;
    }

    // Multi-line selection
    let first = lines[sr].clone();
    let last_idx = er.min(lines.len() - 1);
    let last = lines[last_idx].clone();
    let start_b = char_byte_index(&first, sc);
    let end_b = char_byte_index(&last, ec);

    // Insert open_tag into first line at start_b
    lines[sr] = format!("{}{}{}", &first[..start_b], open_tag, &first[start_b..]);

    // Insert close_tag into last line at end_b (remember first-line insertion
    // shifted nothing in the last line since they're different lines).
    lines[last_idx] = format!("{}{}{}", &last[..end_b], close_tag, &last[end_b..]);
    true
}

/// Strip all HTML tags and entities, returning plain text. Used to get the
/// anchor text of a comment span for display in the comments panel.
fn strip_tags_inline(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.trim().to_string()
}

/// Remove the `<span class="comment" data-comment-id="{id}">...</span>` wrapper
/// for the given id, preserving the inner text/HTML.
fn strip_comment_span(html: &str, id: &str) -> String {
    let needle = format!("data-comment-id=\"{}\"", id);
    let opening_idx = match html.find(&needle) {
        Some(i) => i,
        None => return html.to_string(),
    };
    // Walk back to find `<span`
    let tag_start = html[..opening_idx].rfind("<span").unwrap_or(opening_idx);
    let tag_end = match html[tag_start..].find('>') {
        Some(e) => tag_start + e + 1,
        None => return html.to_string(),
    };
    // Inner content starts at tag_end; find matching `</span>`
    let inner_end = match html[tag_end..].find("</span>") {
        Some(e) => tag_end + e,
        None => return html.to_string(),
    };
    let close_end = inner_end + "</span>".len();

    let mut out = String::with_capacity(html.len());
    out.push_str(&html[..tag_start]);
    out.push_str(&html[tag_end..inner_end]);
    out.push_str(&html[close_end..]);
    out
}
