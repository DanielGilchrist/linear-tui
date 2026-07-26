use ratatui::layout::{Constraint, Layout, Rect};

pub fn split_horizontal(area: Rect, left_pct: u16) -> [Rect; 2] {
    Layout::horizontal([
        Constraint::Percentage(left_pct),
        Constraint::Percentage(100 - left_pct),
    ])
    .areas(area)
}

pub fn centred_rect(area: Rect, width_pct: u16, height_pct: u16) -> Rect {
    area.centered(
        Constraint::Percentage(width_pct),
        Constraint::Percentage(height_pct),
    )
}

pub fn centred_rect_fixed(area: Rect, width_pct: u16, height: u16) -> Rect {
    area.centered(
        Constraint::Percentage(width_pct),
        Constraint::Length(height),
    )
}

pub fn centred_box(area: Rect, width: u16, height: u16) -> Rect {
    area.centered(Constraint::Length(width), Constraint::Length(height))
}

pub fn split_footer(area: Rect, right_width: u16) -> [Rect; 2] {
    Layout::horizontal([Constraint::Min(0), Constraint::Length(right_width)]).areas(area)
}
