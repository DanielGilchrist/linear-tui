use pulldown_cmark::HeadingLevel;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

pub(super) fn heading_style(base: Style, level: HeadingLevel) -> Style {
    let style = base.add_modifier(Modifier::BOLD);
    match level {
        HeadingLevel::H1 => style.fg(Color::White),
        HeadingLevel::H2 => style.fg(Color::Cyan),
        _ => style.fg(Color::Rgb(0x81, 0xa1, 0xc1)),
    }
}

pub(super) fn code_style(base: Style) -> Style {
    base.fg(Color::Rgb(0xa3, 0xbe, 0x8c))
}

pub(super) fn mention_style(base: Style) -> Style {
    base.fg(Color::Blue)
}

pub(super) fn link_style(base: Style) -> Style {
    base.fg(Color::Blue).add_modifier(Modifier::UNDERLINED)
}

pub(super) fn quote_style(base: Style) -> Style {
    base.fg(Color::DarkGray)
}

pub(super) fn dim_style(base: Style) -> Style {
    base.fg(Color::DarkGray)
}

pub(super) fn marker_style(base: Style) -> Style {
    base.fg(Color::Rgb(0x81, 0xa1, 0xc1))
}

pub(super) fn task_marker(base: Style, checked: bool) -> Span<'static> {
    if checked {
        Span::styled("[x] ".to_string(), base.fg(Color::Green))
    } else {
        Span::styled("[ ] ".to_string(), base.fg(Color::DarkGray))
    }
}
