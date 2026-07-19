use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use linear_tui::api::fixture::FixtureClient;
use linear_tui::api::{LinearApi, Timestamp};
use linear_tui::tui::app::App;
use linear_tui::tui::focus::{Focus, LeftPanel};
use linear_tui::tui::message::Message;
use linear_tui::tui::overlay::PickerItem;
use linear_tui::tui::render_to_string;
use linear_tui::tui::update::{apply, handle_key};
use linear_tui::tui::view::ViewKind;

async fn home_app(client: &FixtureClient, view: usize) -> App {
    let mut app = App::new();
    app.now = Timestamp::from("2026-07-16T21:00:00Z").epoch();
    app.session = client.session().await.ok();
    app.view_state.select(Some(view));
    match &app.active_view().kind {
        ViewKind::Issues(filter) => app.issues = client.issues(&filter.clone()).await.unwrap(),
        ViewKind::Inbox => app.notifications = client.notifications().await.unwrap(),
    }
    app
}

async fn opened_detail_app(client: &FixtureClient) -> App {
    let mut app = home_app(client, 0).await;
    app.focus = Focus::Detail(LeftPanel::MyWork);
    app.detail = client.issue_detail("DAN2-7").await.unwrap();
    app
}

#[tokio::test]
async fn assigned_to_me_view() {
    let client = FixtureClient::sample();
    let mut app = home_app(&client, 0).await;
    insta::assert_snapshot!(render_to_string(&mut app, 110, 16));
}

#[tokio::test]
async fn in_progress_view() {
    let client = FixtureClient::sample();
    let mut app = home_app(&client, 1).await;
    insta::assert_snapshot!(render_to_string(&mut app, 110, 16));
}

#[tokio::test]
async fn inbox_view() {
    let client = FixtureClient::sample();
    let mut app = home_app(&client, 2).await;
    insta::assert_snapshot!(render_to_string(&mut app, 110, 12));
}

#[tokio::test]
async fn issue_detail() {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await;
    insta::assert_snapshot!(render_to_string(&mut app, 110, 26));
}

#[tokio::test]
async fn threaded_comments_and_timestamps() {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await;
    insta::assert_snapshot!(render_to_string(&mut app, 90, 46));
}

#[tokio::test]
async fn detail_view_keeps_the_source_panel_expanded() {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await;

    let out = render_to_string(&mut app, 110, 26);

    assert!(
        out.contains("DAN2-2"),
        "My Work collapsed while viewing a detail:\n{out}"
    );
}

#[tokio::test]
async fn loading_placeholder() {
    let mut app = App::new();
    app.loading = true;
    insta::assert_snapshot!(render_to_string(&mut app, 110, 10));
}

#[tokio::test]
async fn stub_panel_focused_expands() {
    let client = FixtureClient::sample();
    let mut app = home_app(&client, 0).await;
    app.focus = Focus::Stub(0);
    insta::assert_snapshot!(render_to_string(&mut app, 84, 24));
}

#[tokio::test]
async fn help_overlay() {
    let mut app = App::new();
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE),
    );
    insta::assert_snapshot!(render_to_string(&mut app, 84, 24));
}

#[tokio::test]
async fn status_picker_overlay() {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await;

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE),
    );
    let states = client.workflow_states("t_pizza").await.unwrap();
    let items = states.into_iter().map(PickerItem::from).collect();
    apply(&mut app, Message::PickerLoaded(items));

    insta::assert_snapshot!(render_to_string(&mut app, 100, 20));
}

#[tokio::test]
async fn go_prefix_overlay() {
    let mut app = App::new();
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
    );
    insta::assert_snapshot!(render_to_string(&mut app, 84, 16));
}

#[tokio::test]
async fn jump_input_overlay() {
    let mut app = App::new();
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
}

#[tokio::test]
async fn local_find_bar() {
    let client = FixtureClient::sample();
    let mut app = home_app(&client, 0).await;

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
}

#[tokio::test]
async fn active_search_bar() {
    let client = FixtureClient::sample();
    let mut app = home_app(&client, 0).await;

    for key in ['/', 'i', 'n', ' ', 'p'] {
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE),
        );
    }
    handle_key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    insta::assert_snapshot!(render_to_string(&mut app, 100, 14));
}

#[tokio::test]
async fn search_results_overlay() {
    let client = FixtureClient::sample();
    let mut app = home_app(&client, 0).await;

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
    let results = client.search_issues("oven").await.unwrap();
    apply(&mut app, Message::SearchResults(results));

    insta::assert_snapshot!(render_to_string(&mut app, 100, 20));
}

#[tokio::test]
async fn confirm_dialog() {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await;

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE),
    );
    let states = client.workflow_states("t_pizza").await.unwrap();
    let items = states.into_iter().map(PickerItem::from).collect();
    apply(&mut app, Message::PickerLoaded(items));
    handle_key(&mut app, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    handle_key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    insta::assert_snapshot!(render_to_string(&mut app, 100, 20));
}
