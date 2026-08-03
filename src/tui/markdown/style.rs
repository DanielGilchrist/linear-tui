use pulldown_cmark::HeadingLevel;
use ratatui::style::Style;
use ratatui::text::Span;

use crate::tui::render::theme;

pub(super) fn heading_style(base: Style, level: HeadingLevel) -> Style {
    match level {
        HeadingLevel::H1 => base.patch(theme::TITLE),
        HeadingLevel::H2 => base.patch(theme::group_header()),
        _ => base.patch(theme::heading()),
    }
}

pub(super) fn code_style(base: Style) -> Style {
    base.patch(theme::code())
}

pub(super) fn mention_style(base: Style) -> Style {
    base.patch(theme::person())
}

pub(super) fn link_style(base: Style) -> Style {
    base.patch(theme::link())
}

pub(super) fn quote_style(base: Style) -> Style {
    base.patch(theme::DIM)
}

pub(super) fn dim_style(base: Style) -> Style {
    base.patch(theme::DIM)
}

pub(super) fn marker_style(base: Style) -> Style {
    base.patch(theme::marker())
}

pub(super) fn task_marker(base: Style, checked: bool) -> Span<'static> {
    let (glyph, style) = if checked {
        ("[x] ", base.patch(theme::done()))
    } else {
        ("[ ] ", base.patch(theme::DIM))
    };

    Span::styled(glyph.to_string(), style)
}
