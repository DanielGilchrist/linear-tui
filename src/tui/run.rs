use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyEventKind};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc::{self, UnboundedSender};

use super::app::App;
use super::message::{Command, Message};
use super::overlay::PickerItem;
use super::platform::Platform;
use super::{render, update};
use crate::api::LinearApi;

pub async fn run(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    api: Arc<dyn LinearApi>,
) -> Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    let mut events = EventStream::new();
    let mut ticker = tokio::time::interval(Duration::from_millis(120));
    let platform = Platform::host();

    for command in update::initial_commands(app) {
        dispatch(&api, &tx, platform, command);
    }

    loop {
        terminal.draw(|frame| render::render(app, frame))?;

        if app.should_quit {
            break;
        }

        tokio::select! {
            maybe_event = events.next() => {
                if let Some(Ok(Event::Key(key))) = maybe_event {
                    if key.kind == KeyEventKind::Press {
                        if let Some(command) = update::handle_key(app, key) {
                            dispatch(&api, &tx, platform, command);
                        }
                    }
                }
            }
            Some(message) = rx.recv() => {
                if let Some(command) = update::apply(app, message) {
                    dispatch(&api, &tx, platform, command);
                }
            }
            _ = ticker.tick() => {
                app.now = super::app::now_epoch();
                if app.loading {
                    app.spinner.tick();
                }
            }
        }
    }

    Ok(())
}

fn dispatch(
    api: &Arc<dyn LinearApi>,
    tx: &UnboundedSender<Message>,
    platform: Platform,
    command: Command,
) {
    if let Command::Batch(commands) = command {
        for command in commands {
            dispatch(api, tx, platform, command);
        }
        return;
    }

    let api = Arc::clone(api);
    let tx = tx.clone();

    tokio::spawn(async move {
        let message: Option<Message> = match command {
            Command::Batch(_) => None,
            Command::LoadSession => Some(match api.session().await {
                Ok(session) => Message::SessionLoaded(session),
                Err(error) => Message::Failed(error.to_string()),
            }),
            Command::LoadIssues { view, filter } => Some(match api.issues(&filter).await {
                Ok(issues) => Message::IssuesLoaded { view, issues },
                Err(error) => Message::Failed(error.to_string()),
            }),
            Command::LoadInbox { view } => Some(match api.notifications().await {
                Ok(items) => Message::InboxLoaded { view, items },
                Err(error) => Message::Failed(error.to_string()),
            }),
            Command::LoadCustomViews => Some(match api.custom_views().await {
                Ok(views) => Message::CustomViewsLoaded(views),
                Err(error) => Message::CustomViewsFailed(error.to_string()),
            }),
            Command::LoadCustomViewIssues { id } => Some(match api.custom_view_issues(&id).await {
                Ok(page) => Message::CustomViewIssuesLoaded {
                    id,
                    issues: page.issues,
                    truncated: page.truncated,
                },
                Err(error) => Message::CustomViewIssuesFailed {
                    id,
                    error: error.to_string(),
                },
            }),
            Command::LoadDetail { id, reveal } => Some(match api.issue_detail(&id).await {
                Ok(Some(detail)) => Message::DetailLoaded {
                    detail: Box::new(detail),
                    reveal,
                },
                Ok(None) => Message::Failed(format!("Issue {id} not found")),
                Err(error) => Message::Failed(error.to_string()),
            }),
            Command::Search(term) => Some(match api.search_issues(&term).await {
                Ok(results) => Message::SearchResults(results),
                Err(error) => Message::Failed(error.to_string()),
            }),
            Command::LoadRecent => Some(Message::RecentLoaded(crate::store::load_recent())),
            Command::SaveRecent(issues) => {
                crate::store::save_recent(&issues);
                None
            }
            Command::ClearRecent => {
                crate::store::save_recent(&[]);
                Some(Message::RecentCleared)
            }
            Command::LoadStates { team_id } => Some(match api.workflow_states(&team_id).await {
                Ok(states) => {
                    Message::PickerLoaded(states.into_iter().map(PickerItem::from).collect())
                }
                Err(error) => Message::Failed(error.to_string()),
            }),
            Command::LoadMembers { team_id } => Some(match api.team_members(&team_id).await {
                Ok(members) => {
                    let mut items = vec![PickerItem::unassign()];
                    items.extend(members.into_iter().map(PickerItem::from));

                    Message::PickerLoaded(items)
                }
                Err(error) => Message::Failed(error.to_string()),
            }),
            Command::LoadMentionMembers { team_id } => {
                Some(match api.team_members(&team_id).await {
                    Ok(members) => Message::MentionMembersLoaded(members),
                    Err(error) => Message::Failed(error.to_string()),
                })
            }
            Command::UpdateIssue { id, update } => {
                Some(match api.update_issue(&id, update).await {
                    Ok(()) => Message::IssueUpdated { id },
                    Err(error) => Message::Failed(error.to_string()),
                })
            }
            Command::CreateComment {
                issue_id,
                body,
                parent_id,
            } => Some(
                match api
                    .create_comment(&issue_id, &body, parent_id.as_deref())
                    .await
                {
                    Ok(()) => Message::CommentPosted { id: issue_id },
                    Err(error) => Message::Failed(error.to_string()),
                },
            ),
            Command::UpdateComment {
                issue_id,
                comment_id,
                body,
            } => Some(match api.update_comment(&comment_id, &body).await {
                Ok(()) => Message::CommentEdited { id: issue_id },
                Err(error) => Message::Failed(error.to_string()),
            }),
            Command::DeleteComment {
                issue_id,
                comment_id,
            } => Some(match api.delete_comment(&comment_id).await {
                Ok(()) => Message::CommentDeleted { id: issue_id },
                Err(error) => Message::Failed(error.to_string()),
            }),
            Command::OpenUrl(url) => {
                match tokio::task::spawn_blocking(move || platform.open_url(&url)).await {
                    Ok(Ok(())) => None,
                    Ok(Err(error)) => Some(Message::Failed(error.to_string())),
                    Err(error) => Some(Message::Failed(error.to_string())),
                }
            }
            Command::CopyToClipboard(text) => {
                match tokio::task::spawn_blocking(move || platform.copy_to_clipboard(&text)).await {
                    Ok(Ok(())) => None,
                    Ok(Err(error)) => Some(Message::Failed(error.to_string())),
                    Err(error) => Some(Message::Failed(error.to_string())),
                }
            }
        };

        if let Some(message) = message {
            let _ = tx.send(message);
        }
    });
}
