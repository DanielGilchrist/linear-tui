use ratatui::{
    layout::Rect,
    text::{Line, Span, Text},
    widgets::{ListItem, ListState},
    Frame,
};

use super::super::theme::{self, Emphasis};
use super::super::widgets::{text_panel, StyledList};

pub fn render(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    items: &[String],
    state: &mut ListState,
    emphasis: Emphasis,
) {
    let list_items: Vec<ListItem> = items
        .iter()
        .map(|item| ListItem::new(Line::from(item.clone())))
        .collect();

    let selected = state.selected();
    let total = items.len();

    frame.render_widget(
        StyledList::new(title)
            .items(list_items)
            .emphasis(emphasis)
            .state(state)
            .position(selected, total),
        area,
    );
}

pub fn render_placeholder(frame: &mut Frame, area: Rect, title: &str, selected: &str) {
    let text = Text::from(vec![
        Line::from(Span::styled("Not implemented yet", theme::DIM)),
        Line::from(""),
        Line::from(selected.to_string()),
    ]);

    text_panel(frame, area, title, text, Emphasis::Focused);
}
