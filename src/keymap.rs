use crate::action::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Map a raw key event to an [`Action`]. There's only one view (status) for
/// now; once more views exist, this should take a `View` enum and dispatch on
/// `(view, key)`.
pub fn key_to_action(k: KeyEvent) -> Option<Action> {
    if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
        return Some(Action::Quit);
    }
    match k.code {
        KeyCode::Char('q') => Some(Action::Quit),
        KeyCode::Char('j') | KeyCode::Down => Some(Action::MoveSelection(1)),
        KeyCode::Char('k') | KeyCode::Up => Some(Action::MoveSelection(-1)),
        KeyCode::Tab => Some(Action::SwitchPane),
        KeyCode::BackTab => Some(Action::SwitchPaneBack),
        KeyCode::Char('l') | KeyCode::Right => Some(Action::EnterDiff),
        KeyCode::Char('h') | KeyCode::Left => Some(Action::LeaveDiff),
        KeyCode::Char('r') => Some(Action::Refresh),
        KeyCode::Char('s') => Some(Action::StageSelected),
        KeyCode::Char('u') => Some(Action::UnstageSelected),
        KeyCode::Char('X') => Some(Action::DiscardSelected),
        KeyCode::Char('c') => Some(Action::Commit),
        KeyCode::Esc => Some(Action::Dismiss),
        _ => None,
    }
}
