use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use linear_tui::api::fixture::FixtureClient;
use linear_tui::api::{LinearApi, Timestamp};
use linear_tui::tui::app::App;
use linear_tui::tui::cache::Remote;
use linear_tui::tui::feed::{Feed, FeedKey, FeedRequest};
use linear_tui::tui::focus::{DetailView, Focus, LeftPanel};
use linear_tui::tui::message::Message;
use linear_tui::tui::render_to_string;
use linear_tui::tui::update::{apply, handle_key};
use linear_tui::tui::view::ViewKind;

async fn home_app(client: &FixtureClient, view: usize) -> App {
    let mut app = App::new();
    app.now = Timestamp::from("2026-07-16T21:00:00Z").epoch();
    app.workspace.session = client.session().await.ok();
    app.view_state.select(Some(view));
    match &app.active_view().kind {
        ViewKind::Issues(filter) => {
            let page = client.issues(&filter.clone(), None).await.unwrap();
            app.workspace
                .feeds
                .insert(FeedKey::Issues(filter.clone()), Feed::ready(page, app.now));
        }
        ViewKind::Inbox => {
            let page = client.notifications(None).await.unwrap();
            app.workspace.inbox = Feed::ready(page, app.now);
        }
    }
    app
}

async fn load_view(app: &mut App, client: &FixtureClient, id: &str) {
    let page = client.custom_view_issues(id, None).await.unwrap();
    apply(
        app,
        Message::FeedLoaded {
            key: FeedKey::View(id.to_string()),
            request: FeedRequest::Refresh,
            page,
        },
    );
}

async fn opened_detail_app(client: &FixtureClient) -> App {
    let mut app = home_app(client, 0).await;
    app.focus = Focus::Detail(LeftPanel::MyWork, DetailView::Reading);
    if let Some(detail) = client.issue_detail("DAN2-7").await.unwrap() {
        app.workspace.detail = Remote::ready(detail, app.now);
    }
    app
}

async fn saved_views_app(client: &FixtureClient) -> App {
    let mut app = App::new();
    app.now = Timestamp::from("2026-07-16T21:00:00Z").epoch();
    app.workspace.session = client.session().await.ok();
    app.focus = Focus::SavedViews;
    apply(
        &mut app,
        Message::CustomViewsLoaded(client.custom_views().await.unwrap()),
    );
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
async fn saved_views_list() {
    let client = FixtureClient::sample();
    let mut app = saved_views_app(&client).await;
    let id = app
        .workspace
        .saved_views
        .selected_view()
        .unwrap()
        .id
        .clone();
    load_view(&mut app, &client, &id).await;
    insta::assert_snapshot!(render_to_string(&mut app, 110, 16));
}

async fn open_view_app(client: &FixtureClient) -> App {
    let mut app = saved_views_app(client).await;
    let id = app
        .workspace
        .saved_views
        .selected_view()
        .unwrap()
        .id
        .clone();
    handle_key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    load_view(&mut app, client, &id).await;
    app
}

#[tokio::test]
async fn view_in_right_pane() {
    let client = FixtureClient::sample();
    let mut app = open_view_app(&client).await;
    insta::assert_snapshot!(render_to_string(&mut app, 110, 26));
}

#[tokio::test]
async fn view_zoomed() {
    let client = FixtureClient::sample();
    let mut app = open_view_app(&client).await;
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE),
    );
    insta::assert_snapshot!(render_to_string(&mut app, 110, 26));
}

#[tokio::test]
async fn a_truncated_view_marks_the_count_with_a_plus() {
    let client = FixtureClient::sample();
    let mut app = saved_views_app(&client).await;
    let id = app
        .workspace
        .saved_views
        .selected_view()
        .unwrap()
        .id
        .clone();
    handle_key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let items = client.custom_view_issues(&id, None).await.unwrap().items;
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
}

#[tokio::test]
async fn view_grouped_by_priority() {
    let client = FixtureClient::sample();
    let mut app = open_view_app(&client).await;
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
}

#[tokio::test]
async fn threaded_comments_and_timestamps() {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await;
    insta::assert_snapshot!(render_to_string(&mut app, 90, 46));
}

#[tokio::test]
async fn comments_mode_scrolls_the_selected_comment_to_the_top() {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await;

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE),
    );

    insta::assert_snapshot!(render_to_string(&mut app, 90, 20));
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
    let key = app.active_feed_key().unwrap();
    app.workspace
        .feeds
        .get_or_default(&key)
        .begin(&FeedRequest::Refresh);
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
    apply(
        &mut app,
        Message::StatesLoaded {
            team_id: "t_pizza".into(),
            states,
        },
    );

    insta::assert_snapshot!(render_to_string(&mut app, 100, 20));
}

#[tokio::test]
async fn assign_picker_overlay() {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await;

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
    );
    let members = client.team_members("t_pizza").await.unwrap();
    apply(
        &mut app,
        Message::MembersLoaded {
            team_id: "t_pizza".into(),
            members,
        },
    );

    insta::assert_snapshot!(render_to_string(&mut app, 100, 20));
}

#[tokio::test]
async fn comment_editor_overlay() {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await;

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
}

#[tokio::test]
async fn mention_autocomplete_popup() {
    let client = FixtureClient::sample();
    let mut app = opened_detail_app(&client).await;

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE),
    );
    let members = client.team_members("t_pizza").await.unwrap();
    apply(
        &mut app,
        Message::MembersLoaded {
            team_id: "t_pizza".into(),
            members,
        },
    );
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('@'), KeyModifiers::NONE),
    );

    insta::assert_snapshot!(render_to_string(&mut app, 90, 24));
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
    let page = client.search_issues("oven", None).await.unwrap();
    apply(
        &mut app,
        Message::FeedLoaded {
            key: FeedKey::Search("oven".to_string()),
            request: FeedRequest::Refresh,
            page,
        },
    );

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
    apply(
        &mut app,
        Message::StatesLoaded {
            team_id: "t_pizza".into(),
            states,
        },
    );
    handle_key(&mut app, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    handle_key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    insta::assert_snapshot!(render_to_string(&mut app, 100, 20));
}
