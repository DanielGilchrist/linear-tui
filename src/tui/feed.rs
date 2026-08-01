use std::collections::HashSet;
use std::hash::Hash;

use serde::{Deserialize, Serialize};

use super::cache::{Access, Cache, CacheStatus, Phase, RefreshPolicy, Remote, Stale};
use crate::api::{Cursor, IssueFilter, IssueId, IssueSummary, Page, Timestamp, ViewId};

pub const FEED_REFRESH: RefreshPolicy = RefreshPolicy::new(60, 30 * 60);
pub const INBOX_REFRESH: RefreshPolicy = RefreshPolicy::new(30, 15 * 60);
pub const STALE_HORIZON: i64 = 7 * 24 * 60 * 60;
pub const PREFETCH_MARGIN: usize = 10;

pub trait HasId {
    type Id: Eq + Hash + Clone;

    fn feed_id(&self) -> &Self::Id;
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

#[derive(Debug, Clone, Default)]
enum Pagination {
    #[default]
    Complete,
    More(Cursor),
    Appending(Cursor),
    TruncatedRestored,
}

impl Pagination {
    fn from_next(next: Option<Cursor>) -> Self {
        match next {
            Some(cursor) => Pagination::More(cursor),
            None => Pagination::Complete,
        }
    }

    fn cursor(&self) -> Option<&Cursor> {
        match self {
            Pagination::More(cursor) => Some(cursor),
            Pagination::Appending(_) | Pagination::Complete | Pagination::TruncatedRestored => None,
        }
    }

    fn appending(&self) -> bool {
        matches!(self, Pagination::Appending(_))
    }

    fn awaits(&self, after: &Cursor) -> bool {
        matches!(self, Pagination::Appending(cursor) if cursor == after)
    }

    fn begin(&mut self, after: &Cursor) {
        if matches!(self, Pagination::More(cursor) if cursor == after) {
            *self = Pagination::Appending(after.clone());
        }
    }

    fn settle(&mut self) {
        if let Pagination::Appending(cursor) = self {
            *self = Pagination::More(cursor.clone());
        }
    }

    fn truncated(&self) -> bool {
        !matches!(self, Pagination::Complete)
    }
}

pub struct Feed<T> {
    page: Remote<Vec<T>>,
    pagination: Pagination,
}

impl<T> Default for Feed<T> {
    fn default() -> Self {
        Self {
            page: Remote::default(),
            pagination: Pagination::Complete,
        }
    }
}

impl<T: HasId> Feed<T> {
    pub fn ready(page: Page<T>, now: Timestamp) -> Self {
        Self {
            pagination: Pagination::from_next(page.next),
            page: Remote::ready(page.items, now),
        }
    }

    pub fn restored(items: Vec<T>, truncated: bool, fetched_at: Timestamp) -> Self {
        Self {
            page: Remote::ready(items, fetched_at),
            pagination: if truncated {
                Pagination::TruncatedRestored
            } else {
                Pagination::Complete
            },
        }
    }

    pub fn items(&self) -> &[T] {
        self.page.value().map_or(&[], Vec::as_slice)
    }

    pub fn status(&self) -> CacheStatus {
        self.page.status()
    }

    pub fn truncated(&self) -> bool {
        self.pagination.truncated()
    }

    pub fn appending(&self) -> bool {
        self.pagination.appending()
    }

    pub fn fetched_at(&self) -> Timestamp {
        self.page.fetched_at()
    }

    pub fn next(&self) -> Option<&Cursor> {
        self.pagination.cursor()
    }

    pub fn in_flight(&self) -> bool {
        self.page.in_flight() || self.appending()
    }

    pub fn can_load_more(&self) -> bool {
        matches!(self.pagination, Pagination::More(_)) && !self.in_flight()
    }

    pub fn needs_initial_load(&self) -> bool {
        self.page.phase() == Phase::Missing
    }

    pub fn access(&self, now: Timestamp, policy: &RefreshPolicy) -> Access {
        self.page.access(now, policy)
    }

    pub fn begin_access(&mut self, now: Timestamp, policy: &RefreshPolicy) -> bool {
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
            FeedRequest::LoadMore { after } => self.pagination.begin(after),
        }
    }

    pub fn bust(&mut self) {
        self.page.bust();
        self.pagination = Pagination::Complete;
    }

    pub fn apply(&mut self, request: &FeedRequest, page: Page<T>, now: Timestamp) -> bool {
        match request {
            FeedRequest::Refresh => {
                self.pagination = Pagination::from_next(page.next);
                self.page.set(page.items, now);

                true
            }
            FeedRequest::LoadMore { after } => {
                if !self.pagination.awaits(after) {
                    return false;
                }

                if let Some(items) = self.page.value_mut() {
                    let seen: HashSet<T::Id> =
                        items.iter().map(|item| item.feed_id().clone()).collect();

                    items.extend(
                        page.items
                            .into_iter()
                            .filter(|item| !seen.contains(item.feed_id())),
                    );
                }
                self.pagination = Pagination::from_next(page.next);

                true
            }
        }
    }

    pub fn fail(&mut self, error: String) {
        self.pagination.settle();
        self.page.fail(error);
    }

    pub fn cancel(&mut self) {
        self.pagination.settle();
        self.page.cancel();
    }
}

impl<T> Stale for Feed<T> {
    fn mark_stale(&mut self) {
        self.page.mark_stale();
    }
}

impl HasId for IssueSummary {
    type Id = IssueId;

    fn feed_id(&self) -> &IssueId {
        &self.id
    }
}

impl HasId for crate::api::NotificationItem {
    type Id = String;

    fn feed_id(&self) -> &String {
        &self.grouping_key
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FeedKey {
    Issues(IssueFilter),
    View(ViewId),
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

    fn at(seconds: i64) -> Timestamp {
        Timestamp::from_epoch(seconds)
    }

    struct Item(String);
    impl HasId for Item {
        type Id = String;

        fn feed_id(&self) -> &String {
            &self.0
        }
    }

    fn page(ids: &[&'static str], next: Option<&str>) -> Page<Item> {
        Page {
            items: ids.iter().map(|id| Item(id.to_string())).collect(),
            next: next.map(|cursor| Cursor(cursor.to_string())),
        }
    }

    fn ids(feed: &Feed<Item>) -> Vec<&str> {
        feed.items().iter().map(|item| item.0.as_str()).collect()
    }

    #[test]
    fn refresh_replaces_and_records_the_cursor() {
        let mut feed: Feed<Item> = Feed::default();
        feed.begin(&FeedRequest::Refresh);
        assert_eq!(feed.status(), CacheStatus::Loading);

        assert!(feed.apply(
            &FeedRequest::Refresh,
            page(&["a", "b"], Some("c1")),
            at(100)
        ));

        assert_eq!(ids(&feed), vec!["a", "b"]);
        assert!(feed.truncated());
        assert!(feed.can_load_more());
        assert_eq!(feed.status(), CacheStatus::Ready);
        assert_eq!(feed.fetched_at(), at(100));
    }

    #[test]
    fn a_second_refresh_revalidates_rather_than_reloads() {
        let mut feed: Feed<Item> = Feed::default();
        feed.apply(&FeedRequest::Refresh, page(&["a"], None), at(100));
        feed.begin(&FeedRequest::Refresh);
        assert_eq!(feed.status(), CacheStatus::Revalidating);
    }

    #[test]
    fn load_more_appends_and_dedupes() {
        let mut feed: Feed<Item> = Feed::default();
        feed.apply(
            &FeedRequest::Refresh,
            page(&["a", "b"], Some("c1")),
            at(100),
        );

        let request = FeedRequest::LoadMore {
            after: Cursor("c1".into()),
        };
        feed.begin(&request);
        assert!(feed.appending());

        assert!(feed.apply(&request, page(&["b", "c"], None), at(200)));
        assert_eq!(ids(&feed), vec!["a", "b", "c"]);
        assert!(!feed.truncated());
        assert!(!feed.can_load_more());
        assert_eq!(feed.fetched_at(), at(100));
    }

    #[test]
    fn a_stale_append_is_dropped_when_it_loses_a_race() {
        let mut feed: Feed<Item> = Feed::default();
        feed.apply(
            &FeedRequest::Refresh,
            page(&["a", "b"], Some("c1")),
            at(100),
        );

        let stale = FeedRequest::LoadMore {
            after: Cursor("c1".into()),
        };
        feed.begin(&stale);
        feed.apply(&FeedRequest::Refresh, page(&["x"], Some("c2")), at(300));

        assert!(!feed.apply(&stale, page(&["a", "b"], None), at(350)));
        assert_eq!(ids(&feed), vec!["x"]);
        assert_eq!(feed.next(), Some(&Cursor("c2".into())));
    }

    #[test]
    fn a_cancelled_append_can_be_asked_for_again() {
        let mut feed: Feed<Item> = Feed::default();
        feed.apply(
            &FeedRequest::Refresh,
            page(&["a", "b"], Some("c1")),
            at(100),
        );

        let request = FeedRequest::LoadMore {
            after: Cursor("c1".into()),
        };
        feed.begin(&request);
        feed.cancel();

        assert!(!feed.appending());
        assert!(
            feed.can_load_more(),
            "a cancelled append must leave the cursor askable again"
        );
        assert_eq!(feed.next(), Some(&Cursor("c1".into())));
    }

    #[test]
    fn an_append_reply_for_a_different_cursor_is_dropped() {
        let mut feed: Feed<Item> = Feed::default();
        feed.apply(
            &FeedRequest::Refresh,
            page(&["a", "b"], Some("c1")),
            at(100),
        );

        let first = FeedRequest::LoadMore {
            after: Cursor("c1".into()),
        };
        feed.begin(&first);
        feed.fail("boom".into());

        feed.apply(&FeedRequest::Refresh, page(&["x"], Some("c2")), at(200));

        let second = FeedRequest::LoadMore {
            after: Cursor("c2".into()),
        };
        feed.begin(&second);

        assert!(
            !feed.apply(&first, page(&["y"], None), at(300)),
            "a reply for a cursor the feed is not awaiting must be rejected"
        );
        assert_eq!(ids(&feed), vec!["x"]);
        assert!(feed.appending(), "the live append is still outstanding");
    }

    #[test]
    fn a_restored_feed_renders_but_cannot_append() {
        let feed = Feed::restored(vec![Item("a".to_string())], true, at(10));
        assert!(feed.truncated());
        assert!(feed.next().is_none());
        assert!(!feed.can_load_more());
    }

    #[test]
    fn bust_clears_everything() {
        let mut feed: Feed<Item> = Feed::default();
        feed.apply(
            &FeedRequest::Refresh,
            page(&["a", "b"], Some("c1")),
            at(100),
        );
        feed.bust();
        assert!(feed.items().is_empty());
        assert!(feed.next().is_none());
        assert!(!feed.truncated());
        assert_eq!(feed.status(), CacheStatus::Idle);
    }

    #[test]
    fn failure_keeps_the_stale_rows() {
        let mut feed: Feed<Item> = Feed::default();
        feed.apply(&FeedRequest::Refresh, page(&["a", "b"], None), at(100));
        feed.fail("boom".into());
        assert_eq!(ids(&feed), vec!["a", "b"]);
        assert_eq!(feed.status(), CacheStatus::Failed("boom".into()));
    }
}
