use ratatui::layout::{Constraint, Layout, Rect};

pub fn split_horizontal(area: Rect, left_pct: u16) -> [Rect; 2] {
    Layout::horizontal([
        Constraint::Percentage(left_pct),
        Constraint::Percentage(100 - left_pct),
    ])
    .areas(area)
}

pub fn centered_rect(area: Rect, width_pct: u16, height_pct: u16) -> Rect {
    let [_, row, _] = Layout::vertical([
        Constraint::Percentage((100 - height_pct) / 2),
        Constraint::Percentage(height_pct),
        Constraint::Percentage((100 - height_pct) / 2),
    ])
    .areas(area);

    let [_, center, _] = Layout::horizontal([
        Constraint::Percentage((100 - width_pct) / 2),
        Constraint::Percentage(width_pct),
        Constraint::Percentage((100 - width_pct) / 2),
    ])
    .areas(row);

    center
}

pub fn centered_rect_fixed(area: Rect, width_pct: u16, height: u16) -> Rect {
    let width = area.width * width_pct / 100;
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width,
        height: height.min(area.height),
    }
}

pub fn split_footer(area: Rect, right_width: u16) -> [Rect; 2] {
    Layout::horizontal([Constraint::Min(0), Constraint::Length(right_width)]).areas(area)
}
