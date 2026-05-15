//! Log view — two-pane (40/60) split. Left lists commits, right shows the
//! selected commit's `git show --stat` output. Mirrors the Status view's
//! visual language so navigation is consistent across tabs.

use crate::app::App;
use crate::git::LogEntry;
use crate::ui::theme;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);
    draw_list(f, cols[0], app);
    draw_detail(f, cols[1], app);
}

fn draw_list(f: &mut Frame, area: Rect, app: &App) {
    let entries: &[LogEntry] = &app.log.entries;
    let items: Vec<ListItem> = entries.iter().map(render_row).collect();

    let title = if entries.is_empty() {
        " Log (empty) ".to_string()
    } else {
        format!(" Log ({}) ", entries.len())
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::FOCUS_BORDER))
        .title(title);

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut state = ListState::default();
    if !entries.is_empty() {
        state.select(Some(app.log_selected.min(entries.len() - 1)));
    }
    f.render_stateful_widget(list, area, &mut state);
}

fn render_row(e: &LogEntry) -> ListItem<'_> {
    let dim = Style::default().fg(Color::DarkGray);
    let mut spans: Vec<Span> = vec![
        Span::raw(" "),
        Span::styled(
            e.short_sha.as_str(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(truncate(&e.author, 12), dim),
        Span::raw(" "),
        Span::styled(truncate(&e.when, 14), dim),
        Span::raw(" "),
        Span::raw(e.subject.as_str()),
    ];
    for r in &e.refs {
        spans.push(Span::raw(" "));
        spans.push(ref_chip(r));
    }
    ListItem::new(Line::from(spans))
}

fn ref_chip(r: &str) -> Span<'_> {
    let style = if r.starts_with("HEAD") {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else if r.starts_with("tag: ") {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else if r.contains('/') {
        // origin/main, upstream/foo, etc.
        Style::default().fg(Color::Black).bg(Color::Magenta)
    } else {
        Style::default().fg(Color::Black).bg(Color::Green)
    };
    Span::styled(format!(" {r} "), style)
}

fn truncate(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        s.to_string()
    } else {
        let cut: String = chars[..max.saturating_sub(1)].iter().collect();
        format!("{cut}…")
    }
}

fn draw_detail(f: &mut Frame, area: Rect, app: &App) {
    let title = match app.selected_commit() {
        Some(c) => format!(" {} ", c.short_sha),
        None => " (no commit) ".to_string(),
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::DIM_BORDER))
        .title(title);

    let lines: Vec<Line> = if app.log_detail.is_empty() {
        vec![Line::from("(empty)")]
    } else {
        app.log_detail.lines().map(detail_line).collect()
    };

    let p = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

/// Highlight `git show --stat` output — message lines as default, file rows
/// (the `| ` summary lines and the totals footer) lightly colored.
fn detail_line(s: &str) -> Line<'_> {
    let style = if s.starts_with("commit ") {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else if s.starts_with("Author:") || s.starts_with("Date:") {
        Style::default().fg(Color::DarkGray)
    } else if s.contains(" | ") {
        Style::default().fg(Color::Cyan)
    } else if s.trim_start().starts_with(|c: char| c.is_ascii_digit())
        && (s.contains("insertion") || s.contains("deletion") || s.contains("changed"))
    {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default()
    };
    Line::styled(s, style)
}
