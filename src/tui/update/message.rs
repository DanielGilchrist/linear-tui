use super::feed::{
    access_feed, feed_keep_id, reconcile_feed, resolve, revalidate_focus, selected_view_key,
};
use super::issue::{
    fill_picker, found_users, newest_comment_index, open_editor, place_editor, status_items,
    stop_assign_picker,
};
use super::nav::clamp_selection;
use crate::api::{
    Credential, IssueDetail, IssueSummary, Label, NotificationItem, Page, Session, StateOption,
    TeamId, User,
};
use crate::store::Account;
use crate::tui::app::{App, AuthState};
use crate::tui::cache::Stale;
use crate::tui::feed::{FeedKey, FeedRequest};
use crate::tui::focus::{Cursor, DetailView, Focus, LeftPanel, Reveal, Scroll};
use crate::tui::message::{
    ApiCommand, Commands, ComposeRecovery, Effect, Effects, FailureTarget, Message, RequestError,
    RuntimeCommand, StoreCommand,
};
use crate::tui::overlay::{LabelResults, Overlay, PickerKind};
use crate::tui::status::Status;
use crate::tui::view::ViewKind;

enum Transition {
    SessionLoaded(Session),
    FeedApplied {
        key: FeedKey,
        request: FeedRequest,
        page: Page<IssueSummary>,
        keep: Option<crate::api::IssueId>,
    },
    InboxApplied {
        request: FeedRequest,
        page: Page<NotificationItem>,
        active: bool,
        keep: Option<String>,
    },
    CustomViewsLoaded(Vec<crate::api::SavedView>),
    TeamsLoaded(Vec<crate::api::Team>),
    DetailLoaded {
        detail: Box<IssueDetail>,
        reveal: Reveal,
        focused: bool,
        settle: bool,
    },
    RecentLoaded(Vec<IssueSummary>),
    RecentCleared {
        leave_panel: bool,
    },
    StatesLoaded {
        team_id: TeamId,
        states: Vec<StateOption>,
    },
    MembersLoaded {
        team_id: TeamId,
        members: Vec<User>,
    },
    UsersFound {
        query: String,
        users: Vec<User>,
    },
    LabelsFound {
        query: String,
        labels: Vec<Label>,
    },
    IssueUpdated {
        id: crate::api::IssueId,
        on_detail: bool,
    },
    ReloadDetail {
        id: crate::api::IssueId,
        reveal: Reveal,
        status: Option<Status>,
        on_detail: bool,
    },
    AccountAdded(Box<Account>),
    LoginSucceeded(Credential),
    TokenRefreshed {
        workspace_key: String,
        credential: Credential,
    },
    RefreshFailed {
        workspace_key: String,
    },
    Failed {
        target: FailureTarget,
        error: RequestError,
    },
}

pub fn apply(app: &mut App, msg: Message) -> Commands {
    let transition = reduce(app, msg);
    commit(app, transition)
}

fn reduce(app: &App, msg: Message) -> Transition {
    match msg {
        Message::SessionLoaded(session) => Transition::SessionLoaded(session),
        Message::FeedLoaded { key, request, page } => {
            let keep = feed_keep_id(app, &key);
            Transition::FeedApplied {
                key,
                request,
                page,
                keep,
            }
        }
        Message::InboxLoaded { request, page } => {
            let active = matches!(app.active_view().kind, ViewKind::Inbox);
            let keep = active
                .then(|| app.selected_notification().map(|n| n.grouping_key.clone()))
                .flatten();
            Transition::InboxApplied {
                request,
                page,
                active,
                keep,
            }
        }
        Message::CustomViewsLoaded(views) => Transition::CustomViewsLoaded(views),
        Message::TeamsLoaded { teams } => Transition::TeamsLoaded(teams),
        Message::DetailLoaded { detail, reveal } => {
            let focused = app
                .focus()
                .detail()
                .is_some_and(|focus| focus.issue.matches_detail(&detail));
            let revalidates = app
                .workspace
                .detail()
                .value()
                .is_some_and(|current| current.id == detail.id);

            Transition::DetailLoaded {
                detail,
                reveal,
                focused,
                settle: focused || revalidates,
            }
        }
        Message::RecentLoaded(issues) => Transition::RecentLoaded(issues),
        Message::RecentCleared => Transition::RecentCleared {
            leave_panel: app.focus().left() == LeftPanel::Recent,
        },
        Message::StatesLoaded { team_id, states } => Transition::StatesLoaded { team_id, states },
        Message::MembersLoaded { team_id, members } => {
            Transition::MembersLoaded { team_id, members }
        }
        Message::UsersFound { query, users } => Transition::UsersFound { query, users },
        Message::LabelsFound { query, labels } => Transition::LabelsFound { query, labels },
        Message::IssueUpdated { id } => Transition::IssueUpdated {
            on_detail: focused_on_issue(app, &id),
            id,
        },
        Message::CommentPosted { id } => {
            let reveal = match app.focus() {
                Focus::Detail(detail) if detail.view.is_comments() => Reveal::NewestComment,
                Focus::Detail(_)
                | Focus::MyWork
                | Focus::Recent
                | Focus::SavedViews
                | Focus::Teams
                | Focus::View(_) => Reveal::Bottom,
            };
            Transition::ReloadDetail {
                on_detail: focused_on_issue(app, &id),
                id,
                reveal,
                status: Some(Status::CommentPosted),
            }
        }
        Message::CommentEdited { id } => Transition::ReloadDetail {
            on_detail: focused_on_issue(app, &id),
            id,
            reveal: Reveal::Top,
            status: Some(Status::CommentEdited),
        },
        Message::CommentDeleted { id } => Transition::ReloadDetail {
            on_detail: focused_on_issue(app, &id),
            id,
            reveal: Reveal::Top,
            status: Some(Status::CommentDeleted),
        },
        Message::ReactionToggled { id } => Transition::ReloadDetail {
            on_detail: focused_on_issue(app, &id),
            id,
            reveal: Reveal::Keep,
            status: None,
        },
        Message::AccountAdded { account } => Transition::AccountAdded(account),
        Message::LoginSucceeded { credential } => Transition::LoginSucceeded(credential),
        Message::TokenRefreshed {
            workspace_key,
            credential,
        } => Transition::TokenRefreshed {
            workspace_key,
            credential,
        },
        Message::RefreshFailed { workspace_key } => Transition::RefreshFailed { workspace_key },
        Message::Failed { target, error } => Transition::Failed { target, error },
    }
}

fn focused_on_issue(app: &App, id: &crate::api::IssueId) -> bool {
    app.focus()
        .detail()
        .is_some_and(|detail| detail.issue.matches_id(id))
}

fn commit(app: &mut App, transition: Transition) -> Commands {
    match transition {
        Transition::SessionLoaded(session) => {
            app.workspace.session.set(session, app.now);
            app.session.authenticated();
            Commands::default()
        }
        Transition::FeedApplied {
            key,
            request,
            page,
            keep,
        } => {
            if !app.apply_feed(&key, &request, page) {
                return Commands::default();
            }

            app.clear_transient_status();
            reconcile_feed(app, &key, keep);

            match request {
                FeedRequest::Refresh => Commands::from(Effect::Store(StoreCommand::SaveFeeds(
                    app.persisted_cache(),
                ))),
                FeedRequest::LoadMore { .. } => Commands::default(),
            }
        }
        Transition::InboxApplied {
            request,
            page,
            active,
            keep,
        } => {
            if !app.apply_inbox(&request, page) {
                return Commands::default();
            }

            app.clear_transient_status();

            if active {
                let idx = resolve(
                    app.workspace.inbox.items(),
                    keep.as_ref(),
                    app.ui.list_state.selected(),
                );
                app.ui.list_state.select(idx);
            }

            match request {
                FeedRequest::Refresh => Commands::from(Effect::Store(StoreCommand::SaveFeeds(
                    app.persisted_cache(),
                ))),
                FeedRequest::LoadMore { .. } => Commands::default(),
            }
        }
        Transition::CustomViewsLoaded(views) => {
            app.workspace.saved_views.views.set(views, app.now);

            let len = app.workspace.saved_views.list().len();
            clamp_selection(&mut app.workspace.saved_views.state, len);

            selected_view_key(app)
                .map(|key| access_feed(app, key))
                .unwrap_or_default()
                .into()
        }
        Transition::TeamsLoaded(teams) => {
            app.workspace.teams.teams.set(teams, app.now);

            let len = app.workspace.teams.list().len();
            clamp_selection(&mut app.workspace.teams.state, len);

            Commands::default()
        }
        Transition::DetailLoaded {
            detail,
            reveal,
            focused,
            settle,
        } => commit_detail(app, *detail, reveal, focused, settle),
        Transition::RecentLoaded(issues) => {
            app.merge_recent(issues);

            let len = app.workspace.recently_viewed.len();
            clamp_selection(&mut app.workspace.recent_state, len);

            Commands::default()
        }
        Transition::RecentCleared { leave_panel } => {
            app.workspace.recently_viewed.clear();
            app.workspace.recent_state.select(Some(0));
            app.ui.status = Some(Status::RecentCleared);

            if leave_panel {
                app.focus_my_work();
            }
            Commands::default()
        }
        Transition::StatesLoaded { team_id, states } => {
            let items = status_items(&states);

            app.workspace
                .states
                .get_or_default(&team_id)
                .set(states, app.now);

            if let Some(picker) = app.picker_mut() {
                if picker.kind == PickerKind::Status && picker.target_team == team_id {
                    fill_picker(picker, items);
                }
            }
            Commands::default()
        }
        Transition::MembersLoaded { team_id, members } => {
            app.workspace
                .members
                .get_or_default(&team_id)
                .set(members.clone(), app.now);

            if let Some(editor) = app.editor_mut() {
                if editor.target_team == team_id {
                    editor.set_members(members);
                }
            }
            Commands::default()
        }
        Transition::UsersFound { query, users } => {
            if let Some(picker) = app.picker_mut() {
                if picker.searching() == Some(query.as_str()) {
                    fill_picker(picker, found_users(users));
                    picker.settle_search();
                }
            }
            Commands::default()
        }
        Transition::LabelsFound { query, labels } => {
            if let Some(overlay) = app.labels_mut() {
                if overlay.query == query {
                    overlay.results = LabelResults::Loaded(labels);

                    let len = overlay.results().len();
                    clamp_selection(&mut overlay.state, len);
                }
            }
            Commands::default()
        }
        Transition::IssueUpdated { id, on_detail } => {
            app.ui.status = Some(Status::IssueUpdated);
            app.workspace.feeds.invalidate_all();
            app.workspace.inbox.mark_stale();
            let mut refresh = revalidate_focus(app);

            if on_detail {
                app.workspace.begin_detail();

                refresh.push(Effect::Api(ApiCommand::LoadDetail {
                    target: id.into(),
                    reveal: Reveal::Top,
                }));
            }

            refresh.into()
        }
        Transition::ReloadDetail {
            id,
            reveal,
            status,
            on_detail,
        } => {
            if let Some(status) = status {
                app.ui.status = Some(status);
            }

            if !on_detail {
                return Commands::default();
            }

            app.workspace.begin_detail();

            Commands::from(Effect::Api(ApiCommand::LoadDetail {
                target: id.into(),
                reveal,
            }))
        }
        Transition::AccountAdded(account) => {
            let account = *account;
            app.session.upsert_account(account.clone());
            Commands::runtime(RuntimeCommand::SwitchWorkspace(Box::new(account)))
        }
        Transition::LoginSucceeded(credential) => {
            app.ui.status = Some(Status::ConnectingWorkspace);
            Commands::runtime(RuntimeCommand::AddAccount { credential })
        }
        Transition::TokenRefreshed {
            workspace_key,
            credential,
        } => {
            app.session.set_credential(&workspace_key, credential);

            if !is_active_workspace(app, &workspace_key) {
                return Commands::default();
            }

            app.session.authenticated();

            Commands::runtime(RuntimeCommand::Reconnect)
        }
        Transition::RefreshFailed { workspace_key } => {
            if is_active_workspace(app, &workspace_key) {
                app.session.expired();
            }

            Commands::default()
        }
        Transition::Failed { target, error } => commit_failure(app, target, error),
    }
}

fn is_active_workspace(app: &App, workspace_key: &str) -> bool {
    app.session.active_workspace() == Some(workspace_key)
}

fn commit_detail(
    app: &mut App,
    detail: IssueDetail,
    reveal: Reveal,
    focused: bool,
    settle: bool,
) -> Commands {
    if !settle {
        return Commands::default();
    }

    let id = detail.id.clone();

    app.workspace.set_detail(detail, app.now);
    app.clear_transient_status();

    if !focused {
        return Commands::default();
    }

    app.refocus_detail_issue(id.into());

    let Some(detail) = app.workspace.detail().value() else {
        return Commands::default();
    };
    let summary = IssueSummary::from_detail(detail);
    let len = detail.thread_len();
    let newest = match reveal {
        Reveal::NewestComment => newest_comment_index(detail),
        Reveal::Keep | Reveal::Top | Reveal::Bottom => None,
    };

    if let Some(view) = app.focus().detail().map(|focus| focus.view) {
        app.set_detail_view(revealed_view(view, reveal, len, newest));
    }

    app.record_recent(summary);

    Commands::from(Effect::Store(StoreCommand::SaveRecent(
        app.workspace.recently_viewed.clone(),
    )))
}

fn revealed_view(
    view: DetailView,
    reveal: Reveal,
    len: usize,
    newest: Option<usize>,
) -> DetailView {
    match view {
        DetailView::Reading { scroll } => DetailView::Reading {
            scroll: match reveal {
                Reveal::Keep => scroll,
                Reveal::Top | Reveal::NewestComment => Scroll::Top,
                Reveal::Bottom => Scroll::Bottom,
            },
        },
        DetailView::Comments { at } if len > 0 => {
            let index = newest.unwrap_or(at.index()).min(len - 1);

            DetailView::Comments {
                at: Cursor::new(index, len).unwrap_or(at),
            }
        }
        DetailView::Comments { .. } => DetailView::reading(),
    }
}

fn commit_failure(app: &mut App, target: FailureTarget, error: RequestError) -> Commands {
    let (error, command) = match error {
        RequestError::Unauthorised(message) => (message, reauthenticate(app)),
        RequestError::Other(message) => (message, Commands::default()),
    };

    match target {
        FailureTarget::Session => app.workspace.session.fail(error.clone()),
        FailureTarget::Feed(key) => app.workspace.feeds.get_or_default(&key).fail(error.clone()),
        FailureTarget::Inbox => app.workspace.inbox.fail(error.clone()),
        FailureTarget::CustomViews => app.workspace.saved_views.views.fail(error.clone()),
        FailureTarget::Teams => app.workspace.teams.teams.fail(error.clone()),
        FailureTarget::Detail => app.workspace.fail_detail(error.clone()),
        FailureTarget::States { team_id } => {
            app.workspace
                .states
                .get_or_default(&team_id)
                .fail(error.clone());
        }
        FailureTarget::Members { team_id } => {
            app.workspace
                .members
                .get_or_default(&team_id)
                .fail(error.clone());
        }
        FailureTarget::UserSearch => stop_assign_picker(app.picker_mut()),
        FailureTarget::LabelSearch => {
            if let Some(overlay) = app.labels_mut() {
                overlay.results = LabelResults::Loaded(Vec::new());
            }
        }
        FailureTarget::Compose(recovery) => {
            app.ui.status = Some(Status::Error(error));

            if command.is_empty() {
                return recover_compose(app, *recovery).into();
            }

            reopen_compose_cached(app, *recovery);

            return command;
        }
        FailureTarget::Ephemeral => {}
    }

    app.ui.status = Some(Status::Error(error));

    command
}

fn recoverable(app: &App, recovery: ComposeRecovery) -> Option<ComposeRecovery> {
    matches!(app.overlay(), Overlay::None).then_some(recovery)
}

fn recover_compose(app: &mut App, recovery: ComposeRecovery) -> Effects {
    let Some(recovery) = recoverable(app, recovery) else {
        return Effects::default();
    };

    let ComposeRecovery {
        issue_id,
        team_id,
        compose,
        body,
    } = recovery;

    open_editor(app, issue_id, compose, team_id, Some(&body))
}

fn reopen_compose_cached(app: &mut App, recovery: ComposeRecovery) {
    let Some(recovery) = recoverable(app, recovery) else {
        return;
    };

    let ComposeRecovery {
        issue_id,
        team_id,
        compose,
        body,
    } = recovery;

    place_editor(app, issue_id, compose, &team_id, Some(&body));
}

fn refreshable(app: &App) -> Option<String> {
    app.session.active_refresh_token()?;

    app.session.active_workspace().map(str::to_string)
}

fn reauthenticate(app: &mut App) -> Commands {
    match app.session.auth() {
        AuthState::Authenticated => match refreshable(app) {
            Some(workspace_key) => {
                app.session.begin_refresh(app.now);
                Commands::runtime(RuntimeCommand::RefreshToken { workspace_key })
            }
            None => {
                app.session.expired();
                Commands::default()
            }
        },
        AuthState::Refreshing { .. } | AuthState::Unauthenticated => Commands::default(),
    }
}
