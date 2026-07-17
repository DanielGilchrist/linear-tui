use linear_tui::api::fixture::FixtureClient;
use linear_tui::api::LinearApi;
use linear_tui::tui::app::{App, Screen, ViewKind};
use linear_tui::tui::render_to_string;

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

#[tokio::test]
async fn assigned_to_me_view() {
    let client = FixtureClient::sample();
    let mut app = home_app(&client, 0).await;
    insta::assert_snapshot!(render_to_string(&mut app, 110, 12));
}

#[tokio::test]
async fn in_progress_view() {
    let client = FixtureClient::sample();
    let mut app = home_app(&client, 1).await;
    insta::assert_snapshot!(render_to_string(&mut app, 110, 12));
}

#[tokio::test]
async fn inbox_view() {
    let client = FixtureClient::sample();
    let mut app = home_app(&client, 2).await;
    insta::assert_snapshot!(render_to_string(&mut app, 110, 10));
}

#[tokio::test]
async fn issue_detail() {
    let client = FixtureClient::sample();
    let mut app = App::new();
    app.screen = Screen::Detail;
    app.detail = client.issue_detail("ENG-4653").await.unwrap();
    insta::assert_snapshot!(render_to_string(&mut app, 110, 26));
}

#[tokio::test]
async fn loading_placeholder() {
    let mut app = App::new();
    app.loading = true;
    insta::assert_snapshot!(render_to_string(&mut app, 110, 8));
}
