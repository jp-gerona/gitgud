pub mod command_bar;
pub mod theme;
pub mod views;

use crate::app::App;
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

    views::status::draw(f, chunks[0], app);
    command_bar::draw(f, chunks[1], app);
    status_line(f, chunks[2], app);
}

fn status_line(f: &mut Frame, area: Rect, app: &App) {
    let line = if let Some(err) = &app.error {
        Line::from(vec![
            Span::styled(
                " error ",
                Style::default().bg(Color::Red).fg(Color::White),
            ),
            Span::raw(" "),
            Span::raw(err.as_str()),
        ])
    } else {
        Line::from(" [Tab] switch pane   [j/k] move   [r] refresh   [q] quit ")
    };
    f.render_widget(Paragraph::new(line), area);
}
