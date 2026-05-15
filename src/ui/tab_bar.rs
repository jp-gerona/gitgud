//! Tab bar — 1-row strip at the top of the frame for tabbed views (Status,
//! Log). Each tab shows a numbered hint plus a live count (file count for
//! Status, commit count for Log). The active tab is bold + cyan.

use crate::app::{App, View};
use crate::ui::theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

/// Order in which tabs render and the numbers used to switch to each.
const TABS: &[(View, &str)] = &[(View::Status, "Status"), (View::Log, "Log")];

pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let mut spans: Vec<Span> = Vec::with_capacity(TABS.len() * 4 + 1);
    spans.push(Span::raw(" "));
    for (i, (v, label)) in TABS.iter().enumerate() {
        let active = app.view == *v;
        let count = count_for(*v, app);
        let chip = format!(" {}{} {} ", i + 1, ".", label_with_count(label, count));
        let style = if active {
            Style::default()
                .fg(theme::FOCUS_BORDER)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        spans.push(Span::styled(chip, style));
        spans.push(Span::raw(" "));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn label_with_count(label: &str, count: Option<usize>) -> String {
    match count {
        Some(n) => format!("{label} ({n})"),
        None => label.to_string(),
    }
}

fn count_for(view: View, app: &App) -> Option<usize> {
    match view {
        View::Status => {
            // Distinct files (a file in both staged and unstaged counts once).
            Some(app.status.entries.len())
        }
        View::Log => {
            if app.log.is_empty() {
                None
            } else {
                Some(app.log.len())
            }
        }
        View::CommitEditor => None,
    }
}
