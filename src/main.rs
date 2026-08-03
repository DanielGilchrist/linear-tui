use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};

use linear_tui::api::{self, fixture::FixtureClient, Client, Credential, IssueRef, LinearApi};
use linear_tui::store::StateDir;
use linear_tui::tui::render::theme::ColourMode;
use linear_tui::tui::{
    self,
    app::App,
    feed::{Feed, FeedKey},
    focus::{DetailFocus, LeftPanel, Origin},
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
        None => run_tui(bootstrap_credential(&args.api_key)).await,
    }
}

fn resolve_api_key(flag: &Option<String>) -> Result<String> {
    flag.clone()
        .or_else(|| std::env::var("LINEAR_API_KEY").ok())
        .ok_or_else(|| anyhow!("Provide an API key via --api-key or LINEAR_API_KEY"))
}

fn bootstrap_credential(flag: &Option<String>) -> Option<Credential> {
    flag.clone()
        .or_else(|| std::env::var("LINEAR_API_KEY").ok())
        .map(Credential::PersonalKey)
}

async fn run_tui(bootstrap: Option<Credential>) -> Result<()> {
    let make_client: tui::run::ClientFactory =
        Arc::new(|credential| Arc::new(Client::new(credential)) as Arc<dyn LinearApi>);

    tui::render::theme::init(colour_mode());

    if let Some(overrides) = theme_overrides()? {
        tui::render::theme::init_overrides(overrides);
    }

    let path = host_state_dir().ok_or_else(|| anyhow!("Could not resolve a state directory"))?;
    migrate_legacy_state_dir(&path);

    let mut terminal = ratatui::init();
    let mut app = App::new();
    let result = tui::run(
        &mut terminal,
        &mut app,
        bootstrap,
        make_client,
        StateDir::at(path),
    )
    .await;

    ratatui::restore();
    result
}

fn host_state_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from)?;

    #[cfg(target_os = "macos")]
    let base = home.join("Library/Application Support");

    #[cfg(target_os = "linux")]
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/state"));

    Some(base.join(linear_tui::APP_NAME))
}

#[cfg(target_os = "macos")]
fn migrate_legacy_state_dir(new: &std::path::Path) {
    if new.exists() {
        return;
    }

    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return;
    };

    let old = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/state"))
        .join(linear_tui::APP_NAME);

    if old.exists() {
        if let Some(parent) = new.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let _ = std::fs::rename(&old, new);
    }
}

#[cfg(not(target_os = "macos"))]
fn migrate_legacy_state_dir(_new: &std::path::Path) {}

fn colour_mode() -> ColourMode {
    match std::env::var_os("NO_COLOR") {
        Some(value) if !value.is_empty() => ColourMode::Monochrome,
        _ => ColourMode::Ansi,
    }
}

fn theme_overrides() -> Result<Option<linear_tui::tui::render::theme::Overrides>> {
    use anyhow::Context;

    let Some(path) = std::env::var_os("LINEAR_TUI_THEME").map(PathBuf::from) else {
        return Ok(None);
    };

    let json = std::fs::read_to_string(&path)
        .with_context(|| format!("Could not read theme file {}", path.display()))?;

    let overrides = linear_tui::tui::render::theme::Overrides::parse(&json)
        .with_context(|| format!("Could not parse theme file {}", path.display()))?;

    Ok(Some(overrides))
}

async fn headless_render(args: RenderArgs) -> Result<()> {
    tui::render::theme::init(ColourMode::Ansi);

    let api: Arc<dyn LinearApi> = match &args.fixture {
        Some(path) => Arc::new(FixtureClient::from_path(path)?),
        None => Arc::new(FixtureClient::sample()),
    };

    let mut app = App::new();
    if let Ok(session) = api.session().await {
        app.workspace.session.set(session, app.now);
    }

    let index = view_index(&args.view);
    app.ui.view_state.select(Some(index));

    match &app.active_view().kind {
        ViewKind::Issues(filter) => {
            let key = FeedKey::Issues(filter.clone());
            let page = api.issues(&filter.clone(), None).await?;

            app.workspace.feeds.insert(key, Feed::ready(page, app.now));
        }
        ViewKind::Inbox => {
            let page = api.notifications(None).await?;
            app.workspace.inbox = Feed::ready(page, app.now);
        }
    }

    if let Some(reference) = &args.detail {
        if let Some(detail) = api.issue_detail(&IssueRef::parse(reference)).await? {
            app.open_detail_focus(DetailFocus::reading(
                detail.id.clone(),
                Origin::Panel(LeftPanel::MyWork),
            ));
            app.workspace.set_detail(detail, app.now);
        }
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

    let client = Client::new(Credential::PersonalKey(api_key.to_string()));
    let session = client.session().await?;
    let issues = client
        .issues(&IssueFilter::assigned_to_me(), None)
        .await?
        .items;

    let notifications = client.notifications(None).await?.items;
    let saved_views = client.custom_views().await?;

    let mut details = Vec::new();
    for issue in issues.iter().take(5) {
        if let Some(detail) = client.issue_detail(&issue.id.clone().into()).await? {
            details.push(detail);
        }
    }

    let mut saved_view_issues = std::collections::HashMap::new();
    for view in &saved_views {
        let page = client.custom_view_issues(&view.id, None).await?;
        saved_view_issues.insert(view.id.clone(), page.items);
    }

    let fixture = Fixture {
        viewer: session.user,
        org_name: session.org_name,
        org_url_key: session.org_url_key,
        notifications,
        saved_views,
        issues,
        saved_view_issues,
        details,
    };

    if let Some(parent) = args.out.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(&args.out, serde_json::to_string_pretty(&fixture)?)?;
    eprintln!("Wrote fixture to {}", args.out.display());

    Ok(())
}
