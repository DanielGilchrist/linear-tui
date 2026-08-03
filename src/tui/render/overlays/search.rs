use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::ListItem,
    Frame,
};

use super::super::theme::{self, Emphasis};
use super::super::widgets::{loading_more_row, placeholder, PlaceholderText, StyledList};
use crate::api::IssueSummary;
use crate::tui::cache::CacheStatus;
use crate::tui::layout;
use crate::tui::overlay::Search;
use crate::tui::spinner::Spinner;

pub struct SearchFeed<'a> {
    pub results: &'a [IssueSummary],
    pub status: Option<CacheStatus>,
    pub appending: bool,
}

pub fn area(frame_area: Rect) -> Rect {
    layout::centred_rect(frame_area, 60, 60)
}

pub fn render(
    search: &mut Search,
    feed: SearchFeed,
    spinner: Spinner,
    frame: &mut Frame,
    area: Rect,
) {
    let SearchFeed {
        results,
        status,
        appending,
    } = feed;

    let mut rows: Vec<ListItem> = results.iter().map(result_row).collect();

    if appending && !results.is_empty() {
        rows.push(loading_more_row(spinner));
    }

    let placeholder = placeholder(
        status,
        PlaceholderText {
            empty: "No matches",
            loading: "Searching…",
            failed: "Search failed  ·  esc to close",
        },
        spinner,
    );

    let title = format!("Search  {}", search.query);
    let selected = search.state.selected();
    let total = results.len();

    let list = StyledList::new(&title)
        .items(rows)
        .emphasis(Emphasis::Focused)
        .state(&mut search.state)
        .position(selected, total)
        .placeholder(placeholder);

    frame.render_widget(list, area);
}

fn result_row(issue: &IssueSummary) -> ListItem<'static> {
    ListItem::new(Line::from(vec![
        Span::styled(issue.identifier.clone(), theme::dim()),
        Span::raw("  "),
        Span::styled(
            issue.title.clone().unwrap_or_else(|| "Untitled".into()),
            theme::TEXT,
        ),
    ]))
}
