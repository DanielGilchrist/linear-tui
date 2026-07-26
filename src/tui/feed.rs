use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::cache::{Access, Cache, CacheStatus, Phase, RefreshPolicy, Remote, Stale};
use crate::api::{Cursor, IssueFilter, IssueSummary, Page};

pub const FEED_REFRESH: RefreshPolicy = RefreshPolicy::new(60, 30 * 60);
pub const INBOX_REFRESH: RefreshPolicy = RefreshPolicy::new(30, 15 * 60);
pub const STALE_HORIZON: i64 = 7 * 24 * 60 * 60;
pub const PREFETCH_MARGIN: usize = 10;

pub trait HasId {
    fn feed_id(&self) -> &str;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedRequest {
    Refresh,
    LoadMore { after: Cursor },
}

impl FeedRequest {
    pub fn cursor(&self) -> Option<&Cursor> {
        match self {
            FeedRequest::Refresh => None,
            FeedRequest::LoadMore { after } => Some(after),
        }
    }
}

pub struct Feed<T> {
    page: Remote<Vec<T>>,
    next: Option<Cursor>,
    truncated: bool,
    appending: bool,
}

impl<T> Default for Feed<T> {
    fn default() -> Self {
        Self {
            page: Remote::default(),
            next: None,
            truncated: false,
            appending: false,
        }
    }
}

impl<T: HasId> Feed<T> {
    pub fn ready(page: Page<T>, now: i64) -> Self {
        Self {
            truncated: page.next.is_some(),
            next: page.next,
            page: Remote::ready(page.items, now),
            appending: false,
        }
    }

    pub fn restored(items: Vec<T>, truncated: bool, fetched_at: i64) -> Self {
        Self {
            page: Remote::ready(items, fetched_at),
            next: None,
            truncated,
            appending: false,
        }
    }

    pub fn items(&self) -> &[T] {
        self.page.value().map_or(&[], Vec::as_slice)
    }

    pub fn status(&self) -> &CacheStatus {
        self.page.status()
    }

    pub fn truncated(&self) -> bool {
        self.truncated
    }

    pub fn appending(&self) -> bool {
        self.appending
    }

    pub fn fetched_at(&self) -> i64 {
        self.page.fetched_at()
    }

    pub fn next(&self) -> Option<&Cursor> {
        self.next.as_ref()
    }

    pub fn in_flight(&self) -> bool {
        self.page.in_flight() || self.appending
    }

    pub fn can_load_more(&self) -> bool {
        self.next.is_some() && !self.in_flight()
    }

    pub fn needs_initial_load(&self) -> bool {
        self.page.phase() == Phase::Missing
    }

    pub fn access(&self, now: i64, policy: &RefreshPolicy) -> Access {
        self.page.access(now, policy)
    }

    pub fn begin_access(&mut self, now: i64, policy: &RefreshPolicy) -> bool {
        match self.access(now, policy) {
            Access::Skip => false,
            Access::Bust => {
                self.bust();
                self.begin(&FeedRequest::Refresh);
                true
            }
            Access::Load | Access::Revalidate => {
                self.begin(&FeedRequest::Refresh);
                true
            }
        }
    }

    pub fn begin(&mut self, request: &FeedRequest) {
        match request {
            FeedRequest::Refresh => self.page.begin(),
            FeedRequest::LoadMore { .. } => self.appending = true,
        }
    }

    pub fn bust(&mut self) {
        self.page.bust();
        self.next = None;
        self.truncated = false;
        self.appending = false;
    }

    pub fn apply(&mut self, request: &FeedRequest, page: Page<T>, now: i64) -> bool {
        match request {
            FeedRequest::Refresh => {
                self.truncated = page.next.is_some();
                self.next = page.next;
                self.page.set(page.items, now);
                self.appending = false;

                true
            }
            FeedRequest::LoadMore { after } => {
                if self.next.as_ref() != Some(after) {
                    self.appending = false;
                    return false;
                }

                if let Some(items) = self.page.value_mut() {
                    let seen: HashSet<String> = items
                        .iter()
                        .map(|item| item.feed_id().to_string())
                        .collect();

                    items.extend(
                        page.items
                            .into_iter()
                            .filter(|item| !seen.contains(item.feed_id())),
                    );
                }
                self.truncated = page.next.is_some();
                self.next = page.next;
                self.appending = false;

                true
            }
        }
    }

    pub fn fail(&mut self, error: String) {
        self.appending = false;
        self.page.fail(error);
    }
}

impl<T> Stale for Feed<T> {
    fn mark_stale(&mut self) {
        self.page.mark_stale();
    }
}

impl HasId for IssueSummary {
    fn feed_id(&self) -> &str {
        &self.id
    }
}

impl HasId for crate::api::NotificationItem {
    fn feed_id(&self) -> &str {
        &self.grouping_key
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FeedKey {
    Issues(IssueFilter),
    View(String),
    Search(String),
}

impl FeedKey {
    pub fn is_search(&self) -> bool {
        match self {
            FeedKey::Search(_) => true,
            FeedKey::Issues(_) | FeedKey::View(_) => false,
        }
    }
}

pub type FeedStore = Cache<FeedKey, Feed<IssueSummary>>;

#[cfg(test)]
mod tests {
    use super::*;

    struct Item(&'static str);
    impl HasId for Item {
        fn feed_id(&self) -> &str {
            self.0
        }
    }

    fn page(ids: &[&'static str], next: Option<&str>) -> Page<Item> {
        Page {
            items: ids.iter().map(|id| Item(id)).collect(),
            next: next.map(|cursor| Cursor(cursor.to_string())),
        }
    }

    fn ids(feed: &Feed<Item>) -> Vec<&str> {
        feed.items().iter().map(|item| item.0).collect()
    }

    #[test]
    fn refresh_replaces_and_records_the_cursor() {
        let mut feed: Feed<Item> = Feed::default();
        feed.begin(&FeedRequest::Refresh);
        assert_eq!(*feed.status(), CacheStatus::Loading);

        assert!(feed.apply(&FeedRequest::Refresh, page(&["a", "b"], Some("c1")), 100));

        assert_eq!(ids(&feed), vec!["a", "b"]);
        assert!(feed.truncated());
        assert!(feed.can_load_more());
        assert_eq!(*feed.status(), CacheStatus::Ready);
        assert_eq!(feed.fetched_at(), 100);
    }

    #[test]
    fn a_second_refresh_revalidates_rather_than_reloads() {
        let mut feed: Feed<Item> = Feed::default();
        feed.apply(&FeedRequest::Refresh, page(&["a"], None), 100);
        feed.begin(&FeedRequest::Refresh);
        assert_eq!(*feed.status(), CacheStatus::Revalidating);
    }

    #[test]
    fn load_more_appends_and_dedupes() {
        let mut feed: Feed<Item> = Feed::default();
        feed.apply(&FeedRequest::Refresh, page(&["a", "b"], Some("c1")), 100);

        let request = FeedRequest::LoadMore {
            after: Cursor("c1".into()),
        };
        feed.begin(&request);
        assert!(feed.appending());

        assert!(feed.apply(&request, page(&["b", "c"], None), 200));
        assert_eq!(ids(&feed), vec!["a", "b", "c"]);
        assert!(!feed.truncated());
        assert!(!feed.can_load_more());
        assert_eq!(feed.fetched_at(), 100);
    }

    #[test]
    fn a_stale_append_is_dropped_when_it_loses_a_race() {
        let mut feed: Feed<Item> = Feed::default();
        feed.apply(&FeedRequest::Refresh, page(&["a", "b"], Some("c1")), 100);

        let stale = FeedRequest::LoadMore {
            after: Cursor("c1".into()),
        };
        feed.begin(&stale);
        feed.apply(&FeedRequest::Refresh, page(&["x"], Some("c2")), 300);

        assert!(!feed.apply(&stale, page(&["a", "b"], None), 350));
        assert_eq!(ids(&feed), vec!["x"]);
        assert_eq!(feed.next(), Some(&Cursor("c2".into())));
    }

    #[test]
    fn a_restored_feed_renders_but_cannot_append() {
        let feed = Feed::restored(vec![Item("a")], true, 10);
        assert!(feed.truncated());
        assert!(feed.next().is_none());
        assert!(!feed.can_load_more());
    }

    #[test]
    fn bust_clears_everything() {
        let mut feed: Feed<Item> = Feed::default();
        feed.apply(&FeedRequest::Refresh, page(&["a", "b"], Some("c1")), 100);
        feed.bust();
        assert!(feed.items().is_empty());
        assert!(feed.next().is_none());
        assert!(!feed.truncated());
        assert_eq!(*feed.status(), CacheStatus::Idle);
    }

    #[test]
    fn failure_keeps_the_stale_rows() {
        let mut feed: Feed<Item> = Feed::default();
        feed.apply(&FeedRequest::Refresh, page(&["a", "b"], None), 100);
        feed.fail("boom".into());
        assert_eq!(ids(&feed), vec!["a", "b"]);
        assert_eq!(*feed.status(), CacheStatus::Failed("boom".into()));
    }
}
