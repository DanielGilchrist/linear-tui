use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyEventKind};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc::{self, UnboundedSender};

use super::app::App;
use super::message::{Command, Message};
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

    for command in update::initial_commands(app) {
        dispatch(&api, &tx, command);
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
                        for command in update::handle_key(app, key) {
                            dispatch(&api, &tx, command);
                        }
                    }
                }
            }
            Some(message) = rx.recv() => {
                for command in update::apply(app, message) {
                    dispatch(&api, &tx, command);
                }
            }
            _ = ticker.tick() => {
                if app.loading {
                    app.spinner_frame = app.spinner_frame.wrapping_add(1);
                }
            }
        }
    }

    Ok(())
}

fn dispatch(api: &Arc<dyn LinearApi>, tx: &UnboundedSender<Message>, command: Command) {
    let api = Arc::clone(api);
    let tx = tx.clone();

    tokio::spawn(async move {
        let message = match command {
            Command::LoadSession => match api.session().await {
                Ok(session) => Message::SessionLoaded(session),
                Err(error) => Message::Failed(error.to_string()),
            },
            Command::LoadIssues { view, filter } => match api.issues(&filter).await {
                Ok(issues) => Message::IssuesLoaded { view, issues },
                Err(error) => Message::Failed(error.to_string()),
            },
            Command::LoadInbox { view } => match api.notifications().await {
                Ok(items) => Message::InboxLoaded { view, items },
                Err(error) => Message::Failed(error.to_string()),
            },
            Command::LoadDetail(id) => match api.issue_detail(&id).await {
                Ok(Some(detail)) => Message::DetailLoaded(Box::new(detail)),
                Ok(None) => Message::Failed(format!("Issue {id} not found")),
                Err(error) => Message::Failed(error.to_string()),
            },
        };

        let _ = tx.send(message);
    });
}
