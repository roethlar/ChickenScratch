use anyhow::{anyhow, Context, Result};
use chickenscratch_core::core::git;
use chickenscratch_core::core::project::{reader, writer};
use chickenscratch_core::{Project, TreeNode};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::Backend, Terminal};
use std::path::PathBuf;
use std::time::Duration;
use tui_textarea::TextArea;

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
    Confirm,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum ViewMode {
    /// Edit markdown (default)
    Markdown,
    /// Edit raw HTML source
    Source,
    /// Read-only formatted preview
    Formatted,
}

impl ViewMode {
    pub fn label(self) -> &'static str {
        match self {
            ViewMode::Markdown => "markdown",
            ViewMode::Source => "source",
            ViewMode::Formatted => "preview",
        }
    }

    pub fn next(self) -> ViewMode {
        match self {
            ViewMode::Markdown => ViewMode::Source,
            ViewMode::Source => ViewMode::Formatted,
            ViewMode::Formatted => ViewMode::Markdown,
        }
    }
}

/// A flat list of (depth, node_id, display_name, is_folder) for the binder view.
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

    pub status: String,
    pub prompt_input: String,
    pub should_quit: bool,
}

impl<'a> App<'a> {
    pub fn new(project_path: PathBuf) -> Result<Self> {
        let project = reader::read_project(&project_path)
            .map_err(|e| anyhow!("Failed to read project: {:?}", e))?;

        let mut expanded = std::collections::HashSet::new();
        // Expand top-level folders by default
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
            view_mode: ViewMode::Markdown,
            status: "Ready. ?=help  Tab=switch pane  q=quit".to_string(),
            prompt_input: String::new(),
            should_quit: false,
        };
        app.rebuild_binder();
        app.configure_editor();
        Ok(app)
    }

    fn configure_editor(&mut self) {
        self.editor.set_cursor_line_style(ratatui::style::Style::default());
        self.editor.set_line_number_style(
            ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray),
        );
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

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
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
            Mode::Confirm => self.handle_confirm_key(key),
            Mode::Normal => self.handle_normal_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> Result<()> {
        // Global shortcuts
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('s') => { self.save_active_doc()?; return Ok(()); }
                KeyCode::Char('r') => {
                    self.prompt_input = format!("Revision {}", chrono::Utc::now().format("%Y-%m-%d %H:%M"));
                    self.mode = Mode::RevisionPrompt;
                    return Ok(());
                }
                KeyCode::Char('t') => {
                    self.cycle_view_mode();
                    return Ok(());
                }
                KeyCode::Char('q') => { self.try_quit(); return Ok(()); }
                _ => {}
            }
        }

        // Tab to switch pane
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
                self.status = "Keys: ↑↓=nav  Enter=open  Space=expand/collapse  Tab=editor  Ctrl+S=save  Ctrl+R=revision  q=quit".to_string();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.binder_selected + 1 < self.binder_items.len() {
                    self.binder_selected += 1;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.binder_selected > 0 {
                    self.binder_selected -= 1;
                }
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
        // Formatted view is read-only — allow only scroll/cursor nav
        if self.view_mode == ViewMode::Formatted {
            match key.code {
                KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right
                | KeyCode::PageUp | KeyCode::PageDown | KeyCode::Home | KeyCode::End => {
                    self.editor.input(key);
                }
                _ => {}
            }
            return Ok(());
        }
        // Editable mode — pass to textarea
        let changed = self.editor.input(key);
        if changed {
            self.dirty = true;
        }
        Ok(())
    }

    fn cycle_view_mode(&mut self) {
        if self.active_doc_id.is_none() {
            self.view_mode = self.view_mode.next();
            self.status = format!("View: {}", self.view_mode.label());
            return;
        }

        // Get current content as raw HTML by converting from whatever mode we're in
        let current_html = self.current_content_as_html();
        let new_mode = self.view_mode.next();
        self.load_content_for_mode(&current_html, new_mode);
        self.view_mode = new_mode;
        self.status = format!("View: {}", new_mode.label());
    }

    fn current_content_as_html(&self) -> String {
        let text = self.editor.lines().join("\n");
        match self.view_mode {
            ViewMode::Markdown => convert::markdown_to_html(&text),
            ViewMode::Source => text,
            ViewMode::Formatted => {
                // Formatted is read-only, so the underlying HTML is whatever's in the doc
                self.active_doc_id
                    .as_ref()
                    .and_then(|id| self.project.documents.get(id))
                    .map(|d| d.content.clone())
                    .unwrap_or_default()
            }
        }
    }

    fn load_content_for_mode(&mut self, html: &str, mode: ViewMode) {
        let text = match mode {
            ViewMode::Markdown => convert::html_to_markdown(html),
            ViewMode::Source => pretty_print_html(html),
            ViewMode::Formatted => {
                // Formatted view uses lines of plain-ish text; rendering handles the styling
                convert::html_to_markdown(html)
            }
        };
        let lines: Vec<String> = if text.is_empty() {
            vec![String::new()]
        } else {
            text.lines().map(String::from).collect()
        };
        self.editor = TextArea::new(lines);
        self.configure_editor();
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
            KeyCode::Backspace => { self.prompt_input.pop(); }
            KeyCode::Char(c) => { self.prompt_input.push(c); }
            _ => {}
        }
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
        // Save current doc first if dirty
        if self.dirty {
            let _ = self.save_active_doc();
        }
        let (content, name) = match self.project.documents.get(doc_id) {
            Some(doc) => (doc.content.clone(), doc.name.clone()),
            None => return,
        };
        let mode = self.view_mode;
        self.load_content_for_mode(&content, mode);
        self.active_doc_id = Some(doc_id.to_string());
        self.dirty = false;
        self.status = format!("Opened: {}", name);
    }

    fn save_active_doc(&mut self) -> Result<()> {
        let doc_id = match &self.active_doc_id {
            Some(id) => id.clone(),
            None => return Ok(()),
        };

        // Formatted mode is read-only, but save should still work using the
        // source of truth from the project (no edits possible anyway).
        let html = self.current_content_as_html();
        let word_source = if self.view_mode == ViewMode::Source {
            convert::html_to_markdown(&html)
        } else if self.view_mode == ViewMode::Formatted {
            convert::html_to_markdown(&html)
        } else {
            self.editor.lines().join("\n")
        };

        if let Some(doc) = self.project.documents.get_mut(&doc_id) {
            doc.content = html;
            doc.modified = chrono::Utc::now().to_rfc3339();
        }

        writer::write_project(&mut self.project)
            .map_err(|e| anyhow!("Write failed: {:?}", e))?;
        self.dirty = false;

        let word_count = convert::count_words(&word_source);
        self.status = format!("Saved. {} words.", word_count);
        Ok(())
    }

    fn save_revision(&mut self, message: &str) -> Result<()> {
        // Ensure any current edits are saved first
        if self.dirty {
            self.save_active_doc()?;
        }
        match git::save_revision(&self.project_path, message) {
            Ok(rev) => {
                self.status = format!("Revision saved: {} ({})", message, &rev.short_id);
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
        let md = self.editor.lines().join("\n");
        convert::count_words(&md)
    }
}

impl App<'_> {
    // Silence "context" unused import warning while still using it via anyhow
    #[allow(dead_code)]
    fn _ctx(&self) {
        let _: fn() = || {
            let _: Result<()> = Err(anyhow!("x")).context("y");
        };
    }
}

/// Minimal HTML pretty-printer — newlines between block elements.
fn pretty_print_html(html: &str) -> String {
    let html = html.trim();
    if html.is_empty() {
        return String::new();
    }
    // Insert newlines before/after common block tags so raw HTML is readable.
    let mut s = html
        .replace("><", ">\n<")
        .replace("<p>", "<p>")
        .replace("</p>", "</p>\n");
    // Collapse triple+ newlines
    while s.contains("\n\n\n") {
        s = s.replace("\n\n\n", "\n\n");
    }
    s
}
