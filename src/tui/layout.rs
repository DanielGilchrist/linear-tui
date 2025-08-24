use ratatui::{
    layout::{Constraint, Direction, Layout as RatatuiLayout, Rect},
    Frame,
};

use super::components::Renderable;

pub struct TwoColumnLayout<L: Renderable, R: Renderable> {
    left: L,
    right: R,
    left_pct: u16,
    right_pct: u16,
}

impl<L: Renderable, R: Renderable> TwoColumnLayout<L, R> {
    pub fn new(left: L, right: R) -> Self {
        Self {
            left,
            right,
            left_pct: 30,
            right_pct: 70,
        }
    }
}

impl<L: Renderable, R: Renderable> Renderable for TwoColumnLayout<L, R> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = RatatuiLayout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(self.left_pct),
                Constraint::Percentage(self.right_pct),
            ])
            .split(area);

        self.left.render(frame, chunks[0]);
        self.right.render(frame, chunks[1]);
    }
}
