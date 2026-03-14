use std::io;
use std::path::Path;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Terminal;

use engram_core::model::{EngramData, FileChangeType, Manifest};
use engram_core::storage::{GitStorage, ListOptions};
use engram_query::SearchEngine;

pub struct App {
    manifests: Vec<Manifest>,
    detail_cache: Vec<Option<EngramData>>,
    list_state: ListState,
    search_query: String,
    search_mode: bool,
    detail_scroll: u16,
    quit: bool,
    repo_path: std::path::PathBuf,
}

impl App {
    fn new(manifests: Vec<Manifest>, repo_path: std::path::PathBuf) -> Self {
        let len = manifests.len();
        let mut list_state = ListState::default();
        if len > 0 {
            list_state.select(Some(0));
        }
        Self {
            detail_cache: vec![None; len],
            manifests,
            list_state,
            search_query: String::new(),
            search_mode: false,
            detail_scroll: 0,
            quit: false,
            repo_path,
        }
    }

    fn selected_index(&self) -> Option<usize> {
        self.list_state.selected()
    }

    fn move_up(&mut self) {
        if let Some(i) = self.list_state.selected() {
            if i > 0 {
                self.list_state.select(Some(i - 1));
                self.detail_scroll = 0;
            }
        }
    }

    fn move_down(&mut self) {
        if let Some(i) = self.list_state.selected() {
            if i + 1 < self.manifests.len() {
                self.list_state.select(Some(i + 1));
                self.detail_scroll = 0;
            }
        }
    }

    fn load_detail(&mut self, index: usize) {
        if self.detail_cache[index].is_some() {
            return;
        }
        if let Ok(storage) = GitStorage::open(&self.repo_path) {
            if let Ok(data) = storage.read(self.manifests[index].id.as_str()) {
                self.detail_cache[index] = Some(data);
            }
        }
    }

    fn search(&mut self) {
        if self.search_query.is_empty() {
            // Reload all
            if let Ok(storage) = GitStorage::open(&self.repo_path) {
                if let Ok(manifests) = storage.list(&ListOptions::default()) {
                    let len = manifests.len();
                    self.detail_cache = vec![None; len];
                    self.manifests = manifests;
                    if len > 0 {
                        self.list_state.select(Some(0));
                    } else {
                        self.list_state.select(None);
                    }
                }
            }
            return;
        }

        if let Ok(storage) = GitStorage::open(&self.repo_path) {
            if let Ok(engine) = SearchEngine::open(&storage) {
                if let Ok(results) = engine.search(&storage, &self.search_query, 50) {
                    let manifests: Vec<_> = results.into_iter().map(|r| r.manifest).collect();
                    let len = manifests.len();
                    self.detail_cache = vec![None; len];
                    self.manifests = manifests;
                    if len > 0 {
                        self.list_state.select(Some(0));
                    } else {
                        self.list_state.select(None);
                    }
                }
            }
        }
    }
}

pub fn run(repo_path: &Path) -> io::Result<()> {
    let storage = GitStorage::open(repo_path).map_err(|e| io::Error::other(e.to_string()))?;

    let manifests = storage
        .list(&ListOptions::default())
        .map_err(|e| io::Error::other(e.to_string()))?;

    let mut app = App::new(manifests, repo_path.to_path_buf());

    // Setup terminal
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;

    // Install panic hook to restore terminal state on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = io::stdout().execute(LeaveAlternateScreen);
        original_hook(info);
    }));

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    let result = run_loop(&mut terminal, &mut app);

    // Restore terminal (also handles normal exit)
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    // Restore the original panic hook
    let _ = std::panic::take_hook();

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    while !app.quit {
        // Eagerly load detail for selected item
        if let Some(idx) = app.selected_index() {
            app.load_detail(idx);
        }

        terminal.draw(|f| ui(f, app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                if app.search_mode {
                    match key.code {
                        KeyCode::Esc => {
                            app.search_mode = false;
                        }
                        KeyCode::Enter => {
                            app.search_mode = false;
                            app.search();
                        }
                        KeyCode::Backspace => {
                            app.search_query.pop();
                        }
                        KeyCode::Char(c) => {
                            app.search_query.push(c);
                        }
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Char('q') => app.quit = true,
                        KeyCode::Char('j') | KeyCode::Down => app.move_down(),
                        KeyCode::Char('k') | KeyCode::Up => app.move_up(),
                        KeyCode::Char('/') => {
                            app.search_mode = true;
                            app.search_query.clear();
                        }
                        KeyCode::Char('d') => {
                            app.detail_scroll = app.detail_scroll.saturating_add(3);
                        }
                        KeyCode::Char('u') => {
                            app.detail_scroll = app.detail_scroll.saturating_sub(3);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}

fn ui(f: &mut ratatui::Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(f.area());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(chunks[0]);

    render_list(f, app, main_chunks[0]);
    render_detail(f, app, main_chunks[1]);
    render_status(f, app, chunks[1]);
}

fn render_list(f: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .manifests
        .iter()
        .map(|m| {
            let short_id = &m.id.as_str()[..8.min(m.id.as_str().len())];
            let date = m.created_at.format("%m-%d %H:%M");
            let summary = m
                .summary
                .as_deref()
                .unwrap_or("(no summary)")
                .chars()
                .take(40)
                .collect::<String>();

            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(format!("{short_id} "), Style::default().fg(Color::Cyan)),
                    Span::styled(format!("{date}"), Style::default().fg(Color::DarkGray)),
                ]),
                Line::from(Span::raw(format!(" {summary}"))),
            ])
        })
        .collect();

    let title = if app.search_query.is_empty() {
        format!("Engrams ({})", app.manifests.len())
    } else {
        format!(
            "Results for '{}' ({})",
            app.search_query,
            app.manifests.len()
        )
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn render_detail(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let Some(idx) = app.selected_index() else {
        let empty = Paragraph::new("No engram selected.")
            .block(Block::default().borders(Borders::ALL).title("Detail"));
        f.render_widget(empty, area);
        return;
    };

    let m = &app.manifests[idx];
    let mut lines: Vec<Line> = Vec::new();

    // Summary
    if let Some(summary) = &m.summary {
        lines.push(Line::from(Span::styled(
            summary.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
    }

    // Metadata
    let model = m.agent.model.as_deref().unwrap_or("unknown");
    let cost = m
        .token_usage
        .effective_cost(m.agent.model.as_deref())
        .unwrap_or(0.0);

    lines.push(Line::from(vec![
        Span::styled("Agent: ", Style::default().fg(Color::DarkGray)),
        Span::raw(format!("{} ({})", m.agent.name, model)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Date:  ", Style::default().fg(Color::DarkGray)),
        Span::raw(m.created_at.format("%Y-%m-%d %H:%M:%S").to_string()),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Tokens:", Style::default().fg(Color::DarkGray)),
        Span::raw(format!(" {}", m.token_usage.total_tokens)),
        Span::styled(" | Cost: ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("${cost:.2}"), Style::default().fg(Color::Green)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("ID:    ", Style::default().fg(Color::DarkGray)),
        Span::styled(m.id.as_str().to_string(), Style::default().fg(Color::Cyan)),
    ]));

    // Full engram data if loaded
    if let Some(Some(data)) = app.detail_cache.get(idx) {
        lines.push(Line::from(""));

        // Intent
        lines.push(Line::from(Span::styled(
            "Intent",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(format!(
            "  Request: {}",
            data.intent.original_request
        )));
        if let Some(goal) = &data.intent.interpreted_goal {
            lines.push(Line::from(format!("  Goal: {goal}")));
        }
        if let Some(summary) = &data.intent.summary {
            lines.push(Line::from(format!("  Summary: {summary}")));
        }

        // File changes
        if !data.operations.file_changes.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "File Changes",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            for fc in &data.operations.file_changes {
                let (symbol, color) = match &fc.change_type {
                    FileChangeType::Created => ("+", Color::Green),
                    FileChangeType::Modified => ("~", Color::Yellow),
                    FileChangeType::Deleted => ("-", Color::Red),
                    FileChangeType::Renamed { .. } => ("R", Color::Blue),
                };
                lines.push(Line::from(Span::styled(
                    format!("  {symbol} {}", fc.path),
                    Style::default().fg(color),
                )));
            }
        }

        // Dead ends
        if !data.intent.dead_ends.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Dead Ends",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )));
            for de in &data.intent.dead_ends {
                lines.push(Line::from(format!("  {} - {}", de.approach, de.reason)));
            }
        }

        // Decisions
        if !data.intent.decisions.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Decisions",
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            )));
            for d in &data.intent.decisions {
                lines.push(Line::from(format!("  {} - {}", d.description, d.rationale)));
            }
        }
    }

    let detail = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Detail"))
        .wrap(Wrap { trim: false })
        .scroll((app.detail_scroll, 0));

    f.render_widget(detail, area);
}

fn render_status(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let content = if app.search_mode {
        Line::from(vec![
            Span::styled("Search: ", Style::default().fg(Color::Yellow)),
            Span::raw(&app.search_query),
            Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK)),
            Span::styled(
                "  (Enter to search, Esc to cancel)",
                Style::default().fg(Color::DarkGray),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(" /", Style::default().fg(Color::Cyan)),
            Span::styled(" search  ", Style::default().fg(Color::DarkGray)),
            Span::styled("j/k", Style::default().fg(Color::Cyan)),
            Span::styled(" navigate  ", Style::default().fg(Color::DarkGray)),
            Span::styled("d/u", Style::default().fg(Color::Cyan)),
            Span::styled(" scroll detail  ", Style::default().fg(Color::DarkGray)),
            Span::styled("q", Style::default().fg(Color::Cyan)),
            Span::styled(" quit", Style::default().fg(Color::DarkGray)),
        ])
    };

    let status = Paragraph::new(content).block(Block::default().borders(Borders::ALL));
    f.render_widget(status, area);
}
