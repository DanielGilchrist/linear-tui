use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::ListItem,
    Frame,
};

use super::super::theme::{self, Emphasis};
use super::super::widgets::StyledList;
use crate::tui::action::{Action, Keymap};
use crate::tui::layout;

pub fn area(frame_area: Rect, keymap: &Keymap<Action>) -> Rect {
    layout::centred_rect_fixed(frame_area, 30, describe_items(keymap).len() as u16 + 2)
}

pub fn render(keymap: &Keymap<Action>, title: &str, frame: &mut Frame, area: Rect) {
    let bindings = StyledList::new(title)
        .items(describe_items(keymap))
        .emphasis(Emphasis::Focused);

    frame.render_widget(bindings, area);
}

fn describe_items(keymap: &Keymap<Action>) -> Vec<ListItem<'static>> {
    keymap
        .bindings
        .iter()
        .filter_map(|binding| {
            keymap.describe(binding.action).map(|(keys, label)| {
                ListItem::new(Line::from(vec![
                    Span::styled(keys, theme::accent()),
                    Span::raw("  "),
                    Span::styled(label, theme::TEXT),
                ]))
            })
        })
        .collect()
}
