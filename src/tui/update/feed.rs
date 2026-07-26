use super::nav::{clamp_selection, reselect_view};
use crate::tui::app::App;
use crate::tui::feed::{
    Feed, FeedKey, FeedRequest, HasId, FEED_REFRESH, INBOX_REFRESH, PREFETCH_MARGIN,
};
use crate::tui::focus::{Focus, LeftPanel, Reveal};
use crate::tui::message::Command;
use crate::tui::overlay::Overlay;
use crate::tui::saved_views::ViewSurface;

pub fn initial_commands(app: &mut App) -> Vec<Command> {
    app.workspace.saved_views.views.begin();

    let mut commands = vec![
        Command::LoadSession,
        Command::LoadRecent,
        Command::LoadCustomViews,
    ];

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

pub(super) fn save_feeds_command(app: &App) -> Command {
    Command::SaveFeeds(crate::store::build_cache(
        &app.workspace.feeds,
        &app.workspace.inbox,
        app.now,
    ))
}

pub(super) fn selected_view_key(app: &App) -> Option<FeedKey> {
    app.workspace
        .saved_views
        .selected_view()
        .map(|view| FeedKey::View(view.id.clone()))
}

pub(super) fn access_feed(app: &mut App, key: FeedKey) -> Option<Command> {
    app.workspace
        .feeds
        .get_or_default(&key)
        .begin_access(app.now, &FEED_REFRESH)
        .then_some(Command::LoadFeed {
            key,
            request: FeedRequest::Refresh,
        })
}

pub(super) fn access_inbox(app: &mut App) -> Option<Command> {
    app.workspace
        .inbox
        .begin_access(app.now, &INBOX_REFRESH)
        .then_some(Command::LoadInboxFeed {
            request: FeedRequest::Refresh,
        })
}

pub(super) fn access_active(app: &mut App) -> Option<Command> {
    match app.active_feed_key() {
        Some(key) => access_feed(app, key),
        None => access_inbox(app),
    }
}

pub(super) fn force_feed(app: &mut App, key: FeedKey) -> Command {
    app.workspace
        .feeds
        .get_or_default(&key)
        .begin(&FeedRequest::Refresh);

    Command::LoadFeed {
        key,
        request: FeedRequest::Refresh,
    }
}

pub(super) fn force_active(app: &mut App) -> Command {
    match app.active_feed_key() {
        Some(key) => force_feed(app, key),
        None => {
            app.workspace.inbox.begin(&FeedRequest::Refresh);
            Command::LoadInboxFeed {
                request: FeedRequest::Refresh,
            }
        }
    }
}

pub(super) fn revalidate_focus(app: &mut App) -> Option<Command> {
    match app.focus {
        Focus::MyWork | Focus::Detail(LeftPanel::MyWork, _) => access_active(app),
        Focus::View | Focus::Detail(LeftPanel::SavedViews, _) => {
            match app.workspace.view_open.as_ref().map(ViewSurface::key) {
                Some(key) => access_feed(app, key),
                None => selected_view_key(app).and_then(|key| access_feed(app, key)),
            }
        }
        Focus::SavedViews => selected_view_key(app).and_then(|key| access_feed(app, key)),
        Focus::Recent | Focus::Stub(_) | Focus::Detail(..) => None,
    }
}

pub(super) fn reload(app: &mut App) -> Command {
    match app.focus {
        Focus::Detail(panel, _) => match app.workspace.detail.value() {
            Some(detail) => {
                let id = detail.id.clone();
                app.workspace.detail.begin();
                let detail_reload = Command::LoadDetail {
                    id,
                    reveal: Reveal::Top,
                };

                match panel {
                    LeftPanel::SavedViews => {
                        match app.workspace.view_open.as_ref().map(ViewSurface::key) {
                            Some(key) => Command::Batch(vec![detail_reload, force_feed(app, key)]),
                            None => detail_reload,
                        }
                    }
                    LeftPanel::MyWork => Command::Batch(vec![detail_reload, force_active(app)]),
                    LeftPanel::Recent | LeftPanel::Stub(_) => detail_reload,
                }
            }
            None => force_active(app),
        },
        Focus::SavedViews => {
            app.workspace.saved_views.views.begin();
            Command::LoadCustomViews
        }
        Focus::View => match app.workspace.view_open.as_ref().map(ViewSurface::key) {
            Some(key) => force_feed(app, key),
            None => force_active(app),
        },
        Focus::MyWork | Focus::Recent | Focus::Stub(_) => force_active(app),
    }
}

pub(super) fn feed_keep_id(app: &App, key: &FeedKey) -> Option<String> {
    if app.active_feed_key().as_ref() == Some(key) {
        return app.selected_issue().map(|issue| issue.id.clone());
    }
    if app
        .workspace
        .view_open
        .as_ref()
        .map(ViewSurface::key)
        .as_ref()
        == Some(key)
    {
        return app.view_selected_issue().map(|issue| issue.id.clone());
    }
    None
}

pub(super) fn reconcile_feed(app: &mut App, key: &FeedKey, keep: Option<String>) {
    if app.active_feed_key().as_ref() == Some(key) {
        let idx = resolve(
            app.active_issues(),
            keep.as_deref(),
            app.list_state.selected(),
        );
        app.list_state.select(idx);
    }

    if app
        .workspace
        .view_open
        .as_ref()
        .map(ViewSurface::key)
        .as_ref()
        == Some(key)
    {
        reselect_view(app, keep);
        return;
    }

    let len = app
        .workspace
        .feeds
        .get(key)
        .map_or(0, |feed| feed.items().len());
    if let Overlay::Search(search) = &mut app.overlay {
        if FeedKey::Search(search.query.clone()) == *key {
            clamp_selection(&mut search.state, len);
        }
    }
}

pub(super) fn resolve<T: HasId>(
    items: &[T],
    keep: Option<&str>,
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

pub(super) fn prefetch_selected_view(app: &mut App) -> Option<Command> {
    if app.focus != Focus::SavedViews {
        return None;
    }

    let key = selected_view_key(app)?;
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
) -> Option<Command> {
    let request = take_load_more(app.workspace.feeds.get_or_default(key), selected, len)?;
    Some(Command::LoadFeed {
        key: key.clone(),
        request,
    })
}

pub(super) fn load_more_inbox(
    app: &mut App,
    selected: Option<usize>,
    len: usize,
) -> Option<Command> {
    let request = take_load_more(&mut app.workspace.inbox, selected, len)?;
    Some(Command::LoadInboxFeed { request })
}

pub(super) fn load_more_for_focus(app: &mut App) -> Option<Command> {
    match app.focus {
        Focus::MyWork => match app.active_feed_key() {
            Some(key) => {
                let len = app.active_issues().len();
                let selected = app.list_state.selected();
                load_more(app, &key, selected, len)
            }
            None => {
                let len = app.workspace.inbox.items().len();
                let selected = app.list_state.selected();
                load_more_inbox(app, selected, len)
            }
        },
        Focus::View => {
            let key = app.workspace.view_open.as_ref().map(ViewSurface::key)?;
            let len = app.view_len();
            let selected = app
                .workspace
                .view_open
                .as_ref()
                .and_then(|view| view.state.selected());
            load_more(app, &key, selected, len)
        }
        _ => None,
    }
}
