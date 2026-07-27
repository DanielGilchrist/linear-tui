use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::api::{Credential, IssueSummary, NotificationItem, Timestamp};
use crate::tui::feed::{Feed, FeedKey, FeedStore, HasId, STALE_HORIZON};

const FEEDS_VERSION: u32 = 2;
const FEED_ITEM_CAP: usize = 100;
const ACCOUNTS_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Account {
    pub workspace_key: String,
    pub org_name: String,
    pub credential: Credential,
}

impl Account {
    pub fn namespace(&self) -> String {
        namespace(&self.workspace_key)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedAccounts {
    version: u32,
    #[serde(default)]
    active: Option<String>,
    accounts: Vec<Account>,
}

#[derive(Debug, Clone, Default)]
pub struct Accounts {
    pub accounts: Vec<Account>,
    pub active: Option<String>,
}

fn accounts_path() -> Option<PathBuf> {
    state_dir().map(|dir| dir.join("accounts.json"))
}

pub fn load_accounts() -> Accounts {
    let Some(path) = accounts_path() else {
        return Accounts::default();
    };

    let Ok(raw) = std::fs::read_to_string(path) else {
        return Accounts::default();
    };

    match serde_json::from_str::<PersistedAccounts>(&raw) {
        Ok(store) if store.version == ACCOUNTS_VERSION => Accounts {
            accounts: store.accounts,
            active: store.active,
        },
        _ => Accounts::default(),
    }
}

pub fn save_accounts(accounts: &[Account], active: Option<&str>) {
    let Some(path) = accounts_path() else {
        return;
    };

    let store = PersistedAccounts {
        version: ACCOUNTS_VERSION,
        active: active.map(str::to_string),
        accounts: accounts.to_vec(),
    };

    if let Ok(json) = serde_json::to_string(&store) {
        write_atomic(&path, &json);
        restrict_permissions(&path);
    }
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) {}

fn state_dir() -> Option<PathBuf> {
    crate::tui::platform::Platform::host().state_dir()
}

pub fn namespace(api_key: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;

    for byte in api_key.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    format!("{hash:016x}")
}

fn recent_path(namespace: &str) -> Option<PathBuf> {
    state_dir().map(|dir| dir.join(format!("recently-viewed-{namespace}.json")))
}

fn feeds_path(namespace: &str) -> Option<PathBuf> {
    state_dir().map(|dir| dir.join(format!("feeds-{namespace}.json")))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedFeed<T> {
    pub items: Vec<T>,
    pub truncated: bool,
    pub fetched_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedCache {
    pub version: u32,
    pub issues: Vec<(FeedKey, PersistedFeed<IssueSummary>)>,
    pub inbox: Option<PersistedFeed<NotificationItem>>,
}

fn persist_feed<T: HasId + Clone>(feed: &Feed<T>) -> PersistedFeed<T> {
    let capped = feed.items().len() > FEED_ITEM_CAP;

    PersistedFeed {
        items: feed.items().iter().take(FEED_ITEM_CAP).cloned().collect(),
        truncated: feed.truncated() || capped,
        fetched_at: feed.fetched_at(),
    }
}

pub fn build_cache(
    feeds: &FeedStore,
    inbox: &Feed<NotificationItem>,
    now: Timestamp,
) -> PersistedCache {
    let issues = feeds
        .iter()
        .filter(|(key, _)| !key.is_search())
        .filter(|(_, feed)| !feed.items().is_empty() && fresh_enough(now, feed.fetched_at()))
        .map(|(key, feed)| (key.clone(), persist_feed(feed)))
        .collect();

    let inbox = (!inbox.items().is_empty() && fresh_enough(now, inbox.fetched_at()))
        .then(|| persist_feed(inbox));

    PersistedCache {
        version: FEEDS_VERSION,
        issues,
        inbox,
    }
}

pub fn load_feeds(namespace: &str) -> Option<PersistedCache> {
    let raw = std::fs::read_to_string(feeds_path(namespace)?).ok()?;
    let cache: PersistedCache = serde_json::from_str(&raw).ok()?;

    (cache.version == FEEDS_VERSION).then_some(cache)
}

fn write_atomic(path: &Path, contents: &str) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }

    let tmp = path.with_extension(format!("tmp{}", std::process::id()));
    if std::fs::write(&tmp, contents).is_ok() && std::fs::rename(&tmp, path).is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
}

pub fn save_feeds(namespace: &str, cache: &PersistedCache) {
    let Some(path) = feeds_path(namespace) else {
        return;
    };

    if let Ok(json) = serde_json::to_string(cache) {
        write_atomic(&path, &json);
    }

    prune_orphan_namespaces(namespace);
}

fn prune_orphan_namespaces(current: &str) {
    let Some(dir) = state_dir() else {
        return;
    };

    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };

    let horizon = std::time::Duration::from_secs(STALE_HORIZON as u64);

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();

        let ours = (name.starts_with("feeds-") || name.starts_with("recently-viewed-"))
            && name.ends_with(".json");

        if !ours || name.contains(current) {
            continue;
        }

        let stale = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age > horizon);

        if stale {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

pub fn load_recent(namespace: &str) -> Vec<IssueSummary> {
    recent_path(namespace)
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save_recent(namespace: &str, issues: &[IssueSummary]) {
    let Some(path) = recent_path(namespace) else {
        return;
    };

    if let Ok(json) = serde_json::to_string(issues) {
        write_atomic(&path, &json);
    }
}

fn fresh_enough(now: Timestamp, fetched_at: Timestamp) -> bool {
    now.seconds_since(fetched_at) <= STALE_HORIZON
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{
        IssueFilter, IssueId, IssueSummary, Page, StateType, TeamId, ViewId, WorkflowState,
    };
    use crate::tui::feed::Feed;

    fn issue(id: &str) -> IssueSummary {
        IssueSummary {
            id: IssueId::from_raw(id),
            identifier: id.into(),
            title: None,
            state: WorkflowState {
                name: "Todo".into(),
                state_type: StateType::Unstarted,
            },
            priority: Default::default(),
            assignee: None,
            labels: Vec::new(),
            url: String::new(),
            branch_name: String::new(),
            team_id: TeamId::default(),
            updated_at: Default::default(),
        }
    }

    #[test]
    fn build_cache_caps_items_marks_truncated_and_drops_search_and_stale() {
        let mut feeds = FeedStore::default();
        let many: Vec<IssueSummary> = (0..FEED_ITEM_CAP + 5)
            .map(|n| issue(&format!("i{n}")))
            .collect();

        feeds.insert(
            FeedKey::Issues(IssueFilter::assigned_to_me()),
            Feed::ready(Page::single(many), Timestamp::from_epoch(1_000)),
        );
        feeds.insert(
            FeedKey::Search("oven".into()),
            Feed::ready(
                Page::single(vec![issue("s1")]),
                Timestamp::from_epoch(1_000),
            ),
        );
        feeds.insert(
            FeedKey::View(ViewId::from_raw("old")),
            Feed::ready(Page::single(vec![issue("o1")]), Timestamp::from_epoch(0)),
        );

        let now = Timestamp::from_epoch(STALE_HORIZON + 500);
        let cache = build_cache(&feeds, &Feed::default(), now);

        assert_eq!(cache.issues.len(), 1, "search and stale feeds excluded");

        let (_, persisted) = &cache.issues[0];
        assert_eq!(persisted.items.len(), FEED_ITEM_CAP);
        assert!(persisted.truncated, "a capped feed is marked truncated");
    }

    #[test]
    fn write_atomic_writes_contents_and_leaves_no_temp_file() {
        let dir = std::env::temp_dir().join(format!("linear-tui-store-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("feeds.json");

        write_atomic(&path, "{\"ok\":true}");

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{\"ok\":true}");

        let stragglers = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| entry.file_name().to_string_lossy().contains("tmp"));

        assert!(
            !stragglers,
            "temp file should be renamed away, not left behind"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn namespace_is_stable_and_per_key() {
        assert_eq!(namespace("key-a"), namespace("key-a"));
        assert_ne!(namespace("key-a"), namespace("key-b"));
    }

    #[test]
    fn a_version_mismatch_is_discarded() {
        let cache = PersistedCache {
            version: FEEDS_VERSION + 1,
            issues: Vec::new(),
            inbox: None,
        };

        let json = serde_json::to_string(&cache).unwrap();
        let parsed: PersistedCache = serde_json::from_str(&json).unwrap();

        assert!((parsed.version == FEEDS_VERSION)
            .then_some(parsed)
            .is_none());
    }
}
