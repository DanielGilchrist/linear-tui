use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::ListItem,
    Frame,
};

use super::super::theme::{self, Emphasis};
use super::super::widgets::StyledList;
use crate::tui::layout;
use crate::tui::overlay::{Menu, MenuRow};

pub fn area(frame_area: Rect) -> Rect {
    layout::centred_rect(frame_area, 44, 70)
}

pub fn render(menu: &mut Menu, frame: &mut Frame, area: Rect) {
    let items = menu_items(menu);

    let keybindings = StyledList::new("Keybindings")
        .items(items)
        .emphasis(Emphasis::Focused)
        .state(&mut menu.state);

    frame.render_widget(keybindings, area);
}

fn menu_items(menu: &Menu) -> Vec<ListItem<'static>> {
    let key_width = menu
        .rows
        .iter()
        .filter_map(|row| match row {
            MenuRow::Item { keys, .. } => Some(keys.chars().count()),
            MenuRow::Header(_) => None,
        })
        .max()
        .unwrap_or(0);

    menu.rows
        .iter()
        .map(|row| match row {
            MenuRow::Header(title) => {
                ListItem::new(Line::from(Span::styled(*title, theme::menu_header())))
            }
            MenuRow::Item { keys, label, .. } => ListItem::new(Line::from(vec![
                Span::styled(format!("{keys:>key_width$}"), theme::accent()),
                Span::raw("  "),
                Span::styled(*label, theme::TEXT),
            ])),
        })
        .collect()
}
