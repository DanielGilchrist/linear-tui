use ratatui::layout::Size;
use ratatui::style::Style;
use ratatui::widgets::ListState;

use super::cache::{Cache, CacheStatus, RefreshPolicy, Remote};
use super::feed::{Feed, FeedKey, FeedStore};
use super::markdown;
use super::message::EncodeImage;
use super::render::image::{Encoded, Loaded};
use super::saved_views::SavedViewsPanel;
use super::view::{View, ViewKind};
use crate::api::{
    IssueDetail, IssueSummary, NotificationItem, Session, StateOption, Team, TeamId, Timestamp,
    User,
};

const IMAGE_REFRESH: RefreshPolicy = RefreshPolicy::new(24 * 60 * 60, 7 * 24 * 60 * 60);

#[derive(Default)]
pub struct RenderedDetail {
    pub description: markdown::Rendered,
    pub comment_bodies: Vec<markdown::Rendered>,
}

impl RenderedDetail {
    pub fn image_urls(&self) -> Vec<String> {
        let mut urls: Vec<String> = Vec::new();

        let blocks = std::iter::once(&self.description).chain(self.comment_bodies.iter());

        for rendered in blocks {
            for image in &rendered.images {
                if !urls.contains(&image.url) {
                    urls.push(image.url.clone());
                }
            }
        }

        urls
    }

    pub fn render(detail: &IssueDetail) -> Self {
        let description = detail
            .description
            .as_deref()
            .filter(|body| !body.is_empty())
            .map(|body| markdown::render(body, Style::default()))
            .unwrap_or_default();

        let comment_bodies = detail
            .threaded_comments()
            .into_iter()
            .map(|threaded| markdown::render(&threaded.comment.body, Style::default()))
            .collect();

        Self {
            description,
            comment_bodies,
        }
    }
}

pub struct TeamsPanel {
    pub teams: Remote<Vec<Team>>,
    pub state: ListState,
}

impl TeamsPanel {
    pub fn new() -> Self {
        Self {
            teams: Remote::default(),
            state: ListState::default().with_selected(Some(0)),
        }
    }

    pub fn list(&self) -> &[Team] {
        self.teams.value().map_or(&[], Vec::as_slice)
    }

    pub fn names(&self) -> Vec<String> {
        self.list().iter().map(|team| team.name.clone()).collect()
    }

    pub fn selected(&self) -> Option<&Team> {
        self.state.selected().and_then(|i| self.list().get(i))
    }
}

impl Default for TeamsPanel {
    fn default() -> Self {
        Self::new()
    }
}

pub struct WorkspaceData {
    pub session: Remote<Session>,
    pub feeds: FeedStore,
    pub inbox: Feed<NotificationItem>,
    detail: Remote<IssueDetail>,
    detail_markdown: RenderedDetail,
    pub states: Cache<TeamId, Remote<Vec<StateOption>>>,
    pub members: Cache<TeamId, Remote<Vec<User>>>,
    pub saved_views: SavedViewsPanel,
    pub recently_viewed: Vec<IssueSummary>,
    pub recent_state: ListState,
    pub teams: TeamsPanel,
    images: ImageStore,
}

pub type ImageStore = Cache<String, Remote<Loaded>>;

impl WorkspaceData {
    pub fn new() -> Self {
        Self {
            session: Remote::default(),
            feeds: FeedStore::default(),
            inbox: Feed::default(),
            detail: Remote::default(),
            detail_markdown: RenderedDetail::default(),
            states: Cache::default(),
            members: Cache::default(),
            saved_views: SavedViewsPanel::new(),
            recently_viewed: Vec::new(),
            recent_state: ListState::default().with_selected(Some(0)),
            teams: TeamsPanel::new(),
            images: ImageStore::default(),
        }
    }

    pub fn set_detail(&mut self, detail: IssueDetail, now: Timestamp) {
        self.detail_markdown = RenderedDetail::render(&detail);
        self.detail.set(detail, now);
    }

    pub fn bust_detail(&mut self) {
        self.detail.bust();
        self.detail_markdown = RenderedDetail::default();
    }

    pub fn begin_detail(&mut self) {
        self.detail.begin();
    }

    pub fn fail_detail(&mut self, error: String) {
        self.detail.fail(error);
    }

    pub fn detail(&self) -> &Remote<IssueDetail> {
        &self.detail
    }

    pub fn cancel_in_flight(&mut self) {
        let Self {
            session,
            feeds,
            inbox,
            detail,
            detail_markdown: _,
            states,
            members,
            saved_views,
            recently_viewed: _,
            recent_state: _,
            teams,
            images,
        } = self;

        session.cancel();
        detail.cancel();
        inbox.cancel();
        saved_views.views.cancel();
        teams.teams.cancel();

        for feed in feeds.values_mut() {
            feed.cancel();
        }

        for states in states.values_mut() {
            states.cancel();
        }

        for members in members.values_mut() {
            members.cancel();
        }

        for image in images.values_mut() {
            image.cancel();
        }
    }

    pub fn detail_markdown(&self) -> &RenderedDetail {
        &self.detail_markdown
    }

    pub fn images_in_flight(&self) -> bool {
        self.images
            .iter()
            .any(|(_, cell)| cell.in_flight() || cell.value().is_some_and(Loaded::is_encoding))
    }

    pub fn take_encode_requests(&mut self) -> Vec<EncodeImage> {
        self.images
            .iter_mut()
            .flat_map(|(url, cell)| {
                let requests = cell.value_mut().map(Loaded::take_requests);

                requests.into_iter().flatten().map(|request| EncodeImage {
                    url: url.clone(),
                    size: request.size,
                    source: request.source,
                })
            })
            .collect()
    }

    pub fn settle_encode(&mut self, url: &str, size: Size, encoded: Option<Encoded>) {
        let Some(loaded) = self
            .images
            .get_mut(&url.to_string())
            .and_then(Remote::value_mut)
        else {
            return;
        };

        match encoded {
            Some(encoded) => loaded.set_encoded(encoded),
            None => loaded.encode_failed(size),
        }
    }

    pub fn image(&self, url: &str) -> Option<&Remote<Loaded>> {
        self.images.get(&url.to_string())
    }

    pub fn set_image(&mut self, url: String, loaded: Loaded, now: Timestamp) {
        self.images.get_or_default(&url).set(loaded, now);
    }

    pub fn begin_image(&mut self, url: &str, now: Timestamp) -> bool {
        self.images
            .get_or_default(&url.to_string())
            .begin_access(now, &IMAGE_REFRESH)
    }

    pub fn fail_image(&mut self, url: &str, error: String) {
        self.images.get_or_default(&url.to_string()).fail(error);
    }

    pub fn overlay_render_parts(&mut self) -> (&FeedStore, &mut ImageStore) {
        (&self.feeds, &mut self.images)
    }

    pub fn detail_render_parts(
        &mut self,
    ) -> (&Remote<IssueDetail>, &RenderedDetail, &mut ImageStore) {
        (&self.detail, &self.detail_markdown, &mut self.images)
    }
}

impl WorkspaceData {
    pub fn issues_for(&self, view: &View) -> &[IssueSummary] {
        match &view.kind {
            ViewKind::Issues(filter) => self
                .feeds
                .get(&FeedKey::Issues(filter.clone()))
                .map_or(&[], |feed| feed.items()),
            ViewKind::Inbox => &[],
        }
    }

    pub fn feed_status_for(&self, view: &View) -> CacheStatus {
        match &view.kind {
            ViewKind::Issues(filter) => self
                .feeds
                .get(&FeedKey::Issues(filter.clone()))
                .map_or(CacheStatus::Idle, |feed| feed.status()),
            ViewKind::Inbox => self.inbox.status(),
        }
    }

    pub fn appending_for(&self, view: &View) -> bool {
        match &view.kind {
            ViewKind::Issues(filter) => self
                .feeds
                .get(&FeedKey::Issues(filter.clone()))
                .is_some_and(|feed| feed.appending()),
            ViewKind::Inbox => self.inbox.appending(),
        }
    }
}

impl Default for WorkspaceData {
    fn default() -> Self {
        Self::new()
    }
}
