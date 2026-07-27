use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::Event,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    env,
    io::{self, Stdout},
};

mod api;
mod tui;

use api::Client;
use tui::{
    events::{handle_key_event, poll_event, AppEvent},
    App, AppState,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    api_key: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let api_key = args
        .api_key
        .or_else(|| env::var("LINEAR_API_KEY").ok())
        .expect(
            "Please provide API key via --api-key argument or LINEAR_API_KEY environment variable",
        );

    let client = Client::new(api_key);

    with_raw_terminal(|mut terminal| async move {
        let mut app = App::new(client);
        app.load_teams().await?;

        run_app(&mut terminal, &mut app).await
    })
    .await
}

async fn with_raw_terminal<F, Fut, T>(f: F) -> Result<T>
where
    F: FnOnce(Terminal<CrosstermBackend<io::Stdout>>) -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let terminal = setup_terminal()?;
    let result = f(terminal).await;

    cleanup_terminal()?;
    show_cursor();

    result
}

fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn cleanup_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    Ok(())
}

fn show_cursor() {
    print!("\x1B[?25h");
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| app.render(f))?;

        if let Some(event) = poll_event()? {
            match event {
                Event::Key(key) => {
                    let app_event = handle_key_event(key);

                    match app_event {
                        AppEvent::Quit => break,
                        AppEvent::NavigateTo => match app.state {
                            AppState::TeamsList => {
                                if let Some(team) = app.get_selected_team() {
                                    let team_id = team.id.inner().to_string();
                                    app.selected_team = Some(team.clone());
                                    app.state = AppState::IssuesList;
                                    app.load_team_issues(&team_id).await?;
                                }
                            }
                            AppState::IssuesList => {
                                if let Some(issue) = app.get_selected_team_issue() {
                                    let issue_id = issue.id.inner().to_string();
                                    app.state = AppState::IssueDetail;
                                    app.load_issue_detail(&issue_id).await?;
                                }
                            }
                            _ => {}
                        },
                        AppEvent::GoBack => match app.state {
                            AppState::IssueDetail => {
                                app.state = AppState::IssuesList;
                            }
                            AppState::IssuesList => {
                                app.state = AppState::TeamsList;
                            }
                            _ => {}
                        },
                        AppEvent::NextItem => app.next_item(),
                        AppEvent::PreviousItem => app.previous_item(),
                        AppEvent::None => {}
                    }
                }
                Event::Resize(_, _) => {
                    // Terminal was resized, will be handled on next draw
                }
                _ => {}
            }
        }
    }

    Ok(())
}
