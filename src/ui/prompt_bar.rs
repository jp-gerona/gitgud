//! One-row prompt strip rendered when the user is in slash-Command mode.
//! Shows `/` followed by the buffer and positions the terminal cursor.

use crate::app::App;
use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let Some(p) = app.prompt.as_ref() else {
        return;
    };

    let line = Line::from(vec![
        Span::styled(
            "/",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(p.buffer.as_str()),
    ]);
    f.render_widget(Paragraph::new(line), area);

    // Cursor sits one char past the `/` plus the char-cursor offset.
    let cx = area.x + 1 + p.cursor as u16;
    if cx < area.x + area.width {
        f.set_cursor_position(Position::new(cx, area.y));
    }
}
