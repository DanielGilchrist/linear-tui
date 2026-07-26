#![allow(clippy::disallowed_types)]

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

use crate::api::{Priority, Rgb, StateType};

pub const TEXT: Style = Style::new().fg(Color::White);
pub const TITLE: Style = Style::new().fg(Color::White).add_modifier(Modifier::BOLD);
pub const DIM: Style = Style::new().fg(Color::DarkGray);
pub const MUTED: Style = Style::new().fg(Color::Gray);
pub const ACCENT: Style = Style::new().fg(Color::Yellow);
pub const PERSON: Style = Style::new().fg(Color::Blue);
pub const ERROR: Style = Style::new().fg(Color::Red);
pub const WORKSPACE: Style = Style::new().fg(Color::Cyan);
pub const GROUP_HEADER: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
pub const COMMENT_AUTHOR: Style = GROUP_HEADER;
pub const MENU_HEADER: Style = Style::new().fg(Color::Green).add_modifier(Modifier::BOLD);
pub const FIND_LABEL: Style = Style::new()
    .fg(Color::Black)
    .bg(Color::Yellow)
    .add_modifier(Modifier::BOLD);

const LABEL_CHIP_FG: Color = Color::Black;

#[derive(Clone, Copy)]
pub enum Emphasis {
    Focused,
    Blurred,
}

impl Emphasis {
    pub fn of_focus(focused: bool) -> Self {
        if focused {
            Emphasis::Focused
        } else {
            Emphasis::Blurred
        }
    }

    pub fn border(self) -> Style {
        match self {
            Emphasis::Focused => Style::new().fg(Color::Yellow),
            Emphasis::Blurred => Style::new().fg(Color::Gray),
        }
    }

    pub fn highlight(self) -> Style {
        match self {
            Emphasis::Focused => Style::new().bg(Color::DarkGray).fg(Color::White),
            Emphasis::Blurred => Style::new().bg(Color::Rgb(45, 45, 48)),
        }
    }
}

pub fn priority_style(priority: Priority) -> Style {
    let colour = match priority {
        Priority::Urgent => Color::Red,
        Priority::High => Color::LightRed,
        Priority::Medium => Color::Yellow,
        Priority::Low => Color::Blue,
        Priority::None => Color::DarkGray,
    };

    Style::new().fg(colour)
}

pub fn priority_glyph(priority: Priority) -> Span<'static> {
    let glyph = match priority {
        Priority::Urgent => "!!!",
        Priority::High => "!! ",
        Priority::Medium => "!  ",
        Priority::Low => "-  ",
        Priority::None => "   ",
    };

    Span::styled(glyph, priority_style(priority))
}

pub fn state(state_type: StateType) -> Style {
    let colour = match state_type {
        StateType::Started => Color::Yellow,
        StateType::Completed => Color::Green,
        StateType::Cancelled => Color::Red,
        StateType::Triage => Color::Magenta,
        StateType::Backlog => Color::DarkGray,
        StateType::Unstarted => Color::Gray,
    };

    Style::new().fg(colour)
}

pub fn label_chip(colour: Rgb) -> Style {
    Style::new()
        .fg(LABEL_CHIP_FG)
        .bg(Color::Rgb(colour.r, colour.g, colour.b))
}
