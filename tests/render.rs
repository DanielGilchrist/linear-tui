use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use linear_tui::api::fixture::FixtureClient;
use linear_tui::api::LinearApi;
use linear_tui::tui::app::{App, Focus, PickerItem, ViewKind};
use linear_tui::tui::message::Message;
use linear_tui::tui::render_to_string;
use linear_tui::tui::update::{apply, handle_key};

async fn home_app(client: &FixtureClient, view: usize) -> App {
    let mut app = App::new();
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
    app.focus = Focus::Detail;
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
async fn loading_placeholder() {
    let mut app = App::new();
    app.loading = true;
    insta::assert_snapshot!(render_to_string(&mut app, 110, 10));
}

#[tokio::test]
async fn status_picker_overlay() {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await;

    handle_key(&mut app, KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
    let states = client.workflow_states("t_pizza").await.unwrap();
    let items = states.into_iter().map(PickerItem::from).collect();
    apply(&mut app, Message::PickerLoaded(items));

    insta::assert_snapshot!(render_to_string(&mut app, 100, 20));
}

#[tokio::test]
async fn confirm_dialog() {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await;

    handle_key(&mut app, KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
    let states = client.workflow_states("t_pizza").await.unwrap();
    let items = states.into_iter().map(PickerItem::from).collect();
    apply(&mut app, Message::PickerLoaded(items));
    handle_key(&mut app, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    handle_key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    insta::assert_snapshot!(render_to_string(&mut app, 100, 20));
}
