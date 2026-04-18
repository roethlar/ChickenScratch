use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, Focus, Mode};

pub fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // Vertical split: main content / status bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(area);

    // Horizontal split: binder / editor
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(32), Constraint::Min(20)])
        .split(chunks[0]);

    render_binder(f, main_chunks[0], app);
    render_editor(f, main_chunks[1], app);
    render_status(f, chunks[1], app);

    // Overlays
    if app.mode == Mode::RevisionPrompt {
        render_prompt(f, area, "Save revision with message:", &app.prompt_input);
    }
    if app.mode == Mode::Confirm {
        render_prompt(f, area, "Unsaved changes. Quit anyway?", "[y/N]");
    }
}

fn render_binder(f: &mut Frame, area: Rect, app: &App) {
    let title = format!(" {} ", app.project.name);
    let border_style = if app.focus == Focus::Binder {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let items: Vec<ListItem> = app
        .binder_items
        .iter()
        .map(|item| {
            let indent = "  ".repeat(item.depth);
            let icon = if item.is_folder {
                if app.expanded.contains(&item.id) { "▾ " } else { "▸ " }
            } else {
                "  "
            };
            let label = format!("{}{}{}", indent, icon, item.name);
            let style = if item.is_folder {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else if Some(&item.id) == app.active_doc_id.as_ref() {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(Span::styled(label, style)))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).border_style(border_style).title(title))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("");

    let mut state = ListState::default();
    if !app.binder_items.is_empty() {
        state.select(Some(app.binder_selected));
    }
    f.render_stateful_widget(list, area, &mut state);
}

fn render_editor(f: &mut Frame, area: Rect, app: &mut App) {
    let border_style = if app.focus == Focus::Editor {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = match app.active_doc_name() {
        Some(name) => {
            let marker = if app.dirty { "●" } else { " " };
            format!(" {} {} ", marker, name)
        }
        None => " No document open ".to_string(),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title);

    if app.active_doc_id.is_none() {
        let inner = Paragraph::new(
            "Select a document from the binder (←) and press Enter.\n\n\
             Keys:\n  ↑↓  navigate  \n  Enter  open document / expand folder\n  \
             Tab  switch focus\n  Ctrl+S  save\n  Ctrl+R  save revision\n  \
             Esc  back to binder\n  q  quit",
        )
        .block(block)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::DarkGray));
        f.render_widget(inner, area);
        return;
    }

    app.editor.set_block(block);
    f.render_widget(&app.editor, area);
}

fn render_status(f: &mut Frame, area: Rect, app: &App) {
    let left = if let Some(name) = app.active_doc_name() {
        format!("{} · {} words{}", name, app.word_count(), if app.dirty { " · modified" } else { "" })
    } else {
        app.status.clone()
    };

    let right = match app.focus {
        Focus::Binder => "[Binder]".to_string(),
        Focus::Editor => "[Editor]".to_string(),
    };

    // If there's a transient status message and no active doc, show it; otherwise
    // show doc info on the left and status on the right of doc info.
    let text = if app.active_doc_id.is_some() && app.status != "Ready. ?=help  Tab=switch pane  q=quit" {
        format!("{} · {}", left, app.status)
    } else {
        left
    };

    let padded = format!(" {:<width$}{}", text, right, width = area.width.saturating_sub(right.len() as u16 + 2) as usize);
    let para = Paragraph::new(padded).style(Style::default().bg(Color::DarkGray).fg(Color::White));
    f.render_widget(para, area);
}

fn render_prompt(f: &mut Frame, area: Rect, title: &str, value: &str) {
    let w = 60.min(area.width - 4);
    let h = 5;
    let x = (area.width - w) / 2;
    let y = (area.height - h) / 2;
    let popup = Rect { x, y, width: w, height: h };
    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(format!(" {} ", title));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let content = Paragraph::new(format!("> {}", value))
        .style(Style::default().fg(Color::White));
    f.render_widget(content, inner);
}
