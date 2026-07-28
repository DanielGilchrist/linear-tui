use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::ListItem,
    Frame,
};

use super::super::theme::{self, Emphasis};
use super::super::widgets::StyledList;
use crate::tui::layout;
use crate::tui::overlay::{Picker, PickerItem};
use crate::tui::spinner::Spinner;

pub fn area(frame_area: Rect) -> Rect {
    layout::centred_rect(frame_area, 44, 55)
}

pub fn render(picker: &mut Picker, spinner: Spinner, frame: &mut Frame, area: Rect) {
    let rows: Vec<ListItem> = picker.items.iter().map(picker_row).collect();

    let placeholder = if picker.loading {
        format!("{spinner}  Searching…")
    } else if picker.searching().is_some() {
        "No matches  ·  / search again".to_string()
    } else if picker.searchable() {
        "/ to search".to_string()
    } else {
        "Nothing to choose".to_string()
    };

    let title = match picker.searching() {
        Some(query) => format!("{}  {}  ·  {query}", picker.verb(), picker.target_label),
        None => format!("{}  {}", picker.verb(), picker.target_label),
    };
    let selected = picker.state.selected();
    let total = picker.items.len();

    let list = StyledList::new(&title)
        .items(rows)
        .emphasis(Emphasis::Focused)
        .state(&mut picker.state)
        .position(selected, total)
        .placeholder(Line::from(placeholder));

    frame.render_widget(list, area);
}

fn picker_row(item: &PickerItem) -> ListItem<'static> {
    let mut spans = vec![Span::styled(item.label.clone(), theme::TEXT)];

    if let Some(hint) = item.hint() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(hint.to_string(), theme::DIM));
    }

    ListItem::new(Line::from(spans))
}
