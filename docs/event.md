# `event`

Source: [`src/event.rs`](../src/event.rs)

## Purpose

A thin adapter over [`crossterm::event`](https://docs.rs/crossterm/0.28/crossterm/event/index.html) that polls for terminal input with a timeout and yields a small `AppEvent` enum.

## API

```rust
pub enum AppEvent {
    Key(KeyEvent),
    Resize(u16, u16),   // currently unused; ratatui redraws on next tick
}

pub fn poll(timeout: Duration) -> Result<Option<AppEvent>>;
```

`poll` returns:

- `Ok(Some(event))` — a relevant terminal event arrived.
- `Ok(None)` — the timeout elapsed with nothing to do (or an event we ignore — mouse, focus, paste).
- `Err(_)` — a terminal-I/O failure.

## Why bounded polling

The main loop calls `event::poll(Duration::from_millis(200))` so the UI thread:

- Wakes promptly for keypresses.
- Doesn't busy-loop when idle.
- Stays available for a future timer-driven feature (auto-refresh, animation) without restructuring.

200 ms is arbitrary; nothing time-based depends on this tick rate today.

## Filtering

We ignore:

- `Event::Mouse` — gitgud is keyboard-only.
- `Event::FocusGained` / `FocusLost` — no use case yet.
- `Event::Paste` — bracketed paste isn't enabled in the terminal setup, so this shouldn't fire.

`Resize` is captured but currently ignored downstream — Ratatui repaints from scratch every `terminal.draw` call, so the next tick already renders at the new size.

## Related

- [`app`](app.md) — drives `event::poll` from the main loop
- [`keymap`](keymap-action.md) — receives the `KeyEvent` payload
