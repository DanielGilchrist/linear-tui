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
static OVERRIDES: OnceLock<Overrides> = OnceLock::new();

pub fn init(mode: ColourMode) {
    let _ = MODE.set(mode);
}

pub fn init_overrides(overrides: Overrides) {
    let _ = OVERRIDES.set(overrides);
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Overrides {
    pub accent: Option<Color>,
    pub person: Option<Color>,
    pub error: Option<Color>,
    pub workspace: Option<Color>,
    pub group_header: Option<Color>,
    pub menu_header: Option<Color>,
    pub heading: Option<Color>,
    pub marker: Option<Color>,
    pub code: Option<Color>,
    pub done: Option<Color>,
    pub link: Option<Color>,
    pub dim: Option<Color>,
    pub selection_bg: Option<Color>,
    pub priority_urgent: Option<Color>,
    pub priority_high: Option<Color>,
    pub priority_medium: Option<Color>,
    pub priority_low: Option<Color>,
    pub state_started: Option<Color>,
    pub state_completed: Option<Color>,
    pub state_cancelled: Option<Color>,
    pub state_triage: Option<Color>,
}

#[derive(Debug, thiserror::Error)]
pub enum OverridesError {
    #[error("theme file is not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("theme key {key:?} has unrecognised colour {value:?} (expected #rrggbb or an ANSI colour name)")]
    Colour { key: &'static str, value: String },
}

impl Overrides {
    pub fn parse(json: &str) -> Result<Self, OverridesError> {
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct File {
            accent: Option<String>,
            person: Option<String>,
            error: Option<String>,
            workspace: Option<String>,
            group_header: Option<String>,
            menu_header: Option<String>,
            heading: Option<String>,
            marker: Option<String>,
            code: Option<String>,
            done: Option<String>,
            link: Option<String>,
            dim: Option<String>,
            selection_bg: Option<String>,
            priority_urgent: Option<String>,
            priority_high: Option<String>,
            priority_medium: Option<String>,
            priority_low: Option<String>,
            state_started: Option<String>,
            state_completed: Option<String>,
            state_cancelled: Option<String>,
            state_triage: Option<String>,
        }

        let file: File = serde_json::from_str(json)?;

        let colour = |key: &'static str, value: Option<String>| {
            value
                .map(|value| parse_colour(&value).ok_or(OverridesError::Colour { key, value }))
                .transpose()
        };

        Ok(Self {
            accent: colour("accent", file.accent)?,
            person: colour("person", file.person)?,
            error: colour("error", file.error)?,
            workspace: colour("workspace", file.workspace)?,
            group_header: colour("group_header", file.group_header)?,
            menu_header: colour("menu_header", file.menu_header)?,
            heading: colour("heading", file.heading)?,
            marker: colour("marker", file.marker)?,
            code: colour("code", file.code)?,
            done: colour("done", file.done)?,
            link: colour("link", file.link)?,
            dim: colour("dim", file.dim)?,
            selection_bg: colour("selection_bg", file.selection_bg)?,
            priority_urgent: colour("priority_urgent", file.priority_urgent)?,
            priority_high: colour("priority_high", file.priority_high)?,
            priority_medium: colour("priority_medium", file.priority_medium)?,
            priority_low: colour("priority_low", file.priority_low)?,
            state_started: colour("state_started", file.state_started)?,
            state_completed: colour("state_completed", file.state_completed)?,
            state_cancelled: colour("state_cancelled", file.state_cancelled)?,
            state_triage: colour("state_triage", file.state_triage)?,
        })
    }
}

fn parse_colour(value: &str) -> Option<Color> {
    let value = value.trim().to_ascii_lowercase();

    if let Some(hex) = value.strip_prefix('#') {
        if hex.len() != 6 {
            return None;
        }

        let channel = |range| u8::from_str_radix(hex.get(range)?, 16).ok();

        return Some(Color::Rgb(channel(0..2)?, channel(2..4)?, channel(4..6)?));
    }

    let colour = match value.as_str() {
        "reset" | "default" => Color::Reset,
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" => Color::Gray,
        "darkgray" | "darkgrey" => Color::DarkGray,
        "lightred" => Color::LightRed,
        "lightgreen" => Color::LightGreen,
        "lightyellow" => Color::LightYellow,
        "lightblue" => Color::LightBlue,
        "lightmagenta" => Color::LightMagenta,
        "lightcyan" => Color::LightCyan,
        "white" => Color::White,
        _ => return None,
    };

    Some(colour)
}

fn overrides() -> Option<&'static Overrides> {
    match MODE.get().copied().unwrap_or(ColourMode::Ansi) {
        ColourMode::Ansi => OVERRIDES.get(),
        ColourMode::Monochrome => None,
    }
}

fn slot(pick: impl Fn(&Overrides) -> Option<Color>, fallback: Color) -> Color {
    coloured(overrides().and_then(pick).unwrap_or(fallback))
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

pub fn dim() -> Style {
    match overrides().and_then(|overrides| overrides.dim) {
        Some(colour) => Style::new().fg(colour),
        None => Style::new().fg(Color::Reset).add_modifier(Modifier::DIM),
    }
}

pub fn reaction() -> Style {
    dim()
}

pub fn accent() -> Style {
    Style::new().fg(slot(|o| o.accent, ACCENT_COLOUR))
}

pub fn person() -> Style {
    Style::new().fg(slot(|o| o.person, Color::Blue))
}

pub fn error() -> Style {
    Style::new().fg(slot(|o| o.error, Color::Red))
}

pub fn workspace() -> Style {
    Style::new().fg(slot(|o| o.workspace, Color::Cyan))
}

pub fn group_header() -> Style {
    Style::new()
        .fg(slot(|o| o.group_header, Color::Cyan))
        .add_modifier(Modifier::BOLD)
}

pub fn comment_author() -> Style {
    group_header()
}

pub fn menu_header() -> Style {
    Style::new()
        .fg(slot(|o| o.menu_header, Color::Green))
        .add_modifier(Modifier::BOLD)
}

pub fn reaction_mine() -> Style {
    Style::new()
        .fg(slot(|o| o.accent, ACCENT_COLOUR))
        .add_modifier(Modifier::BOLD)
}

pub fn heading() -> Style {
    Style::new()
        .fg(slot(|o| o.heading, Color::Blue))
        .add_modifier(Modifier::BOLD)
}

pub fn marker() -> Style {
    Style::new().fg(slot(|o| o.marker, Color::Blue))
}

pub fn code() -> Style {
    Style::new().fg(slot(|o| o.code, Color::Green))
}

pub fn done() -> Style {
    Style::new().fg(slot(|o| o.done, Color::Green))
}

pub fn link() -> Style {
    Style::new()
        .fg(slot(|o| o.link, Color::Blue))
        .add_modifier(Modifier::UNDERLINED)
}

pub fn find_label() -> Style {
    Style::new()
        .fg(slot(|o| o.accent, ACCENT_COLOUR))
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
            Emphasis::Blurred => dim(),
        }
    }

    pub fn title(self) -> Style {
        match self {
            Emphasis::Focused => accent().add_modifier(Modifier::BOLD),
            Emphasis::Blurred => dim(),
        }
    }

    pub fn blur_title<'a>(self, line: Line<'a>) -> Line<'a> {
        match self {
            Emphasis::Focused => line,
            Emphasis::Blurred => Line::from(
                line.spans
                    .into_iter()
                    .map(|span| {
                        let style = span.style.patch(dim()).remove_modifier(Modifier::BOLD);

                        Span::styled(span.content, style)
                    })
                    .collect::<Vec<_>>(),
            ),
        }
    }

    pub fn highlight(self) -> Style {
        let selection_bg = overrides().and_then(|overrides| overrides.selection_bg);

        match (self, selection_bg) {
            (Emphasis::Focused, Some(colour)) => Style::new()
                .bg(colour)
                .add_modifier(Modifier::BOLD)
                .remove_modifier(Modifier::DIM),
            (Emphasis::Focused, None) => Style::new()
                .add_modifier(Modifier::REVERSED)
                .add_modifier(Modifier::BOLD)
                .remove_modifier(Modifier::DIM),
            (Emphasis::Blurred, _) => Style::new()
                .add_modifier(Modifier::BOLD)
                .remove_modifier(Modifier::DIM),
        }
    }
}

pub fn priority_style(priority: Priority) -> Style {
    let colour = match priority {
        Priority::Urgent => slot(|o| o.priority_urgent, Color::Red),
        Priority::High => slot(|o| o.priority_high, Color::LightRed),
        Priority::Medium => slot(|o| o.priority_medium, Color::Yellow),
        Priority::Low => slot(|o| o.priority_low, Color::Blue),
        Priority::None => return dim(),
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
        StateType::Started => slot(|o| o.state_started, Color::Yellow),
        StateType::Completed => slot(|o| o.state_completed, Color::Green),
        StateType::Cancelled => slot(|o| o.state_cancelled, Color::Red),
        StateType::Triage => slot(|o| o.state_triage, Color::Magenta),
        StateType::Backlog => return dim(),
        StateType::Unstarted => return TEXT,
    };

    Style::new().fg(colour)
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
    fn colours_parse_from_hex_and_ansi_names() {
        assert_eq!(parse_colour("#ff9e64"), Some(Color::Rgb(255, 158, 100)));
        assert_eq!(parse_colour("#FF9E64"), Some(Color::Rgb(255, 158, 100)));
        assert_eq!(parse_colour("yellow"), Some(Color::Yellow));
        assert_eq!(parse_colour("lightred"), Some(Color::LightRed));
        assert_eq!(parse_colour("grey"), Some(Color::Gray));
        assert_eq!(parse_colour("reset"), Some(Color::Reset));
        assert_eq!(parse_colour("#ff9e"), None);
        assert_eq!(parse_colour("#gggggg"), None);
        assert_eq!(parse_colour("mauve"), None);
    }

    #[test]
    fn overrides_parse_from_json() {
        let overrides =
            Overrides::parse(r##"{"accent": "#ff9e64", "selection_bg": "darkgrey"}"##).unwrap();

        assert_eq!(overrides.accent, Some(Color::Rgb(255, 158, 100)));
        assert_eq!(overrides.selection_bg, Some(Color::DarkGray));
        assert_eq!(overrides.dim, None);
    }

    #[test]
    fn overrides_reject_unknown_keys_and_bad_colours() {
        assert!(matches!(
            Overrides::parse(r##"{"acent": "#ff9e64"}"##),
            Err(OverridesError::Json(_))
        ));
        assert!(matches!(
            Overrides::parse(r##"{"accent": "mauve"}"##),
            Err(OverridesError::Colour { key: "accent", .. })
        ));
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
