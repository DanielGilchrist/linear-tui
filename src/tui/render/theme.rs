#![allow(clippy::disallowed_types, clippy::disallowed_methods)]

use std::sync::OnceLock;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::api::{Priority, Rgb, StateType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColourMode {
    Ansi,
    Monochrome,
}

static MODE: OnceLock<ColourMode> = OnceLock::new();

pub fn init(mode: ColourMode) {
    let _ = MODE.set(mode);
}

fn coloured(colour: Color) -> Color {
    match MODE.get().copied().unwrap_or(ColourMode::Ansi) {
        ColourMode::Ansi => colour,
        ColourMode::Monochrome => Color::Reset,
    }
}

const ACCENT_COLOUR: Color = Color::Yellow;

pub const TEXT: Style = Style::new().fg(Color::Reset);
pub const TITLE: Style = Style::new().fg(Color::Reset).add_modifier(Modifier::BOLD);
pub const DIM: Style = Style::new().fg(Color::Reset).add_modifier(Modifier::DIM);
pub const REACTION: Style = DIM;

pub fn accent() -> Style {
    Style::new().fg(coloured(ACCENT_COLOUR))
}

pub fn person() -> Style {
    Style::new().fg(coloured(Color::Blue))
}

pub fn error() -> Style {
    Style::new().fg(coloured(Color::Red))
}

pub fn workspace() -> Style {
    Style::new().fg(coloured(Color::Cyan))
}

pub fn group_header() -> Style {
    Style::new()
        .fg(coloured(Color::Cyan))
        .add_modifier(Modifier::BOLD)
}

pub fn comment_author() -> Style {
    group_header()
}

pub fn menu_header() -> Style {
    Style::new()
        .fg(coloured(Color::Green))
        .add_modifier(Modifier::BOLD)
}

pub fn reaction_mine() -> Style {
    Style::new()
        .fg(coloured(ACCENT_COLOUR))
        .add_modifier(Modifier::BOLD)
}

pub fn heading() -> Style {
    Style::new()
        .fg(coloured(Color::Blue))
        .add_modifier(Modifier::BOLD)
}

pub fn marker() -> Style {
    Style::new().fg(coloured(Color::Blue))
}

pub fn code() -> Style {
    Style::new().fg(coloured(Color::Green))
}

pub fn done() -> Style {
    Style::new().fg(coloured(Color::Green))
}

pub fn link() -> Style {
    Style::new()
        .fg(coloured(Color::Blue))
        .add_modifier(Modifier::UNDERLINED)
}

pub fn find_label() -> Style {
    Style::new()
        .fg(coloured(ACCENT_COLOUR))
        .add_modifier(Modifier::REVERSED)
        .add_modifier(Modifier::BOLD)
}

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
            Emphasis::Focused => accent(),
            Emphasis::Blurred => DIM,
        }
    }

    pub fn title(self) -> Style {
        match self {
            Emphasis::Focused => accent().add_modifier(Modifier::BOLD),
            Emphasis::Blurred => DIM,
        }
    }

    pub fn blur_title<'a>(self, line: Line<'a>) -> Line<'a> {
        match self {
            Emphasis::Focused => line,
            Emphasis::Blurred => Line::from(
                line.spans
                    .into_iter()
                    .map(|span| {
                        let style = span.style.patch(DIM).remove_modifier(Modifier::BOLD);

                        Span::styled(span.content, style)
                    })
                    .collect::<Vec<_>>(),
            ),
        }
    }

    pub fn highlight(self) -> Style {
        match self {
            Emphasis::Focused => Style::new()
                .add_modifier(Modifier::REVERSED)
                .add_modifier(Modifier::BOLD)
                .remove_modifier(Modifier::DIM),
            Emphasis::Blurred => Style::new()
                .add_modifier(Modifier::BOLD)
                .remove_modifier(Modifier::DIM),
        }
    }
}

pub fn priority_style(priority: Priority) -> Style {
    let colour = match priority {
        Priority::Urgent => Color::Red,
        Priority::High => Color::LightRed,
        Priority::Medium => Color::Yellow,
        Priority::Low => Color::Blue,
        Priority::None => return DIM,
    };

    Style::new().fg(coloured(colour))
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
        StateType::Backlog => return DIM,
        StateType::Unstarted => return TEXT,
    };

    Style::new().fg(coloured(colour))
}

pub fn label_chip(colour: Rgb) -> Style {
    let luminance = relative_luminance(colour);
    let against_black = (luminance + 0.05) / 0.05;
    let against_white = 1.05 / (luminance + 0.05);

    let fg = if against_black >= against_white {
        Color::Rgb(0, 0, 0)
    } else {
        Color::Rgb(255, 255, 255)
    };

    Style::new()
        .fg(coloured(fg))
        .bg(coloured(Color::Rgb(colour.r, colour.g, colour.b)))
}

fn relative_luminance(colour: Rgb) -> f32 {
    let channel = |value: u8| {
        let value = f32::from(value) / 255.0;

        if value <= 0.040_45 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };

    0.2126 * channel(colour.r) + 0.7152 * channel(colour.g) + 0.0722 * channel(colour.b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chip_fg(hex: &str) -> Option<Color> {
        label_chip(Rgb::parse_hex(hex)).fg
    }

    #[test]
    fn a_chip_takes_whichever_foreground_contrasts_more() {
        let black = Some(Color::Rgb(0, 0, 0));
        let white = Some(Color::Rgb(255, 255, 255));

        assert_eq!(chip_fg("#eb5757"), black);
        assert_eq!(chip_fg("#9b51e0"), black);
        assert_eq!(chip_fg("#56ccf2"), black);
        assert_eq!(chip_fg("#f2c94c"), black);
        assert_eq!(chip_fg("#4f4f4f"), white);
        assert_eq!(chip_fg("#000000"), white);
    }
}
