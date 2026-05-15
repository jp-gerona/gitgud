pub mod command_bar;
pub mod prompt_bar;
pub mod tab_bar;
pub mod theme;
pub mod views;

use crate::app::{App, View};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

pub fn draw(f: &mut Frame, app: &App) {
    let tabbed = app.view.is_tabbed();
    let prompt_active = tabbed && app.prompt.is_some();

    // Top region: optional tab bar (1 row) on tabbed views.
    // Bottom region: command bar (1) + optional prompt row (1) + status line (1).
    let mut constraints: Vec<Constraint> = Vec::new();
    if tabbed {
        constraints.push(Constraint::Length(1));
    }
    constraints.push(Constraint::Min(0));
    constraints.push(Constraint::Length(1));
    if prompt_active {
        constraints.push(Constraint::Length(1));
    }
    constraints.push(Constraint::Length(1));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(f.area());

    let mut i = 0;
    if tabbed {
        tab_bar::draw(f, chunks[i], app);
        i += 1;
    }
    let content_idx = i;
    match app.view {
        View::Status => views::status::draw(f, chunks[content_idx], app),
        View::Log => views::log::draw(f, chunks[content_idx], app),
        View::CommitEditor => views::commit::draw(f, chunks[content_idx], app),
    }
    i += 1;
    command_bar::draw(f, chunks[i], app);
    i += 1;
    if prompt_active {
        prompt_bar::draw(f, chunks[i], app);
        i += 1;
    }
    status_line(f, chunks[i], app);
}

fn status_line(f: &mut Frame, area: Rect, app: &App) {
    let line = match app.view {
        View::Status => status_view_hints(app),
        View::Log => log_view_hints(app),
        View::CommitEditor => Line::from(" [Ctrl+C] quit gitgud "),
    };
    f.render_widget(Paragraph::new(line), area);
}

fn status_view_hints(app: &App) -> Line<'_> {
    if let Some(c) = &app.confirm {
        return confirm_hints(&c.prompt);
    }
    if app.prompt.is_some() {
        return prompt_hints();
    }
    if let Some(err) = &app.error {
        return error_hints(err);
    }
    if app.diff_focused {
        return Line::from(
            " [Tab] pane  [j/k] hunk  [s] stage hunk  [u] unstage hunk  [X] discard hunk  [Esc] back  [/] cmd  [q] quit ",
        );
    }
    Line::from(
        " [1/2] tab  [Tab] pane  [j/k] move  [s] stage  [u] unstage  [X] discard  [c] commit  [/] cmd  [r] refresh  [q] quit ",
    )
}

fn log_view_hints(app: &App) -> Line<'_> {
    if let Some(c) = &app.confirm {
        return confirm_hints(&c.prompt);
    }
    if app.prompt.is_some() {
        return prompt_hints();
    }
    if let Some(err) = &app.error {
        return error_hints(err);
    }
    Line::from(" [1/2] tab  [j/k] move  [g/G] top/bottom  [/] cmd  [r] refresh  [q] quit ")
}

fn prompt_hints() -> Line<'static> {
    Line::from(vec![
        Span::raw(" "),
        Span::styled("[Esc] back", Style::default().fg(Color::DarkGray)),
        Span::raw("   "),
        Span::styled("[↑/↓] history", Style::default().fg(Color::DarkGray)),
        Span::raw("   "),
        Span::styled("[Enter] run", Style::default().fg(Color::DarkGray)),
        Span::raw(" "),
    ])
}

fn confirm_hints(prompt: &str) -> Line<'_> {
    Line::from(vec![
        Span::styled(
            " confirm ",
            Style::default().bg(Color::Yellow).fg(Color::Black),
        ),
        Span::raw(" "),
        Span::raw(prompt),
        Span::raw("  "),
        Span::styled("[y]", Style::default().fg(Color::Red)),
        Span::raw(" yes / "),
        Span::styled("[N]", Style::default().fg(Color::Green)),
        Span::raw(" no (any other key cancels) "),
    ])
}

fn error_hints(err: &str) -> Line<'_> {
    Line::from(vec![
        Span::styled(" error ", Style::default().bg(Color::Red).fg(Color::White)),
        Span::raw(" "),
        Span::raw(err),
        Span::raw("  "),
        Span::styled("[Esc] dismiss", Style::default().fg(Color::DarkGray)),
    ])
}
