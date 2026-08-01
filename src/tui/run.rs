use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{Event as CrosstermEvent, EventStream, KeyEventKind};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use super::app::App;
use super::event::{Event, Generation, Lane, Redraw};
use super::feed::FeedKey;
use super::message::{
    ApiCommand, Commands, Effect, FailureTarget, Message, PlatformCommand, RequestError,
    RuntimeCommand, StoreCommand,
};
use super::platform::Platform;
use super::{render, update};
use crate::api::{Credential, LinearApi, Timestamp};
use crate::store::{Account, StateDir};

pub type ClientFactory = Arc<dyn Fn(Credential) -> Arc<dyn LinearApi> + Send + Sync>;

type Tx = UnboundedSender<(Lane, Message)>;

struct Connection {
    api: Arc<dyn LinearApi>,
    namespace: String,
}

struct Runtime {
    conn: Option<Connection>,
    generation: Generation,
    make_client: ClientFactory,
    tx: Tx,
    platform: Platform,
    state: StateDir,
}

impl Runtime {
    fn lane(&self) -> Lane {
        Lane::Workspace(self.generation)
    }
}

pub async fn run(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    bootstrap: Option<Credential>,
    make_client: ClientFactory,
    state: StateDir,
) -> Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<(Lane, Message)>();
    let mut events = EventStream::new();
    let mut ticker = tokio::time::interval(Duration::from_millis(120));
    let platform = Platform::host();

    let loaded = crate::store::load_accounts(&state);
    app.session.set_accounts(loaded.accounts.clone());

    let conn = startup_connection(app, &loaded, &make_client);

    let mut rt = Runtime {
        conn,
        generation: Generation::START,
        make_client,
        tx,
        platform,
        state,
    };

    match (&rt.conn, bootstrap) {
        (Some(conn), _) => {
            if let Some(cache) = crate::store::load_feeds(&rt.state, &conn.namespace) {
                update::restore_feeds(app, cache);
            }

            for effect in update::initial_commands(app) {
                run_effect(&mut rt, effect);
            }
        }
        (None, Some(credential)) => {
            app.ui.status = Some(super::status::Status::ConnectingWorkspace);
            add_account(&rt, credential);
        }
        (None, None) => update::open_workspaces(app),
    }

    terminal.draw(|frame| render::render(app, frame))?;

    loop {
        match next_event(&mut events, &mut rx, &mut ticker).await {
            Event::Closed => break,
            Event::Ignored => continue,
            Event::Tick(now) => {
                let redraw = update::tick(app, now);

                let commands = app.maybe_refresh_token();
                if commands.is_empty() && redraw == Redraw::Skipped {
                    continue;
                }
                run_commands(&mut rt, app, commands);
            }
            Event::Resize => {}
            Event::Input(key) => {
                let commands = update::handle_key(app, key);
                run_commands(&mut rt, app, commands);
            }
            Event::Message { lane, message } => {
                if is_stale(lane, rt.generation) {
                    continue;
                }
                let commands = update::apply(app, message);
                run_commands(&mut rt, app, commands);
            }
        }

        if app.should_quit {
            break;
        }

        terminal.draw(|frame| render::render(app, frame))?;
    }

    Ok(())
}

fn is_stale(lane: Lane, generation: Generation) -> bool {
    match lane {
        Lane::Workspace(sent) => sent != generation,
        Lane::Host => false,
    }
}

fn startup_connection(
    app: &mut App,
    loaded: &crate::store::Accounts,
    make_client: &ClientFactory,
) -> Option<Connection> {
    let account = loaded
        .active
        .as_deref()
        .and_then(|key| loaded.accounts.iter().find(|a| a.workspace_key == key))
        .or_else(|| loaded.accounts.first())?;

    if !app.session.activate(&account.workspace_key) {
        return None;
    }

    Some(Connection {
        api: make_client(account.credential.clone()),
        namespace: account.namespace(),
    })
}

async fn next_event(
    events: &mut EventStream,
    rx: &mut UnboundedReceiver<(Lane, Message)>,
    ticker: &mut tokio::time::Interval,
) -> Event {
    tokio::select! {
        polled = events.next() => classify(polled),
        Some((lane, message)) = rx.recv() => Event::Message { lane, message },
        _ = ticker.tick() => Event::Tick(Timestamp::now()),
    }
}

fn run_commands(rt: &mut Runtime, app: &mut App, commands: Commands) {
    match commands {
        Commands::Runtime(command) => run_runtime(rt, app, command),
        Commands::Effects(effects) => {
            for effect in effects {
                run_effect(rt, effect);
            }
        }
    }
}

fn run_effect(rt: &mut Runtime, effect: Effect) {
    match effect {
        Effect::Api(command) => match &rt.conn {
            Some(conn) => dispatch_api(conn, rt.lane(), &rt.tx, command),
            None => settle_offline(rt, &command),
        },
        Effect::Store(command) => match &rt.conn {
            Some(conn) => dispatch_store(&rt.state, &conn.namespace, &rt.tx, rt.lane(), command),
            None => settle_disconnected_store(rt),
        },
        Effect::Platform(command) => dispatch_platform(rt.platform, &rt.tx, command),
    }
}

fn settle_disconnected_store(rt: &Runtime) {
    let _ = rt
        .tx
        .send((rt.lane(), ephemeral_failure("Not connected".to_string())));
}

fn run_runtime(rt: &mut Runtime, app: &mut App, command: RuntimeCommand) {
    match command {
        RuntimeCommand::SwitchWorkspace(account) => switch_workspace(rt, app, *account),
        RuntimeCommand::AddAccount { credential } => add_account(rt, credential),
        RuntimeCommand::RefreshToken { workspace_key } => refresh_token(rt, app, workspace_key),
        RuntimeCommand::Reconnect => reconnect(rt, app),
        RuntimeCommand::BeginLogin => begin_login(rt),
    }
}

fn settle_offline(rt: &Runtime, command: &ApiCommand) {
    let message = Message::Failed {
        target: command.failure_target(),
        error: RequestError::Other("Not connected".to_string()),
    };

    let _ = rt.tx.send((rt.lane(), message));
}

fn switch_workspace(rt: &mut Runtime, app: &mut App, account: Account) {
    if !app.session.activate(&account.workspace_key) {
        return;
    }

    if let Some(conn) = &rt.conn {
        let cache = crate::store::build_cache(&app.workspace.feeds, &app.workspace.inbox, app.now);
        crate::store::save_feeds(&rt.state, &conn.namespace, &cache);
    }

    rt.generation = rt.generation.next();
    rt.conn = Some(Connection {
        api: (rt.make_client)(account.credential.clone()),
        namespace: account.namespace(),
    });

    app.reset_workspace();

    crate::store::save_accounts(
        &rt.state,
        app.session.accounts(),
        Some(&account.workspace_key),
    );

    if let Some(conn) = &rt.conn {
        if let Some(cache) = crate::store::load_feeds(&rt.state, &conn.namespace) {
            update::restore_feeds(app, cache);
        }
    }

    for effect in update::initial_commands(app) {
        run_effect(rt, effect);
    }
}

fn add_account(rt: &Runtime, credential: Credential) {
    let api = (rt.make_client)(credential.clone());
    let tx = rt.tx.clone();

    tokio::spawn(async move {
        let message = match api.session().await {
            Ok(session) => Message::AccountAdded {
                account: Box::new(Account {
                    workspace_key: session.org_url_key,
                    org_name: session.org_name,
                    credential,
                }),
            },
            Err(error) => ephemeral_failure(error.to_string()),
        };

        let _ = tx.send((Lane::Host, message));
    });
}

fn refresh_token(rt: &Runtime, app: &App, workspace_key: String) {
    let Some(refresh_token) = app.session.refresh_token_for(&workspace_key) else {
        let _ = rt
            .tx
            .send((Lane::Host, Message::RefreshFailed { workspace_key }));

        return;
    };

    let tx = rt.tx.clone();

    tokio::spawn(async move {
        let message = match crate::oauth::refresh(&refresh_token).await {
            Ok(token) => Message::TokenRefreshed {
                workspace_key,
                credential: Credential::OAuth(token),
            },
            Err(_) => Message::RefreshFailed { workspace_key },
        };

        let _ = tx.send((Lane::Host, message));
    });
}

fn reconnect(rt: &mut Runtime, app: &mut App) {
    let Some(account) = app.session.active_account().cloned() else {
        return;
    };

    rt.generation = rt.generation.next();
    rt.conn = Some(Connection {
        api: (rt.make_client)(account.credential.clone()),
        namespace: account.namespace(),
    });

    crate::store::save_accounts(
        &rt.state,
        app.session.accounts(),
        app.session.active_workspace(),
    );

    if rt.conn.is_some() {
        for effect in update::reconnect(app) {
            run_effect(rt, effect);
        }
    }
}

fn begin_login(rt: &Runtime) {
    let tx = rt.tx.clone();
    let platform = rt.platform;

    tokio::spawn(async move {
        let message = match crate::oauth::login(platform).await {
            Ok(credential) => Message::LoginSucceeded { credential },
            Err(error) => ephemeral_failure(error.to_string()),
        };

        let _ = tx.send((Lane::Host, message));
    });
}

fn classify(polled: Option<std::io::Result<CrosstermEvent>>) -> Event {
    match polled {
        None => Event::Closed,
        Some(Ok(CrosstermEvent::Key(key))) if key.kind == KeyEventKind::Press => Event::Input(key),
        Some(Ok(CrosstermEvent::Resize(..))) => Event::Resize,
        Some(_) => Event::Ignored,
    }
}

fn failed(target: FailureTarget, error: &crate::api::ApiError) -> Message {
    Message::Failed {
        target,
        error: RequestError::from(error),
    }
}

fn ephemeral_failure(error: String) -> Message {
    Message::Failed {
        target: FailureTarget::Ephemeral,
        error: RequestError::Other(error),
    }
}

fn dispatch_api(conn: &Connection, lane: Lane, tx: &Tx, command: ApiCommand) {
    let api = Arc::clone(&conn.api);
    let tx = tx.clone();
    let on_failure = command.failure_target();

    tokio::spawn(async move {
        let message: Option<Message> = match command {
            ApiCommand::LoadSession => Some(match api.session().await {
                Ok(session) => Message::SessionLoaded(session),
                Err(error) => failed(on_failure, &error),
            }),
            ApiCommand::LoadFeed { key, request } => {
                let after = request.cursor().cloned();
                let result = match &key {
                    FeedKey::Issues(filter) => api.issues(filter, after.as_ref()).await,
                    FeedKey::View(id) => api.custom_view_issues(id, after.as_ref()).await,
                    FeedKey::Search(term) => api.search_issues(term, after.as_ref()).await,
                };

                Some(match result {
                    Ok(page) => Message::FeedLoaded { key, request, page },
                    Err(error) => failed(on_failure, &error),
                })
            }
            ApiCommand::LoadInboxFeed { request } => {
                let after = request.cursor().cloned();

                Some(match api.notifications(after.as_ref()).await {
                    Ok(page) => Message::InboxLoaded { request, page },
                    Err(error) => failed(on_failure, &error),
                })
            }
            ApiCommand::LoadCustomViews => Some(match api.custom_views().await {
                Ok(views) => Message::CustomViewsLoaded(views),
                Err(error) => failed(on_failure, &error),
            }),
            ApiCommand::LoadTeams => Some(match api.teams().await {
                Ok(teams) => Message::TeamsLoaded { teams },
                Err(error) => failed(on_failure, &error),
            }),
            ApiCommand::LoadDetail { target, reveal } => {
                Some(match api.issue_detail(&target).await {
                    Ok(Some(detail)) => Message::DetailLoaded {
                        detail: Box::new(detail),
                        reveal,
                    },
                    Ok(None) => Message::Failed {
                        target: on_failure,
                        error: RequestError::Other(format!("Issue {target} not found")),
                    },
                    Err(error) => failed(on_failure, &error),
                })
            }
            ApiCommand::LoadStates { team_id } => Some(match api.workflow_states(&team_id).await {
                Ok(states) => Message::StatesLoaded { team_id, states },
                Err(error) => failed(on_failure, &error),
            }),
            ApiCommand::SearchUsers { query } => Some(match api.search_users(&query).await {
                Ok(users) => Message::UsersFound { query, users },
                Err(error) => failed(on_failure, &error),
            }),
            ApiCommand::SearchLabels { query } => Some(match api.search_labels(&query).await {
                Ok(labels) => Message::LabelsFound { query, labels },
                Err(error) => failed(on_failure, &error),
            }),
            ApiCommand::LoadMembers { team_id } => Some(match api.team_members(&team_id).await {
                Ok(members) => Message::MembersLoaded { team_id, members },
                Err(error) => failed(on_failure, &error),
            }),
            ApiCommand::UpdateIssue { id, update } => {
                Some(match api.update_issue(&id, update).await {
                    Ok(()) => Message::IssueUpdated { id },
                    Err(error) => failed(on_failure, &error),
                })
            }
            ApiCommand::CreateComment {
                issue_id,
                body,
                parent_id,
                ..
            } => Some(
                match api
                    .create_comment(&issue_id, &body, parent_id.as_ref())
                    .await
                {
                    Ok(()) => Message::CommentPosted { id: issue_id },
                    Err(error) => failed(on_failure, &error),
                },
            ),
            ApiCommand::UpdateComment {
                issue_id,
                comment_id,
                body,
                ..
            } => Some(match api.update_comment(&comment_id, &body).await {
                Ok(()) => Message::CommentEdited { id: issue_id },
                Err(error) => failed(on_failure, &error),
            }),
            ApiCommand::DeleteComment {
                issue_id,
                comment_id,
            } => Some(match api.delete_comment(&comment_id).await {
                Ok(()) => Message::CommentDeleted { id: issue_id },
                Err(error) => failed(on_failure, &error),
            }),
            ApiCommand::CreateReaction {
                issue_id,
                target,
                emoji,
            } => Some(match api.create_reaction(&target, &emoji).await {
                Ok(()) => Message::ReactionToggled { id: issue_id },
                Err(error) => failed(on_failure, &error),
            }),
            ApiCommand::DeleteReaction {
                issue_id,
                reaction_id,
            } => Some(match api.delete_reaction(&reaction_id).await {
                Ok(()) => Message::ReactionToggled { id: issue_id },
                Err(error) => failed(on_failure, &error),
            }),
        };

        if let Some(message) = message {
            let _ = tx.send((lane, message));
        }
    });
}

fn dispatch_store(state: &StateDir, namespace: &str, tx: &Tx, lane: Lane, command: StoreCommand) {
    let tx = tx.clone();
    let state = state.clone();
    let namespace = namespace.to_string();

    tokio::spawn(async move {
        let message: Option<Message> = match command {
            StoreCommand::SaveFeeds(cache) => {
                crate::store::save_feeds(&state, &namespace, &cache);
                None
            }
            StoreCommand::LoadRecent => Some(Message::RecentLoaded(crate::store::load_recent(
                &state, &namespace,
            ))),
            StoreCommand::SaveRecent(issues) => {
                crate::store::save_recent(&state, &namespace, &issues);
                None
            }
            StoreCommand::ClearRecent => {
                crate::store::save_recent(&state, &namespace, &[]);
                Some(Message::RecentCleared)
            }
        };

        if let Some(message) = message {
            let _ = tx.send((lane, message));
        }
    });
}

fn dispatch_platform(platform: Platform, tx: &Tx, command: PlatformCommand) {
    let tx = tx.clone();

    tokio::spawn(async move {
        let outcome = match command {
            PlatformCommand::OpenUrl(url) => {
                tokio::task::spawn_blocking(move || platform.open_url(&url)).await
            }
            PlatformCommand::CopyToClipboard(text) => {
                tokio::task::spawn_blocking(move || platform.copy_to_clipboard(&text)).await
            }
        };

        let message = match outcome {
            Ok(Ok(())) => None,
            Ok(Err(error)) => Some(ephemeral_failure(error.to_string())),
            Err(error) => Some(ephemeral_failure(error.to_string())),
        };

        if let Some(message) = message {
            let _ = tx.send((Lane::Host, message));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::fixture::FixtureClient;
    use crate::api::Page;
    use crate::tui::feed::Feed;
    use tempfile::TempDir;

    fn offline_runtime() -> (Runtime, UnboundedReceiver<(Lane, Message)>, TempDir) {
        runtime_with(Platform::inert())
    }

    fn runtime_with(platform: Platform) -> (Runtime, UnboundedReceiver<(Lane, Message)>, TempDir) {
        let dir = tempfile::tempdir().expect("a temporary state directory");
        let (tx, rx) = mpsc::unbounded_channel();
        let make_client: ClientFactory =
            Arc::new(|_| Arc::new(FixtureClient::sample()) as Arc<dyn LinearApi>);

        let rt = Runtime {
            conn: None,
            generation: Generation::START,
            make_client,
            tx,
            platform,
            state: StateDir::at(dir.path().into()),
        };

        (rt, rx, dir)
    }

    fn connected_runtime(
        namespace: &str,
    ) -> (Runtime, UnboundedReceiver<(Lane, Message)>, TempDir) {
        let (mut rt, rx, dir) = offline_runtime();

        rt.conn = Some(Connection {
            api: Arc::new(FixtureClient::sample()),
            namespace: namespace.to_string(),
        });

        (rt, rx, dir)
    }

    fn account(workspace_key: &str) -> Account {
        Account {
            workspace_key: workspace_key.to_string(),
            org_name: workspace_key.to_string(),
            credential: Credential::PersonalKey("k".into()),
        }
    }

    fn issue(id: &str) -> crate::api::IssueSummary {
        crate::api::IssueSummary {
            id: crate::api::IssueId::from_raw(id),
            identifier: id.to_string(),
            title: None,
            state: crate::api::WorkflowState {
                name: "Todo".into(),
                state_type: crate::api::StateType::Unstarted,
            },
            priority: crate::api::Priority::None,
            assignee: None,
            labels: Vec::new(),
            url: String::new(),
            branch_name: String::new(),
            team_id: crate::api::TeamId::from_raw("t"),
            updated_at: Timestamp::from_epoch(1_000),
        }
    }

    fn feed_reply(key: &FeedKey, id: &str) -> Message {
        Message::FeedLoaded {
            key: key.clone(),
            request: crate::tui::feed::FeedRequest::Refresh,
            page: crate::api::Page::single(vec![issue(id)]),
        }
    }

    fn deliver(rt: &mut Runtime, app: &mut App, lane: Lane, message: Message) {
        if is_stale(lane, rt.generation) {
            return;
        }

        let commands = update::apply(app, message);
        run_commands(rt, app, commands);
    }

    #[test]
    fn an_offline_load_settles_the_cell_instead_of_vanishing() {
        let (mut rt, mut rx, _dir) = offline_runtime();

        run_effect(&mut rt, Effect::Api(ApiCommand::LoadCustomViews));

        match rx.try_recv() {
            Ok((_, Message::Failed { target, .. })) => {
                assert!(matches!(target, FailureTarget::CustomViews));
            }
            other => panic!("expected an offline failure that settles the cell, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_platform_effect_runs_without_a_connection() {
        let (mut rt, mut rx, _dir) = offline_runtime();

        run_effect(
            &mut rt,
            Effect::Platform(PlatformCommand::OpenUrl("https://example.com".into())),
        );

        tokio::time::sleep(Duration::from_millis(50)).await;

        assert!(
            rx.try_recv().is_err(),
            "a platform effect dispatches instead of settling as not connected"
        );
    }

    #[tokio::test]
    async fn an_account_added_after_a_generation_bump_still_lands() {
        let (mut rt, mut rx, _dir) = offline_runtime();

        add_account(&rt, Credential::PersonalKey("k".into()));
        rt.generation = rt.generation.next();

        let (lane, message) = rx.recv().await.expect("an account reply");

        assert!(matches!(message, Message::AccountAdded { .. }));
        assert!(
            !is_stale(lane, rt.generation),
            "an account fact is not connection-scoped, so a bump cannot drop it"
        );
    }

    #[tokio::test]
    async fn a_platform_failure_lands_after_a_generation_bump() {
        let (mut rt, mut rx, _dir) = runtime_with(Platform::broken());

        run_effect(
            &mut rt,
            Effect::Platform(PlatformCommand::CopyToClipboard("DAN2-7".into())),
        );
        rt.generation = rt.generation.next();

        let (lane, message) = rx.recv().await.expect("a platform failure");

        assert!(matches!(
            message,
            Message::Failed {
                target: FailureTarget::Ephemeral,
                ..
            }
        ));
        assert!(
            !is_stale(lane, rt.generation),
            "a host fact is invalidated by no retargeting, so a bump cannot drop it"
        );
    }

    #[test]
    fn a_workspace_reply_from_before_a_switch_is_dropped() {
        let (mut rt, _rx, _dir) = offline_runtime();
        let mut app = App::new();
        let key = app.active_feed_key().expect("an active feed");
        let before_switch = rt.lane();

        deliver(&mut rt, &mut app, before_switch, feed_reply(&key, "i1"));
        assert_eq!(
            app.workspace.feeds.get(&key).map(|feed| feed.items().len()),
            Some(1),
            "a reply on the live lane lands"
        );

        rt.generation = rt.generation.next();
        app.reset_workspace();

        assert!(is_stale(
            Lane::Workspace(Generation::START),
            Generation::START.next()
        ));

        deliver(&mut rt, &mut app, before_switch, feed_reply(&key, "i2"));

        assert!(
            app.workspace.feeds.get(&key).is_none(),
            "a reply for the superseded connection must not write the new workspace"
        );
    }

    #[tokio::test]
    async fn reconnect_bumps_the_generation_so_pre_reconnect_replies_drop() {
        let (mut rt, _rx, _dir) = offline_runtime();
        let mut app = App::new();

        app.session.upsert_account(account("ws"));
        assert!(app.session.activate("ws"));

        let before_reconnect = rt.lane();
        assert!(!is_stale(before_reconnect, rt.generation));

        reconnect(&mut rt, &mut app);

        assert!(
            is_stale(before_reconnect, rt.generation),
            "reconnect retargets the connection, so replies for the old one must drop"
        );
    }

    #[tokio::test]
    async fn switching_workspaces_clears_the_previous_workspace_state() {
        let outgoing = account("a");
        let (mut rt, _rx, _dir) = connected_runtime(&outgoing.namespace());
        let mut app = App::new();

        app.session.upsert_account(outgoing);
        app.session.upsert_account(account("b"));
        assert!(app.session.activate("a"));

        let key = app.active_feed_key().expect("an active feed");
        app.workspace.feeds.insert(
            key.clone(),
            Feed::ready(Page::single(vec![issue("i1")]), app.now),
        );
        app.workspace.recently_viewed.push(issue("i1"));

        switch_workspace(&mut rt, &mut app, account("b"));

        assert!(app.workspace.recently_viewed.is_empty());
        assert!(
            app.workspace
                .feeds
                .get(&key)
                .is_none_or(|feed| feed.items().is_empty()),
            "the previous workspace's rows must not survive the switch"
        );
        assert_eq!(app.session.active_workspace(), Some("b"));
    }

    #[tokio::test]
    async fn switching_workspaces_saves_the_outgoing_feed_cache() {
        let outgoing = account("outgoing");
        let namespace = outgoing.namespace();
        let (mut rt, _rx, _dir) = connected_runtime(&namespace);
        let mut app = App::new();

        app.session.upsert_account(outgoing);
        app.session.upsert_account(account("incoming"));
        assert!(app.session.activate("outgoing"));

        let key = app.active_feed_key().expect("an active feed");
        app.workspace
            .feeds
            .insert(key, Feed::ready(Page::single(vec![issue("i1")]), app.now));

        switch_workspace(&mut rt, &mut app, account("incoming"));

        let saved =
            crate::store::load_feeds(&rt.state, &namespace).expect("the outgoing cache is written");
        assert_eq!(saved.issues.len(), 1);
    }

    #[test]
    fn switching_to_an_unknown_workspace_is_a_no_op() {
        let (mut rt, _rx, _dir) = offline_runtime();
        let mut app = App::new();

        app.session.upsert_account(Account {
            workspace_key: "ws".into(),
            org_name: "Known".into(),
            credential: Credential::PersonalKey("k".into()),
        });
        assert!(app.session.activate("ws"));

        let unknown = Account {
            workspace_key: "nope".into(),
            org_name: "Unknown".into(),
            credential: Credential::PersonalKey("k".into()),
        };

        switch_workspace(&mut rt, &mut app, unknown);

        assert_eq!(rt.generation, Generation::START);
        assert!(rt.conn.is_none());
        assert_eq!(app.session.active_workspace(), Some("ws"));
    }

    #[test]
    fn an_offline_store_effect_settles_as_a_failure() {
        let (mut rt, mut rx, _dir) = offline_runtime();

        run_effect(&mut rt, Effect::Store(StoreCommand::ClearRecent));

        match rx.try_recv() {
            Ok((_, Message::Failed { target, .. })) => {
                assert!(matches!(target, FailureTarget::Ephemeral));
            }
            other => panic!("expected an offline store effect to settle, got {other:?}"),
        }
    }
}
