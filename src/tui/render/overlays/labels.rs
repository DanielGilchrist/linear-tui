use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, List, ListItem, Paragraph},
    Frame,
};

use super::super::theme::{self, Emphasis};
use crate::tui::layout;
use crate::tui::overlay::Labels;
use crate::tui::spinner::Spinner;

pub fn area(frame_area: Rect) -> Rect {
    layout::centred_rect_fixed(frame_area, 52, 16)
}

pub fn render(labels: &mut Labels, spinner: Spinner, frame: &mut Frame, area: Rect) {
    let block = Block::bordered()
        .title("Labels")
        .border_style(theme::accent());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [search_area, list_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(inner);

    frame.render_widget(Paragraph::new(search_line(labels)), search_area);

    if labels.results.is_loading() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("{} searching", spinner.glyph()),
                theme::DIM,
            ))),
            list_area,
        );
        return;
    }

    let items: Vec<ListItem> = labels
        .results()
        .iter()
        .map(|label| row_item(labels, label))
        .collect();

    let list = List::new(items)
        .highlight_style(Emphasis::Focused.highlight())
        .scroll_padding(1);

    frame.render_stateful_widget(list, list_area, &mut labels.state);
}

fn search_line(labels: &Labels) -> Line<'static> {
    let query = if labels.query.is_empty() {
        Span::styled("type to search", theme::DIM)
    } else {
        Span::styled(labels.query.clone(), theme::TEXT)
    };

    Line::from(vec![Span::styled("search: ", theme::DIM), query])
}

fn row_item(labels: &Labels, label: &crate::api::Label) -> ListItem<'static> {
    let marker = if labels.is_selected(&label.id) {
        Span::styled("✓ ", theme::accent())
    } else {
        Span::raw("  ")
    };

    ListItem::new(Line::from(vec![
        marker,
        Span::styled("● ", theme::label_chip(label.colour)),
        Span::styled(label.name.clone(), theme::TEXT),
    ]))
}
