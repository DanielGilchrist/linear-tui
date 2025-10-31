use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

pub enum AppEvent {
    Quit,
    NavigateTo,
    GoBack,
    Preview,
    NextItem,
    PreviousItem,
    NextPanel,
    PreviousPanel,
    JumpToPanel(usize),
    None,
}

pub fn handle_key_event(key: KeyEvent) -> AppEvent {
    match (key.modifiers, key.code) {
        (_, KeyCode::Char('q')) | (KeyModifiers::CONTROL, KeyCode::Char('c')) => AppEvent::Quit,
        (_, KeyCode::Enter) => AppEvent::NavigateTo,
        (_, KeyCode::Esc) | (_, KeyCode::Backspace) => AppEvent::GoBack,
        (_, KeyCode::Char('p')) => AppEvent::Preview,
        (_, KeyCode::Down) | (_, KeyCode::Char('j')) => AppEvent::NextItem,
        (_, KeyCode::Up) | (_, KeyCode::Char('k')) => AppEvent::PreviousItem,
        (_, KeyCode::Tab) => AppEvent::NextPanel,
        (_, KeyCode::BackTab) => AppEvent::PreviousPanel,
        (_, KeyCode::Char(c @ '1'..='9')) => {
            AppEvent::JumpToPanel(c.to_digit(10).unwrap() as usize - 1)
        }
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
