use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{Event as CrosstermEvent, EventStream, KeyEventKind};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use super::app::App;
use super::event::{Event, Generation, Redraw};
use super::feed::FeedKey;
use super::message::{Command, FailureTarget, Message};
use super::platform::Platform;
use super::workspace::WorkspaceData;
use super::{render, update};
use crate::api::{Credential, LinearApi, Timestamp};
use crate::store::Account;

pub type ClientFactory = Arc<dyn Fn(Credential) -> Arc<dyn LinearApi> + Send + Sync>;

type Tx = UnboundedSender<(Generation, Message)>;

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
}

pub async fn run(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    bootstrap: Option<Credential>,
    make_client: ClientFactory,
) -> Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<(Generation, Message)>();
    let mut events = EventStream::new();
    let mut ticker = tokio::time::interval(Duration::from_millis(120));
    let platform = Platform::host();

    platform.migrate_state_dir();

    let loaded = crate::store::load_accounts();
    app.accounts = loaded.accounts.clone();

    let mut rt = Runtime {
        conn: startup_connection(app, &loaded, bootstrap, &make_client),
        generation: Generation::START,
        make_client,
        tx,
        platform,
    };

    match &rt.conn {
        Some(conn) => {
            if let Some(cache) = crate::store::load_feeds(&conn.namespace) {
                update::restore_feeds(app, cache);
            }
            for command in update::initial_commands(app) {
                dispatch(conn, rt.generation, &rt.tx, rt.platform, command);
            }
        }
        None => update::open_workspaces(app),
    }

    terminal.draw(|frame| render::render(app, frame))?;

    loop {
        match next_event(&mut events, &mut rx, &mut ticker).await {
            Event::Closed => break,
            Event::Ignored => continue,
            Event::Tick(now) if update::tick(app, now) == Redraw::Skipped => continue,
            Event::Tick(_) | Event::Resize => {}
            Event::Input(key) => {
                if let Some(command) = update::handle_key(app, key) {
                    run_command(&mut rt, app, command);
                }
            }
            Event::Message {
                generation,
                message,
            } => {
                if generation != rt.generation {
                    continue;
                }
                if let Some(command) = update::apply(app, message) {
                    run_command(&mut rt, app, command);
                }
            }
        }

        if app.should_quit {
            break;
        }

        terminal.draw(|frame| render::render(app, frame))?;
    }

    Ok(())
}

fn startup_connection(
    app: &mut App,
    loaded: &crate::store::Accounts,
    bootstrap: Option<Credential>,
    make_client: &ClientFactory,
) -> Option<Connection> {
    let active = loaded
        .active
        .as_deref()
        .and_then(|key| loaded.accounts.iter().find(|a| a.workspace_key == key))
        .or_else(|| loaded.accounts.first());

    if let Some(account) = active {
        app.active_workspace = Some(account.workspace_key.clone());
        return Some(Connection {
            api: make_client(account.credential.clone()),
            namespace: account.namespace(),
        });
    }

    bootstrap.map(|credential| Connection {
        namespace: crate::store::namespace(&credential.secret()),
        api: make_client(credential),
    })
}

async fn next_event(
    events: &mut EventStream,
    rx: &mut UnboundedReceiver<(Generation, Message)>,
    ticker: &mut tokio::time::Interval,
) -> Event {
    tokio::select! {
        polled = events.next() => classify(polled),
        Some((generation, message)) = rx.recv() => Event::Message { generation, message },
        _ = ticker.tick() => Event::Tick(Timestamp::now()),
    }
}

fn run_command(rt: &mut Runtime, app: &mut App, command: Command) {
    match command {
        Command::Batch(commands) => {
            for command in commands {
                run_command(rt, app, command);
            }
        }
        Command::SwitchWorkspace(account) => switch_workspace(rt, app, *account),
        Command::AddAccount { credential } => add_account(rt, credential),
        Command::BeginLogin => begin_login(rt),
        other => {
            if let Some(conn) = &rt.conn {
                dispatch(conn, rt.generation, &rt.tx, rt.platform, other);
            }
        }
    }
}

fn switch_workspace(rt: &mut Runtime, app: &mut App, account: Account) {
    if let Some(conn) = &rt.conn {
        let cache = crate::store::build_cache(&app.workspace.feeds, &app.workspace.inbox, app.now);
        crate::store::save_feeds(&conn.namespace, &cache);
    }

    rt.generation = rt.generation.next();
    rt.conn = Some(Connection {
        api: (rt.make_client)(account.credential.clone()),
        namespace: account.namespace(),
    });

    app.active_workspace = Some(account.workspace_key.clone());
    app.workspace = WorkspaceData::new();
    crate::store::save_accounts(&app.accounts, Some(&account.workspace_key));

    if let Some(conn) = &rt.conn {
        if let Some(cache) = crate::store::load_feeds(&conn.namespace) {
            update::restore_feeds(app, cache);
        }
        for command in update::initial_commands(app) {
            dispatch(conn, rt.generation, &rt.tx, rt.platform, command);
        }
    }
}

fn add_account(rt: &Runtime, credential: Credential) {
    let api = (rt.make_client)(credential.clone());
    let tx = rt.tx.clone();
    let generation = rt.generation;

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

        let _ = tx.send((generation, message));
    });
}

fn begin_login(rt: &Runtime) {
    let tx = rt.tx.clone();
    let generation = rt.generation;
    let platform = rt.platform;

    tokio::spawn(async move {
        let message = match crate::oauth::login(platform).await {
            Ok(credential) => Message::LoginSucceeded { credential },
            Err(error) => ephemeral_failure(error.to_string()),
        };

        let _ = tx.send((generation, message));
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

fn ephemeral_failure(error: String) -> Message {
    Message::Failed {
        target: FailureTarget::Ephemeral,
        error,
    }
}

fn dispatch(
    conn: &Connection,
    generation: Generation,
    tx: &Tx,
    platform: Platform,
    command: Command,
) {
    if let Command::Batch(commands) = command {
        for command in commands {
            dispatch(conn, generation, tx, platform, command);
        }
        return;
    }

    let api = Arc::clone(&conn.api);
    let tx = tx.clone();
    let namespace = conn.namespace.clone();

    tokio::spawn(async move {
        let message: Option<Message> = match command {
            Command::Batch(_) => None,
            Command::SwitchWorkspace(_) => None,
            Command::AddAccount { .. } => None,
            Command::BeginLogin => None,
            Command::LoadSession => Some(match api.session().await {
                Ok(session) => Message::SessionLoaded(session),
                Err(error) => Message::Failed {
                    target: FailureTarget::Ephemeral,
                    error: error.to_string(),
                },
            }),
            Command::LoadFeed { key, request } => {
                let after = request.cursor().cloned();
                let result = match &key {
                    FeedKey::Issues(filter) => api.issues(filter, after.as_ref()).await,
                    FeedKey::View(id) => api.custom_view_issues(id, after.as_ref()).await,
                    FeedKey::Search(term) => api.search_issues(term, after.as_ref()).await,
                };

                Some(match result {
                    Ok(page) => Message::FeedLoaded { key, request, page },
                    Err(error) => Message::Failed {
                        target: FailureTarget::Feed(key),
                        error: error.to_string(),
                    },
                })
            }
            Command::LoadInboxFeed { request } => {
                let after = request.cursor().cloned();

                Some(match api.notifications(after.as_ref()).await {
                    Ok(page) => Message::InboxLoaded { request, page },
                    Err(error) => Message::Failed {
                        target: FailureTarget::Inbox,
                        error: error.to_string(),
                    },
                })
            }
            Command::SaveFeeds(cache) => {
                crate::store::save_feeds(&namespace, &cache);
                None
            }
            Command::LoadCustomViews => Some(match api.custom_views().await {
                Ok(views) => Message::CustomViewsLoaded(views),
                Err(error) => Message::Failed {
                    target: FailureTarget::CustomViews,
                    error: error.to_string(),
                },
            }),
            Command::LoadDetail { target, reveal } => Some(match api.issue_detail(&target).await {
                Ok(Some(detail)) => Message::DetailLoaded {
                    detail: Box::new(detail),
                    reveal,
                },
                Ok(None) => Message::Failed {
                    target: FailureTarget::Detail,
                    error: format!("Issue {target} not found"),
                },
                Err(error) => Message::Failed {
                    target: FailureTarget::Detail,
                    error: error.to_string(),
                },
            }),
            Command::LoadRecent => {
                Some(Message::RecentLoaded(crate::store::load_recent(&namespace)))
            }
            Command::SaveRecent(issues) => {
                crate::store::save_recent(&namespace, &issues);
                None
            }
            Command::ClearRecent => {
                crate::store::save_recent(&namespace, &[]);
                Some(Message::RecentCleared)
            }
            Command::LoadStates { team_id } => Some(match api.workflow_states(&team_id).await {
                Ok(states) => Message::StatesLoaded { team_id, states },
                Err(error) => Message::Failed {
                    target: FailureTarget::States { team_id },
                    error: error.to_string(),
                },
            }),
            Command::SearchUsers { query } => Some(match api.search_users(&query).await {
                Ok(users) => Message::UsersFound { query, users },
                Err(error) => Message::Failed {
                    target: FailureTarget::UserSearch,
                    error: error.to_string(),
                },
            }),
            Command::SearchLabels { query } => Some(match api.search_labels(&query).await {
                Ok(labels) => Message::LabelsFound { query, labels },
                Err(error) => Message::Failed {
                    target: FailureTarget::LabelSearch,
                    error: error.to_string(),
                },
            }),
            Command::LoadMembers { team_id } => Some(match api.team_members(&team_id).await {
                Ok(members) => Message::MembersLoaded { team_id, members },
                Err(error) => Message::Failed {
                    target: FailureTarget::Members { team_id },
                    error: error.to_string(),
                },
            }),
            Command::UpdateIssue { id, update } => {
                Some(match api.update_issue(&id, update).await {
                    Ok(()) => Message::IssueUpdated { id },
                    Err(error) => ephemeral_failure(error.to_string()),
                })
            }
            Command::CreateComment {
                issue_id,
                body,
                parent_id,
            } => Some(
                match api
                    .create_comment(&issue_id, &body, parent_id.as_ref())
                    .await
                {
                    Ok(()) => Message::CommentPosted { id: issue_id },
                    Err(error) => ephemeral_failure(error.to_string()),
                },
            ),
            Command::UpdateComment {
                issue_id,
                comment_id,
                body,
            } => Some(match api.update_comment(&comment_id, &body).await {
                Ok(()) => Message::CommentEdited { id: issue_id },
                Err(error) => ephemeral_failure(error.to_string()),
            }),
            Command::DeleteComment {
                issue_id,
                comment_id,
            } => Some(match api.delete_comment(&comment_id).await {
                Ok(()) => Message::CommentDeleted { id: issue_id },
                Err(error) => ephemeral_failure(error.to_string()),
            }),
            Command::CreateReaction {
                issue_id,
                target,
                emoji,
            } => Some(match api.create_reaction(&target, &emoji).await {
                Ok(()) => Message::ReactionToggled { id: issue_id },
                Err(error) => ephemeral_failure(error.to_string()),
            }),
            Command::DeleteReaction {
                issue_id,
                reaction_id,
            } => Some(match api.delete_reaction(&reaction_id).await {
                Ok(()) => Message::ReactionToggled { id: issue_id },
                Err(error) => ephemeral_failure(error.to_string()),
            }),
            Command::OpenUrl(url) => {
                match tokio::task::spawn_blocking(move || platform.open_url(&url)).await {
                    Ok(Ok(())) => None,
                    Ok(Err(error)) => Some(ephemeral_failure(error.to_string())),
                    Err(error) => Some(ephemeral_failure(error.to_string())),
                }
            }
            Command::CopyToClipboard(text) => {
                match tokio::task::spawn_blocking(move || platform.copy_to_clipboard(&text)).await {
                    Ok(Ok(())) => None,
                    Ok(Err(error)) => Some(ephemeral_failure(error.to_string())),
                    Err(error) => Some(ephemeral_failure(error.to_string())),
                }
            }
        };

        if let Some(message) = message {
            let _ = tx.send((generation, message));
        }
    });
}
