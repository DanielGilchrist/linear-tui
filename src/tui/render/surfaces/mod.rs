pub mod detail;
pub mod footer;
pub mod my_work;
pub mod recent;
pub mod saved_views;
pub mod stub;
pub mod view;

use ratatui::text::Line;

use super::widgets::{placeholder, PlaceholderText};
use crate::api::IssueSummary;
use crate::tui::cache::CacheStatus;
use crate::tui::feed::Feed;
use crate::tui::spinner::Spinner;

#[derive(Clone, Copy)]
#[must_use = "write this back to app.viewport"]
pub struct Viewport(pub usize);

pub(super) const LOADING_TEXT: &str = "Loading…";
pub(super) const LOAD_FAILED_TEXT: &str = "Failed to load  ·  r to retry";

pub(super) fn feed_count(feed: Option<&Feed<IssueSummary>>) -> Option<usize> {
    feed.filter(|feed| !feed.items().is_empty())
        .map(|feed| feed.items().len())
}

pub(super) fn feed_truncated(feed: Option<&Feed<IssueSummary>>) -> bool {
    feed.is_some_and(|feed| feed.truncated())
}

pub(super) fn feed_placeholder(status: Option<&CacheStatus>, spinner: Spinner) -> Line<'static> {
    placeholder(
        status,
        PlaceholderText {
            empty: "No issues in this view",
            loading: LOADING_TEXT,
            failed: LOAD_FAILED_TEXT,
        },
        spinner,
    )
}
