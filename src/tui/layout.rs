use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub fn split_horizontal(area: Rect, left_pct: u16) -> [Rect; 2] {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(left_pct),
            Constraint::Percentage(100 - left_pct),
        ])
        .split(area);

    [chunks[0], chunks[1]]
}

pub fn split_even(area: Rect, direction: Direction, count: u32) -> Vec<Rect> {
    let constraints: Vec<Constraint> = (0..count).map(|_| Constraint::Ratio(1, count)).collect();

    Layout::default()
        .direction(direction)
        .constraints(constraints)
        .split(area)
        .to_vec()
}

pub fn split_half(area: Rect, direction: Direction) -> [Rect; 2] {
    let chunks = Layout::default()
        .direction(direction)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    [chunks[0], chunks[1]]
}
