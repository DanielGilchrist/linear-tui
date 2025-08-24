use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    Quit,
    NavigateTo,
    GoBack,
    NextItem,
    PreviousItem,
    None,
}

pub fn handle_key_event(key: KeyEvent) -> AppEvent {
    match (key.modifiers, key.code) {
        (_, KeyCode::Char('q')) | (KeyModifiers::CONTROL, KeyCode::Char('c')) => AppEvent::Quit,
        (_, KeyCode::Enter) => AppEvent::NavigateTo,
        (_, KeyCode::Esc) | (_, KeyCode::Backspace) | (_, KeyCode::BackTab) => AppEvent::GoBack,
        (_, KeyCode::Down) | (_, KeyCode::Char('j')) => AppEvent::NextItem,
        (_, KeyCode::Up) | (_, KeyCode::Char('k')) => AppEvent::PreviousItem,
        _ => AppEvent::None,
    }
}

pub fn poll_event() -> Result<Option<Event>> {
    if event::poll(Duration::from_millis(100))? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}
