use anyhow::Result;
use crossterm::event::{self, Event, KeyEvent};
use std::time::Duration;

pub enum AppEvent {
    Key(KeyEvent),
    #[allow(dead_code)] // ratatui auto-redraws on the next tick; we'll wire this when we need it
    Resize(u16, u16),
}

pub fn poll(timeout: Duration) -> Result<Option<AppEvent>> {
    if !event::poll(timeout)? {
        return Ok(None);
    }
    match event::read()? {
        Event::Key(k) => Ok(Some(AppEvent::Key(k))),
        Event::Resize(w, h) => Ok(Some(AppEvent::Resize(w, h))),
        _ => Ok(None),
    }
}
