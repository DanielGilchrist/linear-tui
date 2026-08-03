use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Span, Text},
    widgets::{Block, List, Paragraph},
    Frame,
};

use super::super::format::id_column_width;
use super::super::theme::{self, Emphasis};
use super::super::widgets::{breakdown_line, loading_more_row, view_items, view_title};
use super::{feed_count, feed_placeholder, feed_truncated, Viewport};
use crate::api::Timestamp;
use crate::tui::feed::{Feed, FeedStore};
use crate::tui::saved_views::ViewSurface;
use crate::tui::spinner::Spinner;
use crate::tui::team::TeamMode;

pub const VIEW_HEADER_ROWS: u16 = 3;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    feeds: &FeedStore,
    view: &mut ViewSurface,
    spinner: Spinner,
    emphasis: Emphasis,
    now: Timestamp,
) -> Viewport {
    let feed = feeds.get(&view.key());

    let block = Block::bordered()
        .title(Span::styled(
            view_title(
                view.name(),
                view.mode().map(TeamMode::label),
                feed_count(feed),
                feed_truncated(feed),
            ),
            emphasis.title(),
        ))
        .border_style(emphasis.border());

    let inner = block.inner(area);

    frame.render_widget(block, area);

    let issues = match feed.filter(|feed| !feed.items().is_empty()) {
        Some(feed) => feed.items(),
        None => {
            frame.render_widget(
                Paragraph::new(feed_placeholder(feed.map(Feed::status), spinner)),
                inner,
            );
            return Viewport((inner.height as usize).saturating_sub(VIEW_HEADER_ROWS as usize));
        }
    };

    let groups = view.display.arrange(issues);
    let rows =
        Layout::vertical([Constraint::Length(VIEW_HEADER_ROWS), Constraint::Min(1)]).split(inner);

    let header = Text::from(vec![
        Line::from(vec![
            Span::styled("group ", theme::dim()),
            Span::styled(view.display.group.label(), theme::TEXT),
            Span::raw("    "),
            Span::styled("sort ", theme::dim()),
            Span::styled(view.display.sort.label(), theme::TEXT),
        ]),
        breakdown_line(&groups),
    ]);

    frame.render_widget(Paragraph::new(header), rows[0]);

    let id_width = id_column_width(issues);
    let width = rows[1].width as usize;
    let (mut items, selected_row) = view_items(
        issues,
        &groups,
        view.display.group,
        view.state.selected(),
        id_width,
        width,
        now,
    );

    if feed.is_some_and(|feed| feed.appending()) {
        items.push(loading_more_row(spinner));
    }

    view.layout.select(selected_row);

    let list = List::new(items)
        .highlight_style(emphasis.highlight())
        .scroll_padding(1);

    frame.render_stateful_widget(list, rows[1], &mut view.layout);

    Viewport(rows[1].height as usize)
}
