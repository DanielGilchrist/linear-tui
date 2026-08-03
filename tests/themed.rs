use linear_tui::api::fixture::FixtureClient;
use linear_tui::api::{Credential, LinearApi, Timestamp};
use linear_tui::store::Account;
use linear_tui::tui::app::App;
use linear_tui::tui::cache::Remote;
use linear_tui::tui::feed::{Feed, FeedKey};
use linear_tui::tui::render::theme::{self, ColourMode, Overrides};
use linear_tui::tui::render_styled_to_string;
use linear_tui::tui::view::ViewKind;

async fn themed_app(client: &FixtureClient) -> App {
    theme::init(ColourMode::Ansi);
    theme::init_overrides(
        Overrides::parse(
            r##"{
                "accent": "#e69875",
                "dim": "#3b4252",
                "selection_bg": "#2f3540"
            }"##,
        )
        .unwrap(),
    );

    let mut app = App::new();
    app.session.upsert_account(Account {
        workspace_key: "ws".into(),
        org_name: "Test".into(),
        credential: Credential::PersonalKey("k".into()),
    });
    assert!(app.session.activate("ws"));

    app.now = Timestamp::from("2026-07-16T21:00:00Z");
    app.workspace.session = Remote::ready(client.session().await.unwrap(), app.now);
    app.ui.view_state.select(Some(0));

    if let ViewKind::Issues(filter) = &app.active_view().kind {
        let page = client.issues(&filter.clone(), None).await.unwrap();
        app.workspace
            .feeds
            .insert(FeedKey::Issues(filter.clone()), Feed::ready(page, app.now));
    }

    app
}

#[tokio::test]
async fn styled_themed_assigned_view() {
    let client = FixtureClient::sample();
    let mut app = themed_app(&client).await;

    let frame = render_styled_to_string(&mut app, 110, 16);

    assert!(
        frame.contains("Rgb(230, 152, 117)"),
        "the accent override colours the focused border"
    );
    assert!(
        frame.contains("Rgb(59, 66, 82)"),
        "the dim override replaces the DIM modifier with a real colour"
    );
    assert!(
        frame.contains("Rgb(47, 53, 64)"),
        "the selection override paints a background instead of reversing"
    );
}
