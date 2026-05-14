use crate::app::App;
use crate::ui::theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let line = match app.last_command() {
        Some(cmd) => Line::from(vec![
            Span::raw(" $ "),
            Span::styled(cmd, Style::default().fg(theme::COMMAND_BAR_FG)),
        ]),
        None => Line::from(" $ "),
    };
    f.render_widget(Paragraph::new(line), area);
}
