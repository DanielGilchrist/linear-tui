use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use linear_tui::api::fixture::FixtureClient;
use linear_tui::api::LinearApi;
use linear_tui::tui::app::{App, Pane, Screen};
use linear_tui::tui::message::Command;
use linear_tui::tui::update::{apply, handle_key};
use linear_tui::tui::message::Message;

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[tokio::test]
async fn in_progress_filter_returns_only_started() {
    let client = FixtureClient::sample();
    let issues = client
        .issues(&linear_tui::api::IssueFilter::in_progress_mine())
        .await
        .unwrap();
    assert_eq!(issues.len(), 3);
    assert!(issues.iter().all(|i| i.state.state_type == "started"));
}

#[test]
fn moving_in_sidebar_switches_view_and_requests_load() {
    let mut app = App::new();
    app.pane = Pane::Sidebar;

    let commands = handle_key(&mut app, press(KeyCode::Char('j')));

    assert_eq!(app.active_view_index(), 1);
    assert!(app.loading);
    match commands.as_slice() {
        [Command::LoadIssues { view: 1, .. }] => {}
        other => panic!("expected LoadIssues for view 1, got {other:?}"),
    }
}

#[test]
fn number_key_jumps_to_view() {
    let mut app = App::new();

    let commands = handle_key(&mut app, press(KeyCode::Char('3')));

    assert_eq!(app.active_view_index(), 2);
    match commands.as_slice() {
        [Command::LoadInbox { view: 2 }] => {}
        other => panic!("expected LoadInbox for view 2, got {other:?}"),
    }
}

#[test]
fn enter_on_issue_opens_detail() {
    let mut app = App::new();
    app.pane = Pane::Main;
    app.issues = vec![sample_issue("i1", "ENG-1")];
    app.list_state.select(Some(0));

    let commands = handle_key(&mut app, press(KeyCode::Enter));

    assert_eq!(app.screen, Screen::Detail);
    assert!(app.loading);
    match commands.as_slice() {
        [Command::LoadDetail(id)] if id == "i1" => {}
        other => panic!("expected LoadDetail(i1), got {other:?}"),
    }
}

#[test]
fn stale_issue_results_are_ignored() {
    let mut app = App::new();
    app.view_state.select(Some(2));
    app.loading = true;

    apply(
        &mut app,
        Message::IssuesLoaded {
            view: 0,
            issues: vec![sample_issue("i1", "ENG-1")],
        },
    );

    assert!(app.issues.is_empty());
    assert!(app.loading);
}

#[test]
fn esc_from_detail_returns_home() {
    let mut app = App::new();
    app.screen = Screen::Detail;

    handle_key(&mut app, press(KeyCode::Esc));

    assert_eq!(app.screen, Screen::Home);
    assert!(app.detail.is_none());
}

fn sample_issue(id: &str, identifier: &str) -> linear_tui::api::IssueSummary {
    linear_tui::api::IssueSummary {
        id: id.into(),
        identifier: identifier.into(),
        title: Some("Title".into()),
        state: linear_tui::api::WorkflowState {
            name: "Todo".into(),
            state_type: "unstarted".into(),
        },
        priority: 0,
        assignee: None,
        labels: vec![],
    }
}
