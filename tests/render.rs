use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use linear_tui::api::fixture::FixtureClient;
use linear_tui::api::{Credential, ImageUrl, IssueRef, LinearApi, TeamId, Timestamp, ViewId};
use linear_tui::api::{Label, LabelId, Rgb};
use linear_tui::store::Account;
use linear_tui::tui::app::App;
use linear_tui::tui::cache::Remote;
use linear_tui::tui::feed::{Feed, FeedKey, FeedRequest};
use linear_tui::tui::focus::{DetailFocus, LeftPanel, Origin};
use linear_tui::tui::message::{
    Commands, Effect, EncodeImage, FailureTarget, ImageCommand, Message, RequestError,
};
use linear_tui::tui::render::image::EncodeFailure;
use linear_tui::tui::update::{apply, handle_key};
use linear_tui::tui::view::ViewKind;
use linear_tui::tui::{render_styled_to_string, render_to_string};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn edit(app: &mut App, field: char) {
    handle_key(app, KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
    handle_key(app, KeyEvent::new(KeyCode::Char(field), KeyModifiers::NONE));
}

struct Settled {
    frame: String,
    encodes: usize,
}

fn encode_requests(commands: Commands) -> Vec<EncodeImage> {
    match commands {
        Commands::Effects(effects) => effects
            .into_iter()
            .filter_map(|effect| match effect {
                Effect::Image(ImageCommand::Encode(request)) => Some(request),
                Effect::Image(ImageCommand::Fetch { .. })
                | Effect::Api(_)
                | Effect::Store(_)
                | Effect::Platform(_) => None,
            })
            .collect(),
        Commands::Runtime(_) => Vec::new(),
    }
}

fn settle(app: &mut App, width: u16, height: u16) -> TestResult<Settled> {
    let mut frame = render_to_string(app, width, height);
    let mut encodes = 0;

    for _ in 0..8 {
        let requests = encode_requests(linear_tui::tui::update::after_render(app));

        if requests.is_empty() {
            return Ok(Settled { frame, encodes });
        }

        for EncodeImage { url, request } in requests {
            let size = request.size;
            let encoded =
                linear_tui::tui::render::image::encode(&request.source, size).map(Box::new);
            encodes += 1;

            apply(app, Message::ImageEncoded { url, size, encoded });
        }

        frame = render_to_string(app, width, height);
    }

    Err("images kept asking to be re-encoded and never settled".into())
}

fn settled_frame(app: &mut App, width: u16, height: u16) -> TestResult<String> {
    Ok(settle(app, width, height)?.frame)
}

const TRACE: &str = "https://uploads.linear.app/trace.png";

fn trace_url() -> TestResult<ImageUrl> {
    ImageUrl::parse(TRACE).ok_or_else(|| "the trace url parses".into())
}

async fn deliver(app: &mut App, client: &FixtureClient, url: &ImageUrl) -> TestResult {
    app.workspace.begin_image(url, app.now);

    let bytes = client.image(url).await?;
    let decoded =
        linear_tui::tui::render::image::decode(&bytes).ok_or("the fixture png decodes")?;

    apply(
        app,
        Message::ImageLoaded {
            url: url.clone(),
            image: Box::new(decoded),
        },
    );

    Ok(())
}

fn expand_images(app: &mut App) {
    handle_key(app, KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE));
}

fn sign_in(app: &mut App) {
    app.session.upsert_account(Account {
        workspace_key: "ws".into(),
        org_name: "Test".into(),
        credential: Credential::PersonalKey("k".into()),
    });
    assert!(app.session.activate("ws"));
}

async fn home_app(client: &FixtureClient, view: usize) -> TestResult<App> {
    let mut app = App::new();
    sign_in(&mut app);
    app.now = Timestamp::from("2026-07-16T21:00:00Z");
    app.workspace.session = Remote::ready(client.session().await?, app.now);
    app.ui.view_state.select(Some(view));
    match &app.active_view().kind {
        ViewKind::Issues(filter) => {
            let page = client.issues(&filter.clone(), None).await?;
            app.workspace
                .feeds
                .insert(FeedKey::Issues(filter.clone()), Feed::ready(page, app.now));
        }
        ViewKind::Inbox => {
            let page = client.notifications(None).await?;
            app.workspace.inbox = Feed::ready(page, app.now);
        }
    }

    Ok(app)
}

async fn load_view(app: &mut App, client: &FixtureClient, id: &ViewId) -> TestResult {
    let page = client.custom_view_issues(id, None).await?;
    apply(
        app,
        Message::FeedLoaded {
            key: FeedKey::View(id.clone()),
            request: FeedRequest::Refresh,
            page,
        },
    );

    Ok(())
}

async fn opened_detail_app(client: &FixtureClient) -> TestResult<App> {
    let mut app = home_app(client, 0).await?;
    if let Some(detail) = client.issue_detail(&IssueRef::parse("DAN2-7")).await? {
        app.open_detail_focus(DetailFocus::reading(
            detail.id.clone(),
            Origin::Panel(LeftPanel::MyWork),
        ));
        app.workspace.set_detail(detail, app.now);
    }

    Ok(app)
}

#[tokio::test]
async fn reactions_overlay_shows_current_and_add_sections() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await?;
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('+'), KeyModifiers::NONE),
    );
    insta::assert_snapshot!(render_to_string(&mut app, 90, 22));

    Ok(())
}

async fn saved_views_app(client: &FixtureClient) -> TestResult<App> {
    let mut app = App::new();
    sign_in(&mut app);
    app.now = Timestamp::from("2026-07-16T21:00:00Z");
    app.workspace.session = Remote::ready(client.session().await?, app.now);
    app.focus_panel(LeftPanel::SavedViews);
    apply(
        &mut app,
        Message::CustomViewsLoaded(client.custom_views().await?),
    );

    Ok(app)
}

#[tokio::test]
async fn assigned_to_me_view() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = home_app(&client, 0).await?;
    insta::assert_snapshot!(render_to_string(&mut app, 110, 16));

    Ok(())
}

#[tokio::test]
async fn a_refreshing_session_still_shows_an_error_in_the_footer() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = home_app(&client, 0).await?;
    app.session.begin_refresh(app.now);
    app.ui.status = Some(linear_tui::tui::status::Status::Error(
        "linear returned http 500".into(),
    ));

    let output = render_to_string(&mut app, 110, 16);

    assert!(
        output.contains("linear returned http 500"),
        "the error must not be masked by the refreshing banner"
    );
    assert!(!output.contains("Refreshing your session"));

    Ok(())
}

#[tokio::test]
async fn a_first_run_with_no_account_is_not_shown_as_connected() -> TestResult {
    let mut app = App::new();

    let output = render_to_string(&mut app, 84, 16);

    assert!(
        output.contains("Not connected"),
        "a first run with no account must not render as an authenticated session:\n{output}"
    );

    Ok(())
}

#[tokio::test]
async fn in_progress_view() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = home_app(&client, 1).await?;
    insta::assert_snapshot!(render_to_string(&mut app, 110, 16));

    Ok(())
}

#[tokio::test]
async fn inbox_view() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = home_app(&client, 2).await?;
    insta::assert_snapshot!(render_to_string(&mut app, 110, 12));

    Ok(())
}

#[tokio::test]
async fn issue_detail() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await?;
    insta::assert_snapshot!(render_to_string(&mut app, 110, 26));

    Ok(())
}

#[tokio::test]
async fn saved_views_list() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = saved_views_app(&client).await?;
    let id = app
        .workspace
        .saved_views
        .selected_view()
        .ok_or("no saved view is selected")?
        .id
        .clone();
    load_view(&mut app, &client, &id).await?;
    insta::assert_snapshot!(render_to_string(&mut app, 110, 16));

    Ok(())
}

async fn open_view_app(client: &FixtureClient) -> TestResult<App> {
    let mut app = saved_views_app(client).await?;
    let id = app
        .workspace
        .saved_views
        .selected_view()
        .ok_or("no saved view is selected")?
        .id
        .clone();
    handle_key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    load_view(&mut app, client, &id).await?;

    Ok(app)
}

#[tokio::test]
async fn view_in_right_pane() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = open_view_app(&client).await?;
    insta::assert_snapshot!(render_to_string(&mut app, 110, 26));

    Ok(())
}

#[tokio::test]
async fn view_zoomed() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = open_view_app(&client).await?;
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE),
    );
    insta::assert_snapshot!(render_to_string(&mut app, 110, 26));

    Ok(())
}

#[tokio::test]
async fn a_truncated_view_marks_the_count_with_a_plus() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = saved_views_app(&client).await?;
    let id = app
        .workspace
        .saved_views
        .selected_view()
        .ok_or("no saved view is selected")?
        .id
        .clone();
    handle_key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let items = client.custom_view_issues(&id, None).await?.items;
    apply(
        &mut app,
        Message::FeedLoaded {
            key: FeedKey::View(id),
            request: FeedRequest::Refresh,
            page: linear_tui::api::Page {
                items,
                next: Some(linear_tui::api::Cursor("more".into())),
            },
        },
    );

    let out = render_to_string(&mut app, 110, 26);
    assert!(
        out.contains("+ issues"),
        "a truncated page did not mark the count:\n{out}"
    );

    Ok(())
}

#[tokio::test]
async fn view_grouped_by_priority() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = open_view_app(&client).await?;
    // v then g cycles group status -> priority
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE),
    );
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
    );
    insta::assert_snapshot!(render_to_string(&mut app, 110, 26));

    Ok(())
}

#[tokio::test]
async fn threaded_comments_and_timestamps() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await?;
    insta::assert_snapshot!(render_to_string(&mut app, 90, 46));

    Ok(())
}

#[tokio::test]
async fn comments_mode_scrolls_the_selected_comment_to_the_top() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await?;

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE),
    );

    insta::assert_snapshot!(render_to_string(&mut app, 90, 20));

    Ok(())
}

#[tokio::test]
async fn detail_view_keeps_the_source_panel_expanded() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await?;

    let out = render_to_string(&mut app, 110, 26);

    assert!(
        out.contains("DAN2-2"),
        "My Work collapsed while viewing a detail:\n{out}"
    );

    Ok(())
}

#[tokio::test]
async fn loading_placeholder() -> TestResult {
    let mut app = App::new();
    sign_in(&mut app);
    let key = app.active_feed_key().ok_or("no active feed")?;
    app.workspace
        .feeds
        .get_or_default(&key)
        .begin(&FeedRequest::Refresh);
    insta::assert_snapshot!(render_to_string(&mut app, 110, 10));

    Ok(())
}

#[tokio::test]
async fn teams_panel_focused_expands() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = home_app(&client, 0).await?;
    app.focus_panel(LeftPanel::Teams);
    insta::assert_snapshot!(render_to_string(&mut app, 84, 24));

    Ok(())
}

#[tokio::test]
async fn team_surface_shows_its_mode() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = home_app(&client, 0).await?;
    app.focus_panel(LeftPanel::Teams);
    apply(
        &mut app,
        Message::TeamsLoaded {
            teams: client.teams().await?,
        },
    );

    handle_key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let filter = match app.view().map(|view| view.key()) {
        Some(FeedKey::Issues(filter)) => filter,
        other => return Err(format!("expected a team feed, got {other:?}").into()),
    };
    let page = client.issues(&filter, None).await?;
    app.workspace
        .feeds
        .insert(FeedKey::Issues(filter), Feed::ready(page, app.now));

    insta::assert_snapshot!(render_to_string(&mut app, 84, 24));

    Ok(())
}

#[tokio::test]
async fn team_triage_mode_shows_unrouted_issues() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = home_app(&client, 0).await?;
    app.focus_panel(LeftPanel::Teams);
    apply(
        &mut app,
        Message::TeamsLoaded {
            teams: client.teams().await?,
        },
    );

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
    );
    handle_key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE),
    );

    let filter = match app.view().map(|view| view.key()) {
        Some(FeedKey::Issues(filter)) => filter,
        other => return Err(format!("expected a team feed, got {other:?}").into()),
    };
    let page = client.issues(&filter, None).await?;
    app.workspace
        .feeds
        .insert(FeedKey::Issues(filter), Feed::ready(page, app.now));

    insta::assert_snapshot!(render_to_string(&mut app, 84, 24));

    Ok(())
}

#[tokio::test]
async fn teams_panel_loading() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = home_app(&client, 0).await?;
    app.focus_panel(LeftPanel::Teams);
    app.workspace.teams.teams.begin();
    insta::assert_snapshot!(render_to_string(&mut app, 84, 24));

    Ok(())
}

#[tokio::test]
async fn teams_panel_failed() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = home_app(&client, 0).await?;
    app.focus_panel(LeftPanel::Teams);
    apply(
        &mut app,
        Message::Failed {
            target: FailureTarget::Teams,
            error: RequestError::Other("Linear is unreachable".into()),
        },
    );
    insta::assert_snapshot!(render_to_string(&mut app, 84, 24));

    Ok(())
}

#[tokio::test]
async fn help_overlay() -> TestResult {
    let mut app = App::new();
    sign_in(&mut app);
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE),
    );
    insta::assert_snapshot!(render_to_string(&mut app, 84, 24));

    Ok(())
}

#[tokio::test]
async fn status_picker_overlay() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await?;

    edit(&mut app, 's');
    let states = client.workflow_states(&TeamId::from_raw("t_pizza")).await?;
    apply(
        &mut app,
        Message::StatesLoaded {
            team_id: TeamId::from_raw("t_pizza"),
            states,
        },
    );

    insta::assert_snapshot!(render_to_string(&mut app, 100, 20));

    Ok(())
}

#[tokio::test]
async fn labels_overlay_lists_and_marks_selected() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await?;

    edit(&mut app, 'l');
    apply(
        &mut app,
        Message::LabelsFound {
            query: String::new(),
            labels: vec![
                Label {
                    id: LabelId::from_raw("lbl_oven"),
                    name: "oven".into(),
                    colour: Rgb::parse_hex("#eb5757"),
                },
                Label {
                    id: LabelId::from_raw("lbl_bug"),
                    name: "bug".into(),
                    colour: Rgb::parse_hex("#5e6ad2"),
                },
            ],
        },
    );

    insta::assert_snapshot!(render_to_string(&mut app, 100, 20));

    Ok(())
}

#[tokio::test]
async fn assign_picker_overlay() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await?;
    app.workspace.session = Remote::ready(client.session().await?, app.now);

    edit(&mut app, 'a');

    insta::assert_snapshot!(render_to_string(&mut app, 100, 20));

    Ok(())
}

#[tokio::test]
async fn assign_picker_search_results() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await?;
    app.workspace.session = Remote::ready(client.session().await?, app.now);

    edit(&mut app, 'a');
    for key in ['/', 'a'] {
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE),
        );
    }

    handle_key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let users = client.search_users("a").await?;

    apply(
        &mut app,
        Message::UsersFound {
            query: "a".into(),
            users,
        },
    );

    insta::assert_snapshot!(render_to_string(&mut app, 100, 20));

    Ok(())
}

#[tokio::test]
async fn comment_editor_overlay() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await?;

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE),
    );
    let script = "Checked the damper.\nSpring tension looks off, ordering a replacement.";
    for c in script.chars() {
        let key = match c {
            '\n' => KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            _ => KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
        };
        handle_key(&mut app, key);
    }

    insta::assert_snapshot!(render_to_string(&mut app, 90, 22));

    Ok(())
}

#[tokio::test]
async fn mention_autocomplete_popup() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await?;

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE),
    );
    let members = client.team_members(&TeamId::from_raw("t_pizza")).await?;
    apply(
        &mut app,
        Message::MembersLoaded {
            team_id: TeamId::from_raw("t_pizza"),
            members,
        },
    );
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('@'), KeyModifiers::NONE),
    );

    insta::assert_snapshot!(render_to_string(&mut app, 90, 24));

    Ok(())
}

#[tokio::test]
async fn go_prefix_overlay() -> TestResult {
    let mut app = App::new();
    sign_in(&mut app);
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
    );
    insta::assert_snapshot!(render_to_string(&mut app, 84, 16));

    Ok(())
}

#[tokio::test]
async fn jump_input_overlay() -> TestResult {
    let mut app = App::new();
    sign_in(&mut app);
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
    );
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE),
    );
    for c in "DAN2-7".chars() {
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
        );
    }
    insta::assert_snapshot!(render_to_string(&mut app, 84, 16));

    Ok(())
}

#[tokio::test]
async fn jump_input_scrolls_to_keep_a_long_url_cursor_visible() -> TestResult {
    let mut app = App::new();
    sign_in(&mut app);
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
    );
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE),
    );
    for c in "https://linear.app/dans-donuts/issue/DAN2-7/wood-fired-oven".chars() {
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
        );
    }
    insta::assert_snapshot!(render_to_string(&mut app, 84, 16));

    Ok(())
}

#[tokio::test]
async fn local_find_bar() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = home_app(&client, 0).await?;

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE),
    );
    for c in "oven".chars() {
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
        );
    }

    insta::assert_snapshot!(render_to_string(&mut app, 100, 14));

    Ok(())
}

#[tokio::test]
async fn active_search_bar() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = home_app(&client, 0).await?;

    for key in ['/', 'i', 'n', ' ', 'p'] {
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE),
        );
    }
    handle_key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    insta::assert_snapshot!(render_to_string(&mut app, 100, 14));

    Ok(())
}

#[tokio::test]
async fn search_results_overlay() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = home_app(&client, 0).await?;

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
    );
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE),
    );
    for c in "oven".chars() {
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
        );
    }
    handle_key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let page = client.search_issues("oven", None).await?;
    apply(
        &mut app,
        Message::FeedLoaded {
            key: FeedKey::Search("oven".to_string()),
            request: FeedRequest::Refresh,
            page,
        },
    );

    insta::assert_snapshot!(render_to_string(&mut app, 100, 20));

    Ok(())
}

#[tokio::test]
async fn confirm_dialog() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await?;

    edit(&mut app, 's');
    let states = client.workflow_states(&TeamId::from_raw("t_pizza")).await?;
    apply(
        &mut app,
        Message::StatesLoaded {
            team_id: TeamId::from_raw("t_pizza"),
            states,
        },
    );
    handle_key(&mut app, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    handle_key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    insta::assert_snapshot!(render_to_string(&mut app, 100, 20));

    Ok(())
}

#[tokio::test]
async fn detail_viewport_is_the_pane_height_minus_border() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await?;
    let _ = render_to_string(&mut app, 110, 26);
    // right pane = body (26 - 1 footer) = 25, minus 2 for the border.
    assert_eq!(app.ui.viewport, 23);

    Ok(())
}

#[tokio::test]
async fn view_surface_viewport_accounts_for_the_group_sort_header() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = open_view_app(&client).await?;
    let _ = render_to_string(&mut app, 110, 26);
    // right pane 25, minus 2 border, minus the 3-row group/sort header.
    assert_eq!(app.ui.viewport, 20);

    Ok(())
}

#[tokio::test]
async fn styled_assigned_view() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = home_app(&client, 0).await?;
    insta::assert_snapshot!(render_styled_to_string(&mut app, 110, 16));

    Ok(())
}

#[tokio::test]
async fn styled_issue_detail() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await?;
    insta::assert_snapshot!(render_styled_to_string(&mut app, 110, 26));

    Ok(())
}

#[tokio::test]
async fn styled_view_in_right_pane() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = open_view_app(&client).await?;
    insta::assert_snapshot!(render_styled_to_string(&mut app, 110, 26));

    Ok(())
}

#[tokio::test]
async fn styled_view_grouped_by_priority() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = open_view_app(&client).await?;
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE),
    );
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
    );
    insta::assert_snapshot!(render_styled_to_string(&mut app, 110, 26));

    Ok(())
}

#[tokio::test]
async fn styled_status_picker_overlay() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await?;
    edit(&mut app, 's');
    let states = client.workflow_states(&TeamId::from_raw("t_pizza")).await?;
    apply(
        &mut app,
        Message::StatesLoaded {
            team_id: TeamId::from_raw("t_pizza"),
            states,
        },
    );
    insta::assert_snapshot!(render_styled_to_string(&mut app, 100, 20));

    Ok(())
}

#[tokio::test]
async fn styled_local_find_bar() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = home_app(&client, 0).await?;
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE),
    );
    for c in "oven".chars() {
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
        );
    }
    insta::assert_snapshot!(render_styled_to_string(&mut app, 100, 14));

    Ok(())
}

#[tokio::test]
async fn styled_help_overlay() -> TestResult {
    let mut app = App::new();
    sign_in(&mut app);
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE),
    );
    insta::assert_snapshot!(render_styled_to_string(&mut app, 84, 24));

    Ok(())
}

#[tokio::test]
async fn styled_comment_editor_overlay() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await?;
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE),
    );
    let members = client.team_members(&TeamId::from_raw("t_pizza")).await?;
    apply(
        &mut app,
        Message::MembersLoaded {
            team_id: TeamId::from_raw("t_pizza"),
            members,
        },
    );
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('@'), KeyModifiers::NONE),
    );
    insta::assert_snapshot!(render_styled_to_string(&mut app, 90, 24));

    Ok(())
}

#[tokio::test]
async fn styled_reactions_overlay() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await?;
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('+'), KeyModifiers::NONE),
    );
    insta::assert_snapshot!(render_styled_to_string(&mut app, 90, 22));

    Ok(())
}

#[tokio::test]
async fn styled_workspaces_overlay() -> TestResult {
    let mut app = App::new();
    sign_in(&mut app);
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE),
    );
    insta::assert_snapshot!(render_styled_to_string(&mut app, 84, 20));

    Ok(())
}

#[tokio::test]
async fn styled_menu_overlay() -> TestResult {
    let mut app = App::new();
    sign_in(&mut app);
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE),
    );
    insta::assert_snapshot!(render_styled_to_string(&mut app, 84, 24));

    Ok(())
}

#[tokio::test]
async fn styled_mention_autocomplete_popup() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await?;

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE),
    );
    let members = client.team_members(&TeamId::from_raw("t_pizza")).await?;
    apply(
        &mut app,
        Message::MembersLoaded {
            team_id: TeamId::from_raw("t_pizza"),
            members,
        },
    );
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('@'), KeyModifiers::NONE),
    );

    insta::assert_snapshot!(render_styled_to_string(&mut app, 90, 24));

    Ok(())
}

#[tokio::test]
async fn styled_confirm_dialog() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await?;

    edit(&mut app, 's');
    let states = client.workflow_states(&TeamId::from_raw("t_pizza")).await?;
    apply(
        &mut app,
        Message::StatesLoaded {
            team_id: TeamId::from_raw("t_pizza"),
            states,
        },
    );
    handle_key(&mut app, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    handle_key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    insta::assert_snapshot!(render_styled_to_string(&mut app, 100, 20));

    Ok(())
}

#[tokio::test]
async fn styled_teams_panel() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = home_app(&client, 0).await?;
    app.focus_panel(LeftPanel::Teams);
    apply(
        &mut app,
        Message::TeamsLoaded {
            teams: client.teams().await?,
        },
    );
    insta::assert_snapshot!(render_styled_to_string(&mut app, 84, 24));

    Ok(())
}

#[tokio::test]
async fn only_label_chips_may_carry_raw_rgb() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = home_app(&client, 2).await?;

    let frame = render_styled_to_string(&mut app, 110, 16);

    assert!(
        !frame.contains("Rgb("),
        "a surface with no label chips must carry no raw colour"
    );

    Ok(())
}

#[tokio::test]
async fn an_image_is_collapsed_to_one_row_by_default() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await?;

    let mut detail = app.workspace.detail().value().cloned().ok_or("detail")?;
    detail.description = Some(
        "Before the shot\n\n![oven trace](https://uploads.linear.app/trace.png)\n\nAfter the shot"
            .into(),
    );
    app.workspace.set_detail(detail, app.now);

    insta::assert_snapshot!(render_to_string(&mut app, 90, 24));

    Ok(())
}

#[tokio::test]
async fn a_partly_visible_image_is_clipped_rather_than_hidden() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await?;

    let mut detail = app.workspace.detail().value().cloned().ok_or("detail")?;
    detail.description =
        Some("Before the shot\n\n![oven trace](https://uploads.linear.app/trace.png)".into());
    app.workspace.set_detail(detail, app.now);

    expand_images(&mut app);

    let roomy = render_to_string(&mut app, 90, 24);
    let cramped = render_to_string(&mut app, 90, 15);

    assert!(
        roomy.contains("┌oven trace"),
        "the box is drawn when it fits"
    );
    assert!(
        cramped.contains("┌oven trace"),
        "a box with room for a few rows is clipped, not hidden"
    );

    Ok(())
}

#[tokio::test]
async fn a_sliver_of_an_image_still_renders() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await?;

    let mut detail = app.workspace.detail().value().cloned().ok_or("detail")?;
    detail.description =
        Some("Before the shot\n\n![oven trace](https://uploads.linear.app/trace.png)".into());
    app.workspace.set_detail(detail, app.now);

    expand_images(&mut app);

    let frame = render_to_string(&mut app, 90, 12);

    assert!(
        frame.contains("┌oven trace"),
        "even a couple of rows renders rather than vanishing"
    );

    Ok(())
}

#[tokio::test]
async fn a_loaded_image_draws_pixels_into_the_reserved_box() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await?;

    let url = trace_url()?;
    let mut detail = app.workspace.detail().value().cloned().ok_or("detail")?;
    detail.description = Some(format!("Before the shot\n\n![oven trace]({url})"));
    app.workspace.set_detail(detail, app.now);

    expand_images(&mut app);

    let placeholder = render_to_string(&mut app, 90, 24);
    assert!(
        placeholder.contains("┌oven trace"),
        "an unloaded image shows the reserved box"
    );

    deliver(&mut app, &client, &url).await?;

    let first = render_to_string(&mut app, 90, 24);

    assert!(
        first.contains("Loading image"),
        "the first frame after the bytes land shows the loader, got:\n{first}"
    );
    let drawn = settled_frame(&mut app, 90, 24)?;

    assert!(
        !drawn.contains("┌oven trace"),
        "a loaded image replaces the placeholder box"
    );
    assert!(
        drawn.contains('\u{2580}'),
        "halfblocks paint the reserved rows, got:\n{drawn}"
    );

    Ok(())
}

#[tokio::test]
async fn scrolling_past_an_image_does_not_re_encode_it() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await?;

    let url = trace_url()?;
    let mut detail = app.workspace.detail().value().cloned().ok_or("detail")?;
    detail.description = Some(format!(
        "{}\n\n![oven trace]({url})\n\n{}",
        "filler ".repeat(40),
        "tail ".repeat(80)
    ));
    app.workspace.set_detail(detail, app.now);

    deliver(&mut app, &client, &url).await?;

    expand_images(&mut app);
    let first = settle(&mut app, 90, 20)?.encodes;

    assert_eq!(first, 1, "the first settled frame encodes once");

    let mut extra = 0;

    for _ in 0..12 {
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
        );
        extra += settle(&mut app, 90, 20)?.encodes;
    }

    assert_eq!(extra, 0, "clipping must not re-encode while scrolling");

    Ok(())
}

#[tokio::test]
async fn an_image_scrolled_half_off_the_top_still_encodes_once() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await?;

    let url = trace_url()?;
    let mut detail = app.workspace.detail().value().cloned().ok_or("detail")?;
    detail.description = Some(format!(
        "{}\n\n![oven trace]({url})\n\n{}",
        "filler ".repeat(30),
        "tail ".repeat(120)
    ));
    app.workspace.set_detail(detail, app.now);

    deliver(&mut app, &client, &url).await?;

    expand_images(&mut app);
    let first = settle(&mut app, 90, 20)?.encodes;

    assert_eq!(first, 1, "the first settled frame encodes once");

    let mut extra = 0;

    for _ in 0..30 {
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
        );
        extra += settle(&mut app, 90, 20)?.encodes;
    }

    assert_eq!(
        extra, 0,
        "slicing must hold the encode stable while the image scrolls off the top"
    );

    Ok(())
}

#[tokio::test]
async fn a_pending_encode_is_requested_once_across_frames() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await?;

    let url = trace_url()?;
    let mut detail = app.workspace.detail().value().cloned().ok_or("detail")?;
    detail.description = Some(format!("Before the shot\n\n![oven trace]({url})"));
    app.workspace.set_detail(detail, app.now);

    deliver(&mut app, &client, &url).await?;
    expand_images(&mut app);

    let requests =
        |app: &mut App| encode_requests(linear_tui::tui::update::after_render(app)).len();

    render_to_string(&mut app, 90, 24);
    assert_eq!(requests(&mut app), 1);
    assert!(app.is_loading(), "the spinner ticks while the encode runs");

    render_to_string(&mut app, 90, 24);
    assert_eq!(
        requests(&mut app),
        0,
        "a frame drawn while the encode is in flight must not queue another"
    );

    Ok(())
}

#[tokio::test]
async fn a_failed_encode_shows_an_error_and_stops_asking() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await?;

    let url = trace_url()?;
    let mut detail = app.workspace.detail().value().cloned().ok_or("detail")?;
    detail.description = Some(format!("Before the shot\n\n![oven trace]({url})"));
    app.workspace.set_detail(detail, app.now);

    deliver(&mut app, &client, &url).await?;
    expand_images(&mut app);

    render_to_string(&mut app, 90, 24);
    let size = encode_requests(linear_tui::tui::update::after_render(&mut app))
        .first()
        .map(|job| job.request.size)
        .ok_or("an encode request")?;

    apply(
        &mut app,
        Message::ImageEncoded {
            url: url.clone(),
            size,
            encoded: Err(EncodeFailure::Unsupported),
        },
    );

    let frame = render_to_string(&mut app, 90, 24);

    assert!(
        frame.contains("Could not render this image"),
        "got:\n{frame}"
    );
    assert!(!app.is_loading(), "a failed encode must not spin forever");
    assert!(
        linear_tui::tui::update::after_render(&mut app).is_empty(),
        "a failed size is not retried every frame"
    );

    Ok(())
}

#[tokio::test]
async fn an_image_shown_inline_and_in_the_gallery_keeps_both_encodings() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await?;

    let url = trace_url()?;
    let mut detail = app.workspace.detail().value().cloned().ok_or("detail")?;
    detail.description = Some(format!("Before the shot\n\n![oven trace]({url})"));
    app.workspace.set_detail(detail, app.now);

    deliver(&mut app, &client, &url).await?;
    expand_images(&mut app);
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('I'), KeyModifiers::SHIFT),
    );

    let first = settle(&mut app, 90, 24)?.encodes;

    assert_eq!(first, 2, "one encoding for the pane, one for the gallery");

    let mut extra = 0;

    for _ in 0..5 {
        extra += settle(&mut app, 90, 24)?.encodes;
    }

    assert_eq!(
        extra, 0,
        "two sizes on screen at once must not evict each other"
    );

    Ok(())
}

#[tokio::test]
async fn the_gallery_says_so_when_its_image_leaves_the_issue() -> TestResult {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await?;

    let url = trace_url()?;
    let mut detail = app.workspace.detail().value().cloned().ok_or("detail")?;
    let original = detail.description.clone();
    detail.description = Some(format!("Before the shot\n\n![oven trace]({url})"));
    app.workspace.set_detail(detail.clone(), app.now);

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('I'), KeyModifiers::SHIFT),
    );

    detail.description = original;
    app.workspace.set_detail(detail, app.now);

    let frame = render_to_string(&mut app, 90, 24);

    assert!(
        frame.contains("Image not loaded"),
        "a pruned image must not spin forever, got:\n{frame}"
    );
    assert!(
        frame.contains("esc/q/I close"),
        "the close keys come from the keymap, got:\n{frame}"
    );

    Ok(())
}
