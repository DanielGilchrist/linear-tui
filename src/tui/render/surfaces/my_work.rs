use ratatui::{layout::Rect, widgets::ListState, Frame};

use super::super::theme::Emphasis;
use super::super::widgets::{
    issue_items, loading_more_row, notification_items, placeholder, view_tabs, PlaceholderText,
    StyledList,
};
use crate::api::{IssueSummary, NotificationItem};
use crate::tui::cache::CacheStatus;
use crate::tui::spinner::Spinner;
use crate::tui::view::View;

pub enum MyWorkContent<'a> {
    Issues { issues: &'a [IssueSummary] },
    Inbox { items: &'a [NotificationItem] },
}

pub struct MyWorkProps<'a> {
    pub views: &'a [View],
    pub active: usize,
    pub content: MyWorkContent<'a>,
    pub status: CacheStatus,
    pub appending: bool,
    pub list_state: &'a mut ListState,
    pub emphasis: Emphasis,
    pub spinner: Spinner,
}

pub fn render(frame: &mut Frame, area: Rect, props: MyWorkProps) {
    let MyWorkProps {
        views,
        active,
        content,
        status,
        appending,
        list_state,
        emphasis,
        spinner,
    } = props;

    let max_title = area.width.saturating_sub(2) as usize;
    let title = view_tabs(
        views,
        active,
        status.in_flight() || appending,
        spinner,
        max_title,
    );

    let (mut items, total, empty) = match content {
        MyWorkContent::Inbox { items } => (notification_items(items), items.len(), "Inbox empty"),
        MyWorkContent::Issues { issues } => {
            (issue_items(issues), issues.len(), "No issues in this view")
        }
    };

    if appending && total > 0 {
        items.push(loading_more_row(spinner));
    }

    let selected = list_state.selected();
    let list = StyledList::new("My Work")
        .title_line(title)
        .items(items)
        .emphasis(emphasis)
        .state(list_state)
        .position(selected, total);

    let list = match total {
        0 => {
            let text = PlaceholderText {
                empty,
                loading: super::LOADING_TEXT,
                failed: super::LOAD_FAILED_TEXT,
            };

            list.placeholder(placeholder(Some(status), text, spinner))
        }
        _ => list,
    };

    frame.render_widget(list, area);
}
