use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{Event as CrosstermEvent, EventStream, KeyEventKind};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use super::app::App;
use super::event::{Event, Redraw};
use super::feed::FeedKey;
use super::message::{Command, FailureTarget, Message};
use super::platform::Platform;
use super::{render, update};
use crate::api::{LinearApi, Timestamp};

pub async fn run(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    api: Arc<dyn LinearApi>,
    namespace: String,
) -> Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    let mut events = EventStream::new();
    let mut ticker = tokio::time::interval(Duration::from_millis(120));
    let platform = Platform::host();

    platform.migrate_state_dir();

    if let Some(cache) = crate::store::load_feeds(&namespace) {
        update::restore_feeds(app, cache);
    }

    for command in update::initial_commands(app) {
        dispatch(&api, &tx, platform, &namespace, command);
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
                    dispatch(&api, &tx, platform, &namespace, command);
                }
            }
            Event::Message(message) => {
                if let Some(command) = update::apply(app, message) {
                    dispatch(&api, &tx, platform, &namespace, command);
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

async fn next_event(
    events: &mut EventStream,
    rx: &mut UnboundedReceiver<Message>,
    ticker: &mut tokio::time::Interval,
) -> Event {
    tokio::select! {
        polled = events.next() => classify(polled),
        Some(message) = rx.recv() => Event::Message(message),
        _ = ticker.tick() => Event::Tick(Timestamp::now()),
    }
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
    api: &Arc<dyn LinearApi>,
    tx: &UnboundedSender<Message>,
    platform: Platform,
    namespace: &str,
    command: Command,
) {
    if let Command::Batch(commands) = command {
        for command in commands {
            dispatch(api, tx, platform, namespace, command);
        }
        return;
    }

    let api = Arc::clone(api);
    let tx = tx.clone();
    let namespace = namespace.to_string();

    tokio::spawn(async move {
        let message: Option<Message> = match command {
            Command::Batch(_) => None,
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
            let _ = tx.send(message);
        }
    });
}
