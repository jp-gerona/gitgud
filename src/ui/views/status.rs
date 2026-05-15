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

    // When the Diff pane has focus, neither file pane is highlighted.
    let unstaged_focus = app.focused == Pane::Unstaged && !app.diff_focused;
    let staged_focus = app.focused == Pane::Staged && !app.diff_focused;

    draw_pane(
        f,
        left[0],
        "Unstaged",
        &unstaged,
        app.unstaged_selected,
        unstaged_focus,
        Pane::Unstaged,
    );
    draw_pane(
        f,
        left[1],
        "Staged",
        &staged,
        app.staged_selected,
        staged_focus,
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
    let border_color = if app.diff_focused {
        theme::FOCUS_BORDER
    } else {
        theme::DIM_BORDER
    };

    // Hunk-aware rendering when the diff parsed into hunks: a cyan `▌` gutter
    // marks the selected hunk, and the title shows `hunk h/n`.
    let (title, lines): (String, Vec<Line>) = match &app.diff_parsed {
        Some(fd) if !fd.hunks.is_empty() => {
            let n = fd.hunks.len();
            let sel = app.diff_hunk.min(n - 1);
            let mut out: Vec<Line> = fd
                .header_lines
                .iter()
                .map(|s| gutter_line(s.as_str(), false))
                .collect();
            for (i, h) in fd.hunks.iter().enumerate() {
                let active = i == sel;
                out.push(gutter_line(h.header.as_str(), active));
                for l in &h.lines {
                    out.push(gutter_line(l.as_str(), active));
                }
            }
            (format!(" Diff (hunk {}/{}) ", sel + 1, n), out)
        }
        _ if app.diff.is_empty() => (" Diff ".into(), vec![Line::from("(no diff)")]),
        _ => (" Diff ".into(), app.diff.lines().map(diff_line).collect()),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title);

    let p = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

/// A diff body line prefixed with a focus gutter: `▌ ` (cyan) for the active
/// hunk, two spaces otherwise. The content keeps its normal +/-/@@ coloring.
fn gutter_line(s: &str, active: bool) -> Line<'_> {
    let gutter = if active {
        Span::styled(
            "▌ ",
            Style::default()
                .fg(theme::FOCUS_BORDER)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw("  ")
    };
    let mut line = diff_line(s);
    line.spans.insert(0, gutter);
    line
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
