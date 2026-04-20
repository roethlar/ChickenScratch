use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, Focus, Mode, ViewMode};

pub fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(area);

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(32), Constraint::Min(20)])
        .split(chunks[0]);

    render_binder(f, main_chunks[0], app);
    render_editor(f, main_chunks[1], app);
    render_status(f, chunks[1], app);

    if app.mode == Mode::RevisionPrompt {
        render_prompt(f, area, "Save revision with message:", &app.prompt_input);
    }
    if app.mode == Mode::Confirm {
        render_prompt(f, area, "Unsaved changes. Quit anyway?", "[y/N]");
    }
    if app.mode == Mode::Comments {
        render_comments_overlay(f, area, app);
    }
    if app.mode == Mode::CommentEdit {
        render_prompt(
            f,
            area,
            "Edit comment (Enter to save, Esc to cancel):",
            &app.prompt_input,
        );
    }
}

fn render_comments_overlay(f: &mut Frame, area: Rect, app: &App) {
    let comments = app.current_comments();
    let w = 70.min(area.width.saturating_sub(4));
    let h = (area.height.saturating_sub(4)).min(24);
    let x = (area.width.saturating_sub(w)) / 2;
    let y = (area.height.saturating_sub(h)) / 2;
    let popup = Rect {
        x,
        y,
        width: w,
        height: h,
    };
    f.render_widget(Clear, popup);

    let title = format!(
        " Comments ({})  ↑↓=nav  n=doc note  e/Enter=edit  r=resolve  d=delete  Esc=close ",
        comments.len()
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title);

    if comments.is_empty() {
        let inner = Paragraph::new(
            "No comments in this document yet.\n\n\
             Press n (or Enter) to add a document-level note.\n\n\
             Comments anchored to specific text are created in the Tauri app\n\
             by selecting text and pressing the comment button.",
        )
        .block(block)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::DarkGray));
        f.render_widget(inner, popup);
        return;
    }

    let inner_area = block.inner(popup);
    f.render_widget(block, popup);

    let items: Vec<ListItem> = comments
        .iter()
        .map(|c| {
            let anchor = app.comment_anchor_text(&c.id);
            let preview = if anchor.len() > 40 {
                format!("{}...", &anchor.chars().take(40).collect::<String>())
            } else {
                anchor
            };
            let marker = if c.resolved { "✓" } else { "●" };
            let marker_color = if c.resolved {
                Color::DarkGray
            } else {
                Color::Yellow
            };
            let body_preview = c
                .body
                .lines()
                .next()
                .unwrap_or("")
                .chars()
                .take(60)
                .collect::<String>();
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(format!("{} ", marker), Style::default().fg(marker_color)),
                    Span::styled(
                        format!("\"{}\"", preview),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]),
                Line::from(Span::raw(format!("  {}", body_preview))),
            ])
        })
        .collect();

    let list = List::new(items).highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    );
    let mut state = ListState::default();
    state.select(Some(app.comments_selected));
    f.render_stateful_widget(list, inner_area, &mut state);
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
                if app.expanded.contains(&item.id) {
                    "▾ "
                } else {
                    "▸ "
                }
            } else {
                "  "
            };
            let label = format!("{}{}{}", indent, icon, item.name);
            let style = if item.is_folder {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if Some(&item.id) == app.active_doc_id.as_ref() {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(Span::styled(label, style)))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(title),
        )
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

    let wrap_label = if app.wrap { "wrap" } else { "no-wrap" };
    let title = match app.active_doc_name() {
        Some(name) => {
            let marker = if app.dirty { "●" } else { " " };
            format!(
                " {} {}  [{}]  [{}]  Ctrl+T/Ctrl+W ",
                marker,
                name,
                app.view_mode.label(),
                wrap_label
            )
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
             Keys:\n  ↑↓       navigate\n  Enter    open document / expand folder\n  \
             Tab      switch focus\n  Ctrl+S   save\n  Ctrl+R   save revision\n  \
             Ctrl+T   toggle edit/preview\n  \
             Ctrl+W   toggle word wrap\n  \
             F2       comments overlay\n  \
             F3       anchor comment on selected text\n  \
             Esc      back to binder\n  q        quit",
        )
        .block(block)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::DarkGray));
        f.render_widget(inner, area);
        return;
    }

    if app.view_mode == ViewMode::Preview {
        render_formatted_preview(f, area, app, block);
        return;
    }

    app.editor.set_block(block);
    f.render_widget(&app.editor, area);
}

fn render_formatted_preview(f: &mut Frame, area: Rect, app: &App, block: Block) {
    let md = app.editor_content_string();
    let lines = render_markdown_as_lines(&md);
    let para = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(para, area);
}

fn render_status(f: &mut Frame, area: Rect, app: &App) {
    let left = if let Some(name) = app.active_doc_name() {
        format!(
            "{} · {} words{}",
            name,
            app.word_count(),
            if app.dirty { " · modified" } else { "" }
        )
    } else {
        app.status.clone()
    };

    let right = match app.focus {
        Focus::Binder => "[Binder]".to_string(),
        Focus::Editor => "[Editor]".to_string(),
    };

    let text =
        if app.active_doc_id.is_some() && app.status != "Ready. ?=help  Tab=switch pane  q=quit" {
            format!("{} · {}", left, app.status)
        } else {
            left
        };

    let padded = format!(
        " {:<width$}{}",
        text,
        right,
        width = area.width.saturating_sub(right.len() as u16 + 2) as usize
    );
    let para = Paragraph::new(padded).style(Style::default().bg(Color::DarkGray).fg(Color::White));
    f.render_widget(para, area);
}

fn render_prompt(f: &mut Frame, area: Rect, title: &str, value: &str) {
    let w = 60.min(area.width - 4);
    let h = 5;
    let x = (area.width - w) / 2;
    let y = (area.height - h) / 2;
    let popup = Rect {
        x,
        y,
        width: w,
        height: h,
    };
    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(format!(" {} ", title));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let content = Paragraph::new(format!("> {}", value)).style(Style::default().fg(Color::White));
    f.render_widget(content, inner);
}

/// Convert markdown into styled ratatui lines for terminal rendering.
fn render_markdown_as_lines(md: &str) -> Vec<Line<'static>> {
    use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

    let parser = Parser::new(md);
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current: Vec<Span<'static>> = Vec::new();

    let mut heading_level: Option<HeadingLevel> = None;
    let mut in_emph = false;
    let mut in_strong = false;
    let mut in_code = false;
    let mut in_blockquote = false;
    let mut list_depth: usize = 0;
    let mut ordered_counters: Vec<usize> = Vec::new();

    let push_line = |lines: &mut Vec<Line<'static>>, buf: &mut Vec<Span<'static>>| {
        if buf.is_empty() {
            lines.push(Line::from(""));
        } else {
            lines.push(Line::from(std::mem::take(buf)));
        }
    };

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => heading_level = Some(level),
                Tag::Emphasis => in_emph = true,
                Tag::Strong => in_strong = true,
                Tag::BlockQuote(_) => in_blockquote = true,
                Tag::CodeBlock(_) => in_code = true,
                Tag::List(start) => {
                    list_depth += 1;
                    ordered_counters.push(start.unwrap_or(0) as usize);
                }
                Tag::Item => {
                    let indent = "  ".repeat(list_depth.saturating_sub(1));
                    let marker = if let Some(last) = ordered_counters.last_mut() {
                        if *last > 0 {
                            let m = format!("{}{}. ", indent, *last);
                            *last += 1;
                            m
                        } else {
                            format!("{}• ", indent)
                        }
                    } else {
                        format!("{}• ", indent)
                    };
                    current.push(Span::styled(marker, Style::default().fg(Color::Yellow)));
                }
                Tag::Link { .. } => {
                    current.push(Span::styled("[", Style::default().fg(Color::Blue)));
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Heading(_) => {
                    push_line(&mut lines, &mut current);
                    lines.push(Line::from(""));
                    heading_level = None;
                }
                TagEnd::Paragraph => {
                    push_line(&mut lines, &mut current);
                    lines.push(Line::from(""));
                }
                TagEnd::Emphasis => in_emph = false,
                TagEnd::Strong => in_strong = false,
                TagEnd::BlockQuote(_) => {
                    push_line(&mut lines, &mut current);
                    lines.push(Line::from(""));
                    in_blockquote = false;
                }
                TagEnd::CodeBlock => {
                    push_line(&mut lines, &mut current);
                    lines.push(Line::from(""));
                    in_code = false;
                }
                TagEnd::List(_) => {
                    list_depth = list_depth.saturating_sub(1);
                    ordered_counters.pop();
                    if list_depth == 0 {
                        lines.push(Line::from(""));
                    }
                }
                TagEnd::Item => {
                    push_line(&mut lines, &mut current);
                }
                TagEnd::Link => {
                    current.push(Span::styled("]", Style::default().fg(Color::Blue)));
                }
                _ => {}
            },
            Event::Text(text) => {
                let text = text.to_string();
                let mut style = Style::default();
                if let Some(level) = heading_level {
                    let (color, add) = match level {
                        HeadingLevel::H1 => (Color::Magenta, Modifier::BOLD | Modifier::UNDERLINED),
                        HeadingLevel::H2 => (Color::Cyan, Modifier::BOLD),
                        HeadingLevel::H3 => (Color::Blue, Modifier::BOLD),
                        _ => (Color::Gray, Modifier::BOLD),
                    };
                    style = style.fg(color).add_modifier(add);
                }
                if in_strong {
                    style = style.add_modifier(Modifier::BOLD);
                }
                if in_emph {
                    style = style.add_modifier(Modifier::ITALIC);
                }
                if in_code {
                    style = style.fg(Color::Green).bg(Color::Black);
                }
                if in_blockquote {
                    style = style.fg(Color::DarkGray).add_modifier(Modifier::ITALIC);
                    if current.is_empty() {
                        current.push(Span::styled("│ ", Style::default().fg(Color::DarkGray)));
                    }
                }
                current.push(Span::styled(text, style));
            }
            Event::Code(code) => {
                current.push(Span::styled(
                    code.to_string(),
                    Style::default().fg(Color::Green).bg(Color::Black),
                ));
            }
            Event::SoftBreak => {
                current.push(Span::raw(" "));
            }
            Event::HardBreak => {
                push_line(&mut lines, &mut current);
            }
            Event::Rule => {
                push_line(&mut lines, &mut current);
                lines.push(Line::from(Span::styled(
                    "─".repeat(40),
                    Style::default().fg(Color::DarkGray),
                )));
                lines.push(Line::from(""));
            }
            _ => {}
        }
    }

    if !current.is_empty() {
        push_line(&mut lines, &mut current);
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "(empty)",
            Style::default().fg(Color::DarkGray),
        )));
    }

    lines
}
