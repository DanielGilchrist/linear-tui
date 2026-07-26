use crossterm::event::KeyEvent;

use super::message::Message;
use crate::api::Timestamp;

pub enum Event {
    Input(KeyEvent),
    Resize,
    Message(Message),
    Tick(Timestamp),
    Ignored,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Redraw {
    Needed,
    Skipped,
}
