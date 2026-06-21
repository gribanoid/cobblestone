// SPDX-License-Identifier: MIT

use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Terminal,
};

use cobblestone_core::{Note, Store};

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

enum UiMode {
    Normal,
    Search,
    Confirm { note_name: String },
    NewNote  { title_buf: String },
    Message  { text: String },
}

struct App {
    store:    Store,
    notes:    Vec<Note>,
    selected: usize,
    scroll:   u16,
    mode:     UiMode,
    search:   String,
}

impl App {
    fn new(store: Store) -> Result<Self> {
        let notes = store.list_notes()?;
        Ok(Self {
            store,
            notes,
            selected: 0,
            scroll: 0,
            mode: UiMode::Normal,
            search: String::new(),
        })
    }

    fn refresh(&mut self) -> Result<()> {
        let prev = self.current_note_name();
        self.notes = self.store.list_notes()?;
        // Try to keep the cursor on the same note after refresh.
        if let Some(name) = prev
            && let Some(i) = self.notes.iter().position(|n| n.name == name)
        {
            self.selected = i;
            return Ok(());
        }
        self.selected = self.selected.min(self.notes.len().saturating_sub(1));
        Ok(())
    }

    fn current_note_name(&self) -> Option<String> {
        self.visible_notes()
            .get(self.selected)
            .map(|n| n.name.clone())
    }

    fn visible_notes(&self) -> Vec<&Note> {
        if self.search.is_empty() {
            self.notes.iter().collect()
        } else {
            let q = self.search.to_lowercase();
            self.notes
                .iter()
                .filter(|n| {
                    n.title.to_lowercase().contains(&q)
                        || n.name.contains(&q)
                        || n.preview.to_lowercase().contains(&q)
                })
                .collect()
        }
    }

    fn current_content(&self) -> String {
        let notes = self.visible_notes();
        if notes.is_empty() {
            if self.notes.is_empty() {
                return "No notes yet.\n\nPress  n  to create your first note.".to_string();
            }
            return "No results for that search.".to_string();
        }
        let note = &notes[self.selected];
        self.store
            .read(&note.name)
            .unwrap_or_else(|e| format!("(error reading note: {e})"))
    }

    fn nav_down(&mut self) {
        let len = self.visible_notes().len();
        if len == 0 { return; }
        if self.selected + 1 < len {
            self.selected += 1;
            self.scroll = 0;
        }
    }

    fn nav_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.scroll = 0;
        }
    }

    fn show_error(&mut self, e: anyhow::Error) {
        self.mode = UiMode::Message { text: format!("Error: {e}") };
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn run(store: Store) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend  = CrosstermBackend::new(io::stdout());
    let mut term = Terminal::new(backend)?;

    let mut app    = App::new(store)?;
    let result     = event_loop(&mut term, &mut app);

    // Always restore the terminal — even if event_loop returned an error.
    let _ = disable_raw_mode();
    let _ = execute!(term.backend_mut(), LeaveAlternateScreen);
    let _ = term.show_cursor();

    result
}

// ---------------------------------------------------------------------------
// Event loop
// ---------------------------------------------------------------------------

fn event_loop(
    term: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        term.draw(|f| draw(f, app))?;

        if !event::poll(Duration::from_millis(200))? {
            continue;
        }

        let Event::Key(key) = event::read()? else { continue };

        match &app.mode {
            // ------------------------------------------------------------------
            UiMode::Normal => {
                match (key.code, key.modifiers) {
                    (KeyCode::Char('q'), _)
                    | (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,

                    (KeyCode::Down | KeyCode::Char('j'), _) => app.nav_down(),
                    (KeyCode::Up   | KeyCode::Char('k'), _) => app.nav_up(),

                    (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                        app.scroll = app.scroll.saturating_add(8);
                    }
                    (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                        app.scroll = app.scroll.saturating_sub(8);
                    }
                    (KeyCode::PageDown, _) => app.scroll = app.scroll.saturating_add(8),
                    (KeyCode::PageUp,   _) => app.scroll = app.scroll.saturating_sub(8),

                    (KeyCode::Char('/'), _) => {
                        app.mode    = UiMode::Search;
                        app.search  = String::new();
                        app.selected = 0;
                    }

                    (KeyCode::Char('n'), _) => {
                        app.mode = UiMode::NewNote { title_buf: String::new() };
                    }

                    (KeyCode::Char('e'), _) => {
                        if let Some(name) = app.current_note_name() {
                            leave_tui(term)?;
                            let result = crate::commands::cmd_edit(&app.store, &name);
                            enter_tui(term)?;
                            if let Err(e) = result {
                                app.show_error(e);
                            } else if let Err(e) = app.refresh() {
                                app.show_error(e);
                            }
                        }
                    }

                    (KeyCode::Char('D'), _) => {
                        if let Some(name) = app.current_note_name() {
                            app.mode = UiMode::Confirm { note_name: name };
                        }
                    }

                    _ => {}
                }
            }

            // ------------------------------------------------------------------
            UiMode::Search => {
                match key.code {
                    KeyCode::Esc | KeyCode::Enter => {
                        app.mode = UiMode::Normal;
                    }
                    KeyCode::Backspace => {
                        app.search.pop();
                        app.selected = 0;
                    }
                    KeyCode::Char(c) => {
                        app.search.push(c);
                        app.selected = 0;
                    }
                    KeyCode::Down => app.nav_down(),
                    KeyCode::Up   => app.nav_up(),
                    _ => {}
                }
            }

            // ------------------------------------------------------------------
            UiMode::NewNote { .. } => {
                // Reborrow mutably inside the arm.
                let UiMode::NewNote { ref mut title_buf } = app.mode else { unreachable!() };
                match key.code {
                    KeyCode::Esc => {
                        app.mode = UiMode::Normal;
                    }
                    KeyCode::Enter => {
                        let title = title_buf.trim().to_string();
                        app.mode = UiMode::Normal;
                        if !title.is_empty() {
                            let name = cobblestone_core::slugify(&title);
                            if app.store.exists(&name) {
                                app.mode = UiMode::Message {
                                    text: format!("Note '{name}' already exists. Press 'e' to edit."),
                                };
                            } else {
                                let content = format!("# {title}\n\n");
                                if let Err(e) = app.store.write(&name, &content) {
                                    app.show_error(e);
                                } else {
                                    let _ = app.refresh();
                                    if let Some(i) = app.notes.iter().position(|n| n.name == name) {
                                        app.selected = i;
                                    }
                                    leave_tui(term)?;
                                    let result = crate::commands::cmd_edit(&app.store, &name);
                                    enter_tui(term)?;
                                    if let Err(e) = result {
                                        app.show_error(e);
                                    } else {
                                        let _ = app.refresh();
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Backspace => { title_buf.pop(); }
                    KeyCode::Char(c)   => { title_buf.push(c); }
                    _ => {}
                }
            }

            // ------------------------------------------------------------------
            UiMode::Confirm { note_name } => {
                let name = note_name.clone();
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        if let Err(e) = app.store.delete(&name) {
                            app.show_error(e);
                        } else if let Err(e) = app.refresh() {
                            app.show_error(e);
                        } else {
                            app.mode = UiMode::Normal;
                        }
                    }
                    _ => {
                        app.mode = UiMode::Normal;
                    }
                }
            }

            // ------------------------------------------------------------------
            UiMode::Message { .. } => {
                app.mode = UiMode::Normal;
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Terminal helpers
// ---------------------------------------------------------------------------

fn leave_tui(term: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

fn enter_tui(term: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    enable_raw_mode()?;
    execute!(term.backend_mut(), EnterAlternateScreen)?;
    term.clear()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

fn draw(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();

    let h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(28), Constraint::Min(0)])
        .split(area);

    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(h[1]);

    draw_sidebar(f, app, h[0]);
    draw_content(f, app, v[0]);
    draw_status(f, app, v[1]);
}

fn draw_sidebar(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let visible = app.visible_notes();

    let title = match &app.mode {
        UiMode::Search => format!(" / {} ", app.search),
        _              => format!(" 🪨  Notes ({}) ", app.notes.len()),
    };

    let items: Vec<ListItem> = visible
        .iter()
        .map(|note| {
            ListItem::new(vec![
                Line::from(Span::styled(
                    note.title.clone(),
                    Style::default().fg(Color::White),
                )),
                Line::from(Span::styled(
                    format!("  {}", note.modified),
                    Style::default().fg(Color::DarkGray),
                )),
            ])
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
                .bg(Color::Rgb(40, 40, 70)),
        )
        .highlight_symbol("> ");

    // ListState drives the actual selection highlight.
    let mut list_state = ListState::default();
    if !visible.is_empty() {
        list_state.select(Some(app.selected));
    }
    f.render_stateful_widget(list, area, &mut list_state);

    // ── Overlay popups ────────────────────────────────────────────────────────

    if let UiMode::NewNote { title_buf } = &app.mode {
        let popup = centered_rect(60, 20, f.area());
        let para  = Paragraph::new(format!("New note title:\n\n> {title_buf}_"))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" ✏  New Note ")
                    .style(Style::default().fg(Color::Yellow)),
            )
            .wrap(Wrap { trim: false });
        f.render_widget(Clear, popup);
        f.render_widget(para, popup);
    }

    if let UiMode::Confirm { note_name } = &app.mode {
        let popup = centered_rect(50, 15, f.area());
        let para  = Paragraph::new(format!(
            "Delete note '{note_name}'?\n\n  y  →  Yes, delete\n  any  →  Cancel"
        ))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" ⚠  Confirm Delete ")
                .style(Style::default().fg(Color::Red)),
        )
        .wrap(Wrap { trim: false });
        f.render_widget(Clear, popup);
        f.render_widget(para, popup);
    }

    if let UiMode::Message { text } = &app.mode {
        let popup = centered_rect(55, 14, f.area());
        let para  = Paragraph::new(text.as_str())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" ℹ  Info  (any key) ")
                    .style(Style::default().fg(Color::Cyan)),
            )
            .wrap(Wrap { trim: false });
        f.render_widget(Clear, popup);
        f.render_widget(para, popup);
    }
}

fn draw_content(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let content = app.current_content();
    let lines   = render_md(&content);

    let title = app
        .visible_notes()
        .get(app.selected)
        .map(|n| format!(" {} ", n.title))
        .unwrap_or_else(|| " Welcome ".to_string());

    let para = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: false })
        .scroll((app.scroll, 0));

    f.render_widget(para, area);
}

fn draw_status(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let text = match &app.mode {
        UiMode::Search      => " ESC/ENTER: done  ↑↓: navigate ",
        UiMode::NewNote { .. } => " Type title and press ENTER  ·  ESC: cancel ",
        UiMode::Confirm { .. } => " y: delete  ·  any key: cancel ",
        UiMode::Message { .. } => " Press any key to dismiss ",
        UiMode::Normal => " q:quit  n:new  e:edit  D:delete  /:search  j/k:navigate  ^D/^U:scroll ",
    };

    let bar = Paragraph::new(text)
        .style(Style::default().bg(Color::Rgb(30, 30, 50)).fg(Color::DarkGray));
    f.render_widget(bar, area);
}

// ---------------------------------------------------------------------------
// Markdown → ratatui Lines
// ---------------------------------------------------------------------------

fn render_md(content: &str) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut in_code = false;

    for raw in content.lines() {
        let s = raw.to_string();

        if s.starts_with("```") {
            in_code = !in_code;
            lines.push(Line::from(Span::styled(s, Style::default().fg(Color::DarkGray))));
            continue;
        }

        if in_code {
            lines.push(Line::from(Span::styled(
                format!("  {s}"),
                Style::default().fg(Color::Rgb(180, 220, 180)),
            )));
            continue;
        }

        let line = if let Some(h) = s.strip_prefix("# ") {
            Line::from(Span::styled(
                h.to_string(),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ))
        } else if let Some(h) = s.strip_prefix("## ") {
            Line::from(Span::styled(
                h.to_string(),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ))
        } else if let Some(h) = s.strip_prefix("### ") {
            Line::from(Span::styled(
                h.to_string(),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ))
        } else if let Some(t) = s.strip_prefix("- [ ] ") {
            Line::from(vec![
                Span::styled("  ☐ ", Style::default().fg(Color::Red)),
                Span::raw(t.to_string()),
            ])
        } else if let Some(t) = s.strip_prefix("- [x] ").or_else(|| s.strip_prefix("- [X] ")) {
            Line::from(vec![
                Span::styled("  ✓ ", Style::default().fg(Color::Green)),
                Span::styled(t.to_string(), Style::default().add_modifier(Modifier::CROSSED_OUT)),
            ])
        } else if let Some(t) = s.strip_prefix("- ").or_else(|| s.strip_prefix("* ")) {
            Line::from(vec![
                Span::styled("  • ", Style::default().fg(Color::Blue)),
                Span::raw(t.to_string()),
            ])
        } else if let Some(t) = s.strip_prefix("> ") {
            Line::from(vec![
                Span::styled("  │ ", Style::default().fg(Color::DarkGray)),
                Span::styled(t.to_string(), Style::default().fg(Color::Gray)),
            ])
        } else if s.trim_matches('-').trim().is_empty() && s.len() >= 3 {
            Line::from(Span::styled(
                "─".repeat(40),
                Style::default().fg(Color::DarkGray),
            ))
        } else {
            Line::from(Span::raw(s))
        };

        lines.push(line);
    }

    lines
}

// ---------------------------------------------------------------------------
// Geometry helper
// ---------------------------------------------------------------------------

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_v[1])[1]
}
