use linear_tui::api::fixture::FixtureClient;
use linear_tui::api::{Credential, LinearApi, Timestamp};
use linear_tui::store::Account;
use linear_tui::tui::app::App;
use linear_tui::tui::cache::Remote;
use linear_tui::tui::feed::{Feed, FeedKey};
use linear_tui::tui::render::theme::{self, ColourMode};
use linear_tui::tui::render_styled_to_string;
use linear_tui::tui::view::ViewKind;

async fn monochrome_app(client: &FixtureClient) -> App {
    theme::init(ColourMode::Monochrome);

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
async fn styled_monochrome_assigned_view() {
    let client = FixtureClient::sample();
    let mut app = monochrome_app(&client).await;

    let frame = render_styled_to_string(&mut app, 110, 16);

    assert!(
        !frame.contains("Rgb("),
        "NO_COLOR collapses even the label-chip colours"
    );
    for colour in ["Yellow", "Blue", "Cyan", "Red", "Green", "Magenta"] {
        assert!(
            !frame.contains(&format!("Some({colour})")),
            "NO_COLOR leaves no palette colour behind, found {colour}"
        );
    }
    assert!(
        frame.contains("REVERSED"),
        "modifiers survive, so the selection is still visible"
    );

    insta::with_settings!({prepend_module_to_snapshot => false}, {
        insta::assert_snapshot!("render__styled_monochrome_assigned_view", frame);
    });
}
