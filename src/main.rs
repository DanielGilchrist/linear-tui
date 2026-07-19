use std::io::{self, Stdout};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use linear_tui::api::{self, fixture::FixtureClient, Client, LinearApi};
use linear_tui::tui::{
    self,
    app::App,
    focus::{DetailView, Focus, LeftPanel},
    view::ViewKind,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, global = true)]
    api_key: Option<String>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    Render(RenderArgs),
    Record(RecordArgs),
}

#[derive(Parser)]
struct RenderArgs {
    #[arg(long)]
    fixture: Option<PathBuf>,

    #[arg(long, default_value = "assigned")]
    view: String,

    #[arg(long)]
    detail: Option<String>,

    #[arg(long, default_value_t = 110)]
    width: u16,

    #[arg(long, default_value_t = 32)]
    height: u16,
}

#[derive(Parser)]
struct RecordArgs {
    #[arg(long, default_value = "fixtures/recorded.json")]
    out: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Some(Command::Render(render_args)) => headless_render(render_args).await,
        Some(Command::Record(record_args)) => {
            record(&resolve_api_key(&args.api_key)?, record_args).await
        }
        None => run_tui(resolve_api_key(&args.api_key)?).await,
    }
}

fn resolve_api_key(flag: &Option<String>) -> Result<String> {
    flag.clone()
        .or_else(|| std::env::var("LINEAR_API_KEY").ok())
        .ok_or_else(|| anyhow!("Provide an API key via --api-key or LINEAR_API_KEY"))
}

async fn run_tui(api_key: String) -> Result<()> {
    let api: Arc<dyn LinearApi> = Arc::new(Client::new(api_key));

    let mut terminal = setup_terminal()?;
    let mut app = App::new();
    let result = tui::run(&mut terminal, &mut app, api).await;

    cleanup_terminal()?;
    result
}

async fn headless_render(args: RenderArgs) -> Result<()> {
    let api: Arc<dyn LinearApi> = match &args.fixture {
        Some(path) => Arc::new(FixtureClient::from_path(path)?),
        None => Arc::new(FixtureClient::sample()),
    };

    let mut app = App::new();
    app.session = api.session().await.ok();

    let index = view_index(&args.view);
    app.view_state.select(Some(index));
    match &app.active_view().kind {
        ViewKind::Issues(filter) => app.issues = api.issues(&filter.clone()).await?,
        ViewKind::Inbox => app.notifications = api.notifications().await?,
    }

    if let Some(id) = &args.detail {
        app.focus = Focus::Detail(LeftPanel::MyWork, DetailView::Reading);
        app.detail = api.issue_detail(id).await?;
    }

    let output = tui::render_to_string(&mut app, args.width, args.height);
    print!("{output}");
    Ok(())
}

fn view_index(name: &str) -> usize {
    match name {
        "progress" | "in-progress" => 1,
        "inbox" => 2,
        _ => 0,
    }
}

async fn record(api_key: &str, args: RecordArgs) -> Result<()> {
    use api::fixture::Fixture;
    use api::IssueFilter;

    let client = Client::new(api_key.to_string());
    let session = client.session().await?;
    let issues = client.issues(&IssueFilter::assigned_to_me()).await?;
    let notifications = client.notifications().await?;

    let mut details = Vec::new();
    for issue in issues.iter().take(5) {
        if let Some(detail) = client.issue_detail(&issue.id).await? {
            details.push(detail);
        }
    }

    let fixture = Fixture {
        viewer: session.user,
        org_name: session.org_name,
        org_url_key: session.org_url_key,
        notifications,
        issues,
        details,
    };

    if let Some(parent) = args.out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&args.out, serde_json::to_string_pretty(&fixture)?)?;
    eprintln!("Wrote fixture to {}", args.out.display());
    Ok(())
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn cleanup_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    print!("\x1B[?25h");
    Ok(())
}
