pub mod command_bar;
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
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(f.area());

    match app.view {
        View::Status => views::status::draw(f, chunks[0], app),
        View::CommitEditor => views::commit::draw(f, chunks[0], app),
    }
    command_bar::draw(f, chunks[1], app);
    status_line(f, chunks[2], app);
}

fn status_line(f: &mut Frame, area: Rect, app: &App) {
    let line = match app.view {
        View::Status => {
            if let Some(err) = &app.error {
                Line::from(vec![
                    Span::styled(
                        " error ",
                        Style::default().bg(Color::Red).fg(Color::White),
                    ),
                    Span::raw(" "),
                    Span::raw(err.as_str()),
                    Span::raw("  "),
                    Span::styled("[Esc] dismiss", Style::default().fg(Color::DarkGray)),
                ])
            } else {
                Line::from(
                    " [Tab] pane  [j/k] move  [s] stage  [u] unstage  [c] commit  [r] refresh  [q] quit ",
                )
            }
        }
        View::CommitEditor => Line::from(" [Ctrl+C] quit gitgud "),
    };
    f.render_widget(Paragraph::new(line), area);
}
