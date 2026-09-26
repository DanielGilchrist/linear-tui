use std::collections::HashSet;

use ratatui::layout::Size;
use ratatui::style::Style;
use ratatui::widgets::ListState;

use super::cache::{Cache, CacheStatus, RefreshPolicy, Remote};
use super::feed::{Feed, FeedKey, FeedStore};
use super::markdown;
use super::message::EncodeImage;
use super::render::image::{DrawnImage, EncodeFailure, Encoded, Loaded};
use super::saved_views::SavedViewsPanel;
use super::view::{View, ViewKind};
use crate::api::ImageUrl;
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
    pub fn contains_image(&self, url: &ImageUrl) -> bool {
        std::iter::once(&self.description)
            .chain(self.comment_bodies.iter())
            .any(|rendered| rendered.images.iter().any(|image| image.url == *url))
    }

    pub fn image_urls(&self) -> Vec<ImageUrl> {
        let mut urls: Vec<ImageUrl> = Vec::new();

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
    expanded_images: HashSet<ImageUrl>,
}

pub type ImageStore = Cache<ImageUrl, Remote<Loaded>>;

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
            expanded_images: HashSet::new(),
        }
    }

    pub fn set_detail(&mut self, detail: IssueDetail, now: Timestamp) {
        self.detail_markdown = RenderedDetail::render(&detail);
        self.detail.set(detail, now);

        let rendered = &self.detail_markdown;
        self.images.retain(|url, _| rendered.contains_image(url));
        self.expanded_images
            .retain(|url| rendered.contains_image(url));
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
            expanded_images: _,
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

    pub fn claim_encode(&mut self, drawn: &DrawnImage) -> Option<EncodeImage> {
        let request = self
            .images
            .get_mut(&drawn.url)?
            .value_mut()?
            .claim(drawn.size)?;

        Some(EncodeImage {
            url: drawn.url.clone(),
            request,
        })
    }

    pub fn settle_encode(
        &mut self,
        url: &ImageUrl,
        size: Size,
        encoded: Result<Encoded, EncodeFailure>,
    ) {
        let Some(loaded) = self.images.get_mut(url).and_then(Remote::value_mut) else {
            return;
        };

        match encoded {
            Ok(encoded) => loaded.set_encoded(encoded),
            Err(failure) => loaded.encode_failed(size, failure),
        }
    }

    pub fn loading_images(&self) -> Vec<ImageUrl> {
        self.images
            .iter()
            .filter(|(_, cell)| cell.in_flight())
            .map(|(url, _)| url.clone())
            .collect()
    }

    pub fn expanded_images(&self) -> &HashSet<ImageUrl> {
        &self.expanded_images
    }

    pub fn toggle_images(&mut self, urls: &[ImageUrl]) -> bool {
        let expanding = urls.iter().any(|url| !self.expanded_images.contains(url));

        for url in urls {
            if expanding {
                self.expanded_images.insert(url.clone());
            } else {
                self.expanded_images.remove(url);
            }
        }

        expanding
    }

    pub fn image(&self, url: &ImageUrl) -> Option<&Remote<Loaded>> {
        self.images.get(url)
    }

    pub fn set_image(&mut self, url: &ImageUrl, loaded: Loaded, now: Timestamp) {
        if let Some(cell) = self.images.get_mut(url) {
            cell.set(loaded, now);
        }
    }

    pub fn begin_image(&mut self, url: &ImageUrl, now: Timestamp) -> bool {
        if !self.detail_markdown.contains_image(url) {
            return false;
        }

        self.images
            .get_or_default(url)
            .begin_access(now, &IMAGE_REFRESH)
    }

    pub fn fail_image(&mut self, url: &ImageUrl, error: String) {
        if let Some(cell) = self.images.get_mut(url) {
            cell.fail(error);
        }
    }

    pub fn overlay_render_parts(&self) -> (&FeedStore, &ImageStore) {
        (&self.feeds, &self.images)
    }

    pub fn detail_render_parts(&self) -> (&Remote<IssueDetail>, &RenderedDetail, &ImageStore) {
        (&self.detail, &self.detail_markdown, &self.images)
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
