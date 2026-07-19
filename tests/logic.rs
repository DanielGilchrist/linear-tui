use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use linear_tui::api::fixture::FixtureClient;
use linear_tui::api::LinearApi;
use linear_tui::tui::app::{App, Focus, PickerItem, PickerKind};
use linear_tui::tui::message::{Command, Message};
use linear_tui::tui::update::{apply, handle_key};

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
fn bracket_cycles_to_next_view_and_requests_load() {
    let mut app = App::new();

    let commands = handle_key(&mut app, press(KeyCode::Char(']')));

    assert_eq!(app.active_view_index(), 1);
    assert_eq!(app.focus, Focus::MyWork);
    assert!(app.loading);
    match commands.as_slice() {
        [Command::LoadIssues { view: 1, .. }] => {}
        other => panic!("expected LoadIssues for view 1, got {other:?}"),
    }
}

#[test]
fn number_key_jumps_to_panel() {
    let mut app = App::new();

    let commands = handle_key(&mut app, press(KeyCode::Char('2')));

    assert_eq!(app.focus, Focus::Stub(0));
    assert!(commands.is_empty());
}

#[test]
fn tab_cycles_from_my_work_into_the_stack() {
    let mut app = App::new();

    handle_key(&mut app, press(KeyCode::Tab));

    assert_eq!(app.focus, Focus::Stub(0));
}

#[test]
fn brackets_do_nothing_off_my_work() {
    let mut app = App::new();
    app.focus = Focus::Stub(0);

    let commands = handle_key(&mut app, press(KeyCode::Char(']')));

    assert_eq!(app.active_view_index(), 0);
    assert!(commands.is_empty());
}

#[test]
fn enter_on_issue_opens_detail() {
    let mut app = list_app_with_issue();

    let commands = handle_key(&mut app, press(KeyCode::Enter));

    assert_eq!(app.focus, Focus::Detail);
    assert!(app.detail_loading);
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
fn esc_from_detail_focuses_my_work() {
    let mut app = detail_app();

    handle_key(&mut app, press(KeyCode::Esc));

    assert_eq!(app.focus, Focus::MyWork);
}

#[test]
fn status_action_requires_an_opened_issue() {
    let mut app = list_app_with_issue();

    let commands = handle_key(&mut app, press(KeyCode::Char('s')));

    assert!(app.picker().is_none());
    assert!(commands.is_empty());
}

#[test]
fn s_opens_status_picker_once_issue_is_loaded() {
    let mut app = detail_app();

    let commands = handle_key(&mut app, press(KeyCode::Char('s')));

    assert_eq!(app.picker().map(|p| p.kind), Some(PickerKind::Status));
    match commands.as_slice() {
        [Command::LoadStates { team_id }] if team_id == "t_pizza" => {}
        other => panic!("expected LoadStates for t_pizza, got {other:?}"),
    }
}

#[test]
fn picker_enter_opens_confirmation_then_applies() {
    let mut app = detail_app();
    handle_key(&mut app, press(KeyCode::Char('s')));
    apply(
        &mut app,
        Message::PickerLoaded(vec![PickerItem {
            id: "s_done".into(),
            label: "Done".into(),
            hint: "completed".into(),
        }]),
    );

    let no_commands = handle_key(&mut app, press(KeyCode::Enter));
    assert!(app.picker().is_none());
    assert!(app.confirm().is_some());
    assert!(no_commands.is_empty());

    let commands = handle_key(&mut app, press(KeyCode::Char('y')));
    assert!(app.confirm().is_none());
    match commands.as_slice() {
        [Command::UpdateIssue { id, update }]
            if id == "i1" && update.state_id.as_deref() == Some("s_done") => {}
        other => panic!("expected UpdateIssue with state_id, got {other:?}"),
    }
}

#[test]
fn confirmation_cancel_does_not_write() {
    let mut app = detail_app();
    handle_key(&mut app, press(KeyCode::Char('s')));
    apply(
        &mut app,
        Message::PickerLoaded(vec![PickerItem {
            id: "s_done".into(),
            label: "Done".into(),
            hint: "completed".into(),
        }]),
    );
    handle_key(&mut app, press(KeyCode::Enter));

    let commands = handle_key(&mut app, press(KeyCode::Char('n')));

    assert!(app.confirm().is_none());
    assert!(commands.is_empty());
}

#[test]
fn o_opens_url_from_highlighted_issue() {
    let mut app = list_app_with_issue();
    let commands = handle_key(&mut app, press(KeyCode::Char('o')));

    match commands.as_slice() {
        [Command::OpenUrl(url)] if url.contains("DAN2-7") => {}
        other => panic!("expected OpenUrl, got {other:?}"),
    }
}

#[test]
fn y_copies_url_from_highlighted_issue() {
    let mut app = list_app_with_issue();
    let commands = handle_key(&mut app, press(KeyCode::Char('y')));

    assert!(app.status.is_some());
    match commands.as_slice() {
        [Command::CopyToClipboard(url)] if url.contains("DAN2-7") => {}
        other => panic!("expected CopyToClipboard, got {other:?}"),
    }
}

#[test]
fn esc_closes_picker_without_updating() {
    let mut app = detail_app();
    handle_key(&mut app, press(KeyCode::Char('a')));
    assert!(app.picker().is_some());

    let commands = handle_key(&mut app, press(KeyCode::Esc));

    assert!(app.picker().is_none());
    assert!(commands.is_empty());
}

#[test]
fn open_and_yank_do_nothing_without_a_selected_issue() {
    let mut app = App::new();
    app.focus = Focus::MyWork;
    app.issues = vec![];
    app.list_state.select(None);

    for key in ['o', 'y'] {
        let commands = handle_key(&mut app, press(KeyCode::Char(key)));
        assert!(commands.is_empty(), "{key} should not act without a selection");
    }
}

fn list_app_with_issue() -> App {
    let mut app = App::new();
    app.focus = Focus::MyWork;
    app.issues = vec![sample_issue("i1", "DAN2-7")];
    app.list_state.select(Some(0));
    app
}

fn detail_app() -> App {
    let mut app = list_app_with_issue();
    app.focus = Focus::Detail;
    app.detail = Some(sample_detail("i1", "DAN2-7"));
    app
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
        url: format!("https://linear.app/dans-donuts/issue/{identifier}"),
        branch_name: format!("dan/{}", identifier.to_lowercase()),
        team_id: "t_pizza".into(),
    }
}

fn sample_detail(id: &str, identifier: &str) -> linear_tui::api::IssueDetail {
    linear_tui::api::IssueDetail {
        id: id.into(),
        identifier: identifier.into(),
        title: Some("Title".into()),
        description: Some("Body".into()),
        url: format!("https://linear.app/dans-donuts/issue/{identifier}"),
        state: linear_tui::api::WorkflowState {
            name: "Todo".into(),
            state_type: "unstarted".into(),
        },
        priority: 0,
        assignee: None,
        labels: vec![],
        comments: vec![],
        branch_name: format!("dan/{}", identifier.to_lowercase()),
        team_id: "t_pizza".into(),
    }
}
