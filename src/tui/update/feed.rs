use super::nav::{clamp_selection, reselect_view};
use crate::api::IssueId;
use crate::tui::app::App;
use crate::tui::cache::RefreshPolicy;
use crate::tui::feed::{
    Feed, FeedKey, FeedRequest, HasId, FEED_REFRESH, INBOX_REFRESH, PREFETCH_MARGIN,
};
use crate::tui::focus::{Focus, LeftPanel, Reveal};
use crate::tui::message::{ApiCommand, Effect, Effects, StoreCommand};
use crate::tui::saved_views::ViewSurface;

const TEAMS_REFRESH: RefreshPolicy = RefreshPolicy::new(60 * 60, 24 * 60 * 60);

pub fn initial_commands(app: &mut App) -> Effects {
    app.workspace.saved_views.views.begin();

    let mut commands = access_session(app);

    commands.extend([
        Effect::Store(StoreCommand::LoadRecent),
        Effect::Api(ApiCommand::LoadCustomViews),
    ]);

    commands.extend(access_teams(app));
    commands.extend(access_active(app));

    commands
}

pub fn restore_feeds(app: &mut App, cache: crate::store::PersistedCache) {
    for (key, persisted) in cache.issues {
        app.workspace.feeds.get_or_insert_with(key, || {
            Feed::restored(persisted.items, persisted.truncated, persisted.fetched_at)
        });
    }

    if app.workspace.inbox.needs_initial_load() {
        if let Some(persisted) = cache.inbox {
            app.workspace.inbox =
                Feed::restored(persisted.items, persisted.truncated, persisted.fetched_at);
        }
    }
}

pub(super) fn selected_view_key(app: &App) -> Option<FeedKey> {
    app.workspace
        .saved_views
        .selected_view()
        .map(|view| FeedKey::View(view.id.clone()))
}

pub(super) fn access_feed(app: &mut App, key: FeedKey) -> Effects {
    let began = app
        .workspace
        .feeds
        .get_or_default(&key)
        .begin_access(app.now, &FEED_REFRESH);

    Effects::when(
        began,
        Effect::Api(ApiCommand::LoadFeed {
            key,
            request: FeedRequest::Refresh,
        }),
    )
}

pub(super) fn access_session(app: &mut App) -> Effects {
    if app.workspace.session.in_flight() {
        return Effects::default();
    }

    app.workspace.session.begin();

    Effects::one(Effect::Api(ApiCommand::LoadSession))
}

pub(super) fn access_teams(app: &mut App) -> Effects {
    let began = app
        .workspace
        .teams
        .teams
        .begin_access(app.now, &TEAMS_REFRESH);

    Effects::when(began, Effect::Api(ApiCommand::LoadTeams))
}

pub(super) fn force_teams(app: &mut App) -> Effects {
    if app.workspace.teams.teams.in_flight() {
        return Effects::default();
    }

    app.workspace.teams.teams.begin();

    Effects::one(Effect::Api(ApiCommand::LoadTeams))
}

pub(super) fn access_focused_panel(app: &mut App) -> Effects {
    match app.focus() {
        Focus::SavedViews => prefetch_selected_view(app),
        Focus::Teams => access_teams(app),
        Focus::MyWork | Focus::Recent | Focus::View(_) | Focus::Detail(..) => Effects::default(),
    }
}

pub(super) fn access_inbox(app: &mut App) -> Effects {
    let began = app.workspace.inbox.begin_access(app.now, &INBOX_REFRESH);

    Effects::when(
        began,
        Effect::Api(ApiCommand::LoadInboxFeed {
            request: FeedRequest::Refresh,
        }),
    )
}

pub(super) fn access_active(app: &mut App) -> Effects {
    match app.active_feed_key() {
        Some(key) => access_feed(app, key),
        None => access_inbox(app),
    }
}

pub(super) fn force_feed(app: &mut App, key: FeedKey) -> Effects {
    let feed = app.workspace.feeds.get_or_default(&key);
    if feed.in_flight() {
        return Effects::default();
    }

    feed.begin(&FeedRequest::Refresh);

    Effects::one(Effect::Api(ApiCommand::LoadFeed {
        key,
        request: FeedRequest::Refresh,
    }))
}

pub(super) fn force_active(app: &mut App) -> Effects {
    match app.active_feed_key() {
        Some(key) => force_feed(app, key),
        None => {
            if app.workspace.inbox.in_flight() {
                return Effects::default();
            }

            app.workspace.inbox.begin(&FeedRequest::Refresh);

            Effects::one(Effect::Api(ApiCommand::LoadInboxFeed {
                request: FeedRequest::Refresh,
            }))
        }
    }
}

pub(super) fn revalidate_focus(app: &mut App) -> Effects {
    let left = app.focus().left();

    match app.focus() {
        Focus::MyWork => access_active(app),
        Focus::View(_) => access_open_view(app),
        Focus::SavedViews => selected_view_key(app)
            .map(|key| access_feed(app, key))
            .unwrap_or_default(),
        Focus::Teams => access_teams(app),
        Focus::Recent => Effects::default(),
        Focus::Detail(..) => match left {
            LeftPanel::MyWork => access_active(app),
            LeftPanel::SavedViews => access_open_view(app),
            LeftPanel::Teams => access_focused_view(app),
            LeftPanel::Recent => Effects::default(),
        },
    }
}

fn access_focused_view(app: &mut App) -> Effects {
    match app.view().map(ViewSurface::key) {
        Some(key) => access_feed(app, key),
        None => Effects::default(),
    }
}

fn access_open_view(app: &mut App) -> Effects {
    match app.view().map(ViewSurface::key) {
        Some(key) => access_feed(app, key),
        None => selected_view_key(app)
            .map(|key| access_feed(app, key))
            .unwrap_or_default(),
    }
}

pub(super) fn reload(app: &mut App) -> Effects {
    let commands = reload_focus(app);

    if app.workspace.session.is_failed() {
        let mut all = access_session(app);
        all.extend(commands);

        return all;
    }

    commands
}

fn reload_focus(app: &mut App) -> Effects {
    match app.focus().clone() {
        Focus::Detail(detail) => {
            app.workspace.begin_detail();

            let mut commands = Effects::one(Effect::Api(ApiCommand::LoadDetail {
                target: detail.issue,
                reveal: Reveal::Top,
            }));

            let feed = match detail.origin.panel() {
                LeftPanel::SavedViews | LeftPanel::Teams => {
                    match app.view().map(ViewSurface::key) {
                        Some(key) => force_feed(app, key),
                        None => Effects::default(),
                    }
                }
                LeftPanel::MyWork => force_active(app),
                LeftPanel::Recent => Effects::default(),
            };

            commands.extend(feed);

            commands
        }
        Focus::SavedViews => {
            app.workspace.saved_views.views.begin();
            Effects::one(Effect::Api(ApiCommand::LoadCustomViews))
        }
        Focus::View(_) => match app.view().map(ViewSurface::key) {
            Some(key) => force_feed(app, key),
            None => force_active(app),
        },
        Focus::Teams => force_teams(app),
        Focus::MyWork | Focus::Recent => force_active(app),
    }
}

pub(super) fn feed_keep_id(app: &App, key: &FeedKey) -> Option<IssueId> {
    if app.active_feed_key().as_ref() == Some(key) {
        return app.selected_issue().map(|issue| issue.id.clone());
    }
    if app.view().map(ViewSurface::key).as_ref() == Some(key) {
        return app.view_selected_issue().map(|issue| issue.id.clone());
    }
    None
}

pub(super) fn reconcile_feed(app: &mut App, key: &FeedKey, keep: Option<IssueId>) {
    if app.active_feed_key().as_ref() == Some(key) {
        let idx = resolve(
            app.active_issues(),
            keep.as_ref(),
            app.ui.list_state.selected(),
        );
        app.ui.list_state.select(idx);
    }

    if app.view().map(ViewSurface::key).as_ref() == Some(key) {
        reselect_view(app, keep);
        return;
    }

    let len = app
        .workspace
        .feeds
        .get(key)
        .map_or(0, |feed| feed.items().len());
    if let Some(search) = app.search_mut() {
        if FeedKey::Search(search.query.clone()) == *key {
            clamp_selection(&mut search.state, len);
        }
    }
}

pub(super) fn resolve<T: HasId>(
    items: &[T],
    keep: Option<&T::Id>,
    current: Option<usize>,
) -> Option<usize> {
    if items.is_empty() {
        return Some(0);
    }
    if let Some(id) = keep {
        if let Some(pos) = items.iter().position(|item| item.feed_id() == id) {
            return Some(pos);
        }
    }
    Some(current.unwrap_or(0).min(items.len() - 1))
}

pub(super) fn prefetch_selected_view(app: &mut App) -> Effects {
    if !app.focus().is_panel(LeftPanel::SavedViews) {
        return Effects::default();
    }

    let Some(key) = selected_view_key(app) else {
        return Effects::default();
    };

    access_feed(app, key)
}

pub(super) fn take_load_more<T: HasId>(
    feed: &mut Feed<T>,
    selected: Option<usize>,
    len: usize,
) -> Option<FeedRequest> {
    if !feed.can_load_more() {
        return None;
    }
    if selected.is_none_or(|sel| sel + PREFETCH_MARGIN < len) {
        return None;
    }
    let after = feed.next().cloned()?;
    let request = FeedRequest::LoadMore { after };
    feed.begin(&request);
    Some(request)
}

pub(super) fn load_more(
    app: &mut App,
    key: &FeedKey,
    selected: Option<usize>,
    len: usize,
) -> Effects {
    let Some(request) = take_load_more(app.workspace.feeds.get_or_default(key), selected, len)
    else {
        return Effects::default();
    };

    Effects::one(Effect::Api(ApiCommand::LoadFeed {
        key: key.clone(),
        request,
    }))
}

pub(super) fn load_more_inbox(app: &mut App, selected: Option<usize>, len: usize) -> Effects {
    let Some(request) = take_load_more(&mut app.workspace.inbox, selected, len) else {
        return Effects::default();
    };

    Effects::one(Effect::Api(ApiCommand::LoadInboxFeed { request }))
}

pub(super) fn load_more_for_focus(app: &mut App) -> Effects {
    match app.focus() {
        Focus::MyWork => match app.active_feed_key() {
            Some(key) => {
                let len = app.active_issues().len();
                let selected = app.ui.list_state.selected();
                load_more(app, &key, selected, len)
            }
            None => {
                let len = app.workspace.inbox.items().len();
                let selected = app.ui.list_state.selected();
                load_more_inbox(app, selected, len)
            }
        },
        Focus::View(_) => {
            let Some(key) = app.view().map(ViewSurface::key) else {
                return Effects::default();
            };
            let len = app.view_len();
            let selected = app.view().and_then(|view| view.state.selected());
            load_more(app, &key, selected, len)
        }
        Focus::Recent | Focus::SavedViews | Focus::Teams | Focus::Detail(..) => Effects::default(),
    }
}
