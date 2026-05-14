use crate::app::{App, Pane};
use crate::git::{FileEntry, FileStatus};
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

    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(cols[0]);

    let unstaged: Vec<&FileEntry> = app.status.unstaged().collect();
    let staged: Vec<&FileEntry> = app.status.staged().collect();

    draw_pane(
        f,
        left[0],
        "Unstaged",
        &unstaged,
        app.unstaged_selected,
        app.focused == Pane::Unstaged,
        Pane::Unstaged,
    );
    draw_pane(
        f,
        left[1],
        "Staged",
        &staged,
        app.staged_selected,
        app.focused == Pane::Staged,
        Pane::Staged,
    );

    draw_diff(f, cols[1], app);
}

fn draw_pane(
    f: &mut Frame,
    area: Rect,
    title: &str,
    entries: &[&FileEntry],
    selected: usize,
    focused: bool,
    pane: Pane,
) {
    let border_color = if focused {
        theme::FOCUS_BORDER
    } else {
        theme::DIM_BORDER
    };

    let items: Vec<ListItem> = entries
        .iter()
        .map(|e| {
            let (sym, color) = symbol_and_color(e, pane);
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {sym} "),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::raw(e.path.as_str()),
            ]))
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(format!(" {} ({}) ", title, entries.len()));

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut state = ListState::default();
    if focused && !entries.is_empty() {
        state.select(Some(selected.min(entries.len() - 1)));
    }
    f.render_stateful_widget(list, area, &mut state);
}

fn symbol_and_color(e: &FileEntry, pane: Pane) -> (char, Color) {
    let status = match pane {
        Pane::Staged => &e.index,
        Pane::Unstaged => &e.worktree,
    };
    let color = match status {
        FileStatus::Added | FileStatus::Renamed | FileStatus::Copied => theme::STAGED,
        FileStatus::Modified | FileStatus::TypeChange => theme::UNSTAGED,
        FileStatus::Deleted | FileStatus::Untracked | FileStatus::Unmerged => theme::UNTRACKED,
        _ => Color::Gray,
    };
    (status.symbol(), color)
}

fn draw_diff(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::DIM_BORDER))
        .title(" Diff ");

    let lines: Vec<Line> = if app.diff.is_empty() {
        vec![Line::from("(no diff)")]
    } else {
        app.diff.lines().map(diff_line).collect()
    };

    let p = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

fn diff_line(s: &str) -> Line<'_> {
    let style = if s.starts_with("+++") || s.starts_with("---") {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else if s.starts_with('+') {
        Style::default().fg(Color::Green)
    } else if s.starts_with('-') {
        Style::default().fg(Color::Red)
    } else if s.starts_with("@@") {
        Style::default().fg(Color::Magenta)
    } else if s.starts_with("diff ") {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    Line::styled(s, style)
}
