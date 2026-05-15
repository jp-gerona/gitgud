use crate::app::App;
use crate::commit_editor::{CommitEditor, EditorMode};
use crate::ui::theme;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

const SUBJECT_SOFT_LIMIT: usize = 50;
const SUBJECT_HARD_LIMIT: usize = 72;
const BODY_LIMIT: usize = 72;

pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(6),    // editor (bordered)
            Constraint::Length(1), // vim status line (mode / cmd input / status msg)
            Constraint::Length(6), // mode-aware hints (bordered)
        ])
        .split(area);

    let editor_inner = draw_editor(f, chunks[0], &app.commit_editor);
    draw_vim_status_line(f, chunks[1], &app.commit_editor);
    draw_hints(f, chunks[2], &app.commit_editor.mode);
    place_cursor(f, &app.commit_editor, editor_inner, chunks[1]);
}

fn draw_editor(f: &mut Frame, area: Rect, ed: &CommitEditor) -> Rect {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::FOCUS_BORDER))
        .title(Line::from(vec![
            Span::raw(" Commit message  "),
            Span::styled(
                "(subject ≤50 chars, body ≤72)",
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw(" "),
        ]));
    let inner = block.inner(area);

    let lines: Vec<Line> = ed
        .lines
        .iter()
        .enumerate()
        .map(|(i, line)| render_line(i, line))
        .collect();
    f.render_widget(Paragraph::new(lines).block(block), area);
    inner
}

fn render_line(row: usize, line: &str) -> Line<'static> {
    let (soft, hard) = if row == 0 {
        (SUBJECT_SOFT_LIMIT, SUBJECT_HARD_LIMIT)
    } else {
        (BODY_LIMIT, BODY_LIMIT)
    };
    let chars: Vec<char> = line.chars().collect();
    let total = chars.len();
    let normal_end = soft.min(total);
    let warn_end = hard.min(total);

    let mut spans: Vec<Span<'static>> = Vec::new();
    if normal_end > 0 {
        spans.push(Span::raw(chars[..normal_end].iter().collect::<String>()));
    }
    if warn_end > normal_end {
        spans.push(Span::styled(
            chars[normal_end..warn_end].iter().collect::<String>(),
            Style::default().fg(Color::Yellow),
        ));
    }
    if total > warn_end {
        spans.push(Span::styled(
            chars[warn_end..].iter().collect::<String>(),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));
    }
    Line::from(spans)
}

fn draw_vim_status_line(f: &mut Frame, area: Rect, ed: &CommitEditor) {
    let line = if let Some(msg) = &ed.status_message {
        Line::from(vec![Span::styled(
            format!(" {msg} "),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )])
    } else {
        match &ed.mode {
            EditorMode::Normal => Line::from(Span::styled(
                " -- NORMAL -- ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            EditorMode::Insert => Line::from(Span::styled(
                " -- INSERT -- ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )),
            EditorMode::Command(s) => Line::from(vec![
                Span::raw(" "),
                Span::styled(":", Style::default().fg(Color::Yellow)),
                Span::raw(s.clone()),
            ]),
        }
    };
    f.render_widget(Paragraph::new(line), area);
}

fn draw_hints(f: &mut Frame, area: Rect, mode: &EditorMode) {
    let bold = Style::default().add_modifier(Modifier::BOLD);
    let (title, lines): (&str, Vec<Line>) = match mode {
        EditorMode::Normal => (
            " NORMAL mode ",
            vec![
                Line::from(vec![
                    Span::styled("Move:    ", bold),
                    Span::raw("h j k l  ·  0 $ line ends  ·  w b word  ·  gg G top/bottom"),
                ]),
                Line::from(vec![
                    Span::styled("Insert:  ", bold),
                    Span::raw(
                        "i at cursor  ·  a after  ·  I line start  ·  A line end  ·  o below  ·  O above",
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Delete:  ", bold),
                    Span::raw("x char  ·  dd line  ·  dw word  ·  D to end of line"),
                ]),
                Line::from(vec![
                    Span::styled("Command: ", bold),
                    Span::raw(":  then  "),
                    Span::styled("w  wq  x", Style::default().fg(Color::Green)),
                    Span::raw(" commit  ·  "),
                    Span::styled("q!  q", Style::default().fg(Color::Red)),
                    Span::raw(" cancel"),
                ]),
            ],
        ),
        EditorMode::Insert => (
            " INSERT mode ",
            vec![
                Line::from(
                    "Type to insert text.  Enter for newline.  Arrows / Home / End / Backspace / Delete to edit.",
                ),
                Line::from(vec![
                    Span::styled("Esc ", bold),
                    Span::raw("returns to NORMAL mode."),
                ]),
            ],
        ),
        EditorMode::Command(_) => (
            " COMMAND mode ",
            vec![
                Line::from(vec![
                    Span::styled(":w  :wq  :x   ", bold),
                    Span::raw("commit the message (write & quit)"),
                ]),
                Line::from(vec![
                    Span::styled(":q!           ", bold),
                    Span::raw("cancel and discard the message"),
                ]),
                Line::from(vec![
                    Span::styled(":q            ", bold),
                    Span::raw("cancel (only if buffer is empty)"),
                ]),
                Line::from(vec![
                    Span::styled("Enter ", bold),
                    Span::raw("executes  ·  "),
                    Span::styled("Esc ", bold),
                    Span::raw("returns to NORMAL"),
                ]),
            ],
        ),
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::DIM_BORDER))
        .title(title);
    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn place_cursor(f: &mut Frame, ed: &CommitEditor, editor_inner: Rect, status_area: Rect) {
    match &ed.mode {
        EditorMode::Command(s) => {
            // Cursor at the end of ":<input>" in the vim status line.
            // " :" leading is one space + colon = 2 cells.
            let cx = status_area.x + 2 + s.chars().count() as u16;
            if cx < status_area.x + status_area.width {
                f.set_cursor_position(Position::new(cx, status_area.y));
            }
        }
        _ => {
            let cx = editor_inner.x + ed.col as u16;
            let cy = editor_inner.y + ed.row as u16;
            if cx < editor_inner.x + editor_inner.width && cy < editor_inner.y + editor_inner.height
            {
                f.set_cursor_position(Position::new(cx, cy));
            }
        }
    }
}
