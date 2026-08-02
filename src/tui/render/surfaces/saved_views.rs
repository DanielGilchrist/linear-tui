use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Text},
    widgets::{Block, List, ListItem, Paragraph},
    Frame,
};

use super::super::format::id_column_width;
use super::super::theme::Emphasis;
use super::super::widgets::{
    breakdown_line, placeholder, view_items, view_title, PlaceholderText, StyledList,
};
use super::{feed_count, feed_placeholder, feed_truncated};
use crate::api::{Timestamp, ViewId};
use crate::tui::display::{self, GroupBy, SortBy};
use crate::tui::feed::{Feed, FeedKey, FeedStore};
use crate::tui::saved_views::SavedViewsPanel;
use crate::tui::spinner::Spinner;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    panel: &mut SavedViewsPanel,
    emphasis: Emphasis,
    spinner: Spinner,
) {
    let selected = panel.state.selected();
    let total = panel.list().len();
    let status = panel.views.status();

    let items: Vec<ListItem> = panel
        .list()
        .iter()
        .map(|view| ListItem::new(Line::from(view.name.clone())))
        .collect();

    let list = StyledList::new("Saved Views")
        .items(items)
        .emphasis(emphasis)
        .state(&mut panel.state)
        .position(selected, total);

    let list = match total {
        0 => {
            let text = PlaceholderText {
                empty: "No saved views",
                loading: super::LOADING_TEXT,
                failed: super::LOAD_FAILED_TEXT,
            };

            list.placeholder(placeholder(Some(status), text, spinner))
        }
        _ => list,
    };

    frame.render_widget(list, area);
}

pub fn render_preview(
    frame: &mut Frame,
    area: Rect,
    feeds: &FeedStore,
    id: &ViewId,
    name: &str,
    spinner: Spinner,
    now: Timestamp,
) {
    let feed = feeds.get(&FeedKey::View(id.clone()));

    let block = Block::bordered()
        .title(view_title(
            name,
            None,
            feed_count(feed),
            feed_truncated(feed),
        ))
        .border_style(Emphasis::Blurred.border());

    let inner = block.inner(area);

    frame.render_widget(block, area);

    let issues = match feed.filter(|feed| !feed.items().is_empty()) {
        Some(feed) => feed.items(),
        None => {
            frame.render_widget(
                Paragraph::new(feed_placeholder(feed.map(Feed::status), spinner)),
                inner,
            );
            return;
        }
    };

    let groups = display::arrange(issues, GroupBy::Status, SortBy::Manual);
    let rows = Layout::vertical([Constraint::Length(2), Constraint::Min(1)]).split(inner);

    frame.render_widget(
        Paragraph::new(Text::from(vec![breakdown_line(&groups), Line::from("")])),
        rows[0],
    );

    let id_width = id_column_width(issues);
    let width = rows[1].width as usize;
    let (items, _) = view_items(issues, &groups, GroupBy::Status, None, id_width, width, now);

    frame.render_widget(List::new(items), rows[1]);
}
