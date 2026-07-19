use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use linear_tui::api::fixture::FixtureClient;
use linear_tui::api::LinearApi;
use linear_tui::tui::app::App;
use linear_tui::tui::focus::{Focus, LeftPanel, Reveal};
use linear_tui::tui::message::{Command, Message};
use linear_tui::tui::overlay::{InputPurpose, PickerItem, PickerKind, Search};
use linear_tui::tui::status::Status;
use linear_tui::tui::update::{apply, handle_key};

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
}

#[tokio::test]
async fn in_progress_filter_returns_only_started() {
    let client = FixtureClient::sample();
    let issues = client
        .issues(&linear_tui::api::IssueFilter::in_progress_mine())
        .await
        .unwrap();
    assert_eq!(issues.len(), 3);
    assert!(issues
        .iter()
        .all(|i| i.state.state_type == linear_tui::api::StateType::Started));
}

#[test]
fn bracket_cycles_to_next_view_and_requests_load() {
    let mut app = App::new();

    let commands = handle_key(&mut app, press(KeyCode::Char(']')));

    assert_eq!(app.active_view_index(), 1);
    assert_eq!(app.focus, Focus::MyWork);
    assert!(app.loading);
    match commands {
        Some(Command::LoadIssues { view: 1, .. }) => {}
        other => panic!("expected LoadIssues for view 1, got {other:?}"),
    }
}

#[test]
fn question_mark_toggles_the_menu_overlay() {
    let mut app = App::new();

    handle_key(&mut app, press(KeyCode::Char('?')));
    assert!(app.menu().is_some());

    handle_key(&mut app, press(KeyCode::Esc));
    assert!(app.menu().is_none());
}

#[test]
fn menu_enter_runs_the_selected_action() {
    let mut app = detail_app();

    handle_key(&mut app, press(KeyCode::Char('?')));
    assert!(app.menu().is_some());

    let commands = handle_key(&mut app, press(KeyCode::Enter));

    assert!(app.menu().is_none());
    match commands {
        Some(Command::LoadStates { .. }) => {}
        other => panic!("expected the first Detail action (status) to run, got {other:?}"),
    }
}

#[test]
fn tab_in_menu_jumps_between_sections() {
    let mut app = detail_app();
    handle_key(&mut app, press(KeyCode::Char('?')));

    let first = app.menu().and_then(|m| m.selected_action());
    handle_key(&mut app, press(KeyCode::Tab));
    let after_tab = app.menu().and_then(|m| m.selected_action());

    assert!(app.menu().is_some());
    assert_ne!(first, after_tab);
    assert_eq!(after_tab, Some(linear_tui::tui::action::Action::GoPrefix));
}

#[test]
fn number_key_jumps_to_panel() {
    let mut app = App::new();

    let commands = handle_key(&mut app, press(KeyCode::Char('3')));

    assert_eq!(app.focus, Focus::Stub(0));
    assert!(commands.is_none());
}

#[test]
fn tab_cycles_from_my_work_into_the_stack() {
    let mut app = App::new();

    handle_key(&mut app, press(KeyCode::Tab));

    assert_eq!(app.focus, Focus::Recent);
}

#[test]
fn recent_loaded_populates_the_panel() {
    let mut app = App::new();

    apply(
        &mut app,
        Message::RecentLoaded(vec![
            sample_issue("i1", "DAN-1"),
            sample_issue("i2", "DAN-2"),
        ]),
    );

    assert_eq!(app.recently_viewed.len(), 2);
}

#[test]
fn clearing_recently_viewed_confirms_first() {
    let mut app = App::new();
    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(sample_detail("i1", "DAN-1")),
            reveal: Reveal::Top,
        },
    );
    handle_key(&mut app, press(KeyCode::Char('2')));
    assert_eq!(app.focus, Focus::Recent);

    handle_key(&mut app, press(KeyCode::Char('x')));
    assert!(app.confirm().is_some());

    let command = handle_key(&mut app, press(KeyCode::Char('y')));
    match command {
        Some(Command::ClearRecent) => {}
        other => panic!("expected ClearRecent, got {other:?}"),
    }

    apply(&mut app, Message::RecentCleared);
    assert!(app.recently_viewed.is_empty());
    assert!(
        !app.loading,
        "confirming a non-fetch command must not leave the view spinner stuck"
    );
}

#[test]
fn clearing_does_nothing_off_the_recent_panel() {
    let mut app = list_app_with_issue();
    app.recently_viewed = vec![sample_issue("i1", "DAN-1")];

    handle_key(&mut app, press(KeyCode::Char('x')));

    assert!(app.confirm().is_none());
}

#[test]
fn brackets_do_nothing_off_my_work() {
    let mut app = App::new();
    app.focus = Focus::Stub(0);

    let commands = handle_key(&mut app, press(KeyCode::Char(']')));

    assert_eq!(app.active_view_index(), 0);
    assert!(commands.is_none());
}

#[test]
fn enter_on_issue_opens_detail() {
    let mut app = list_app_with_issue();

    let commands = handle_key(&mut app, press(KeyCode::Enter));

    assert!(app.focus.is_detail());
    assert!(app.detail_loading);
    match commands {
        Some(Command::LoadDetail { id, .. }) if id == "i1" => {}
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
    assert!(commands.is_none());
}

#[test]
fn s_opens_status_picker_once_issue_is_loaded() {
    let mut app = detail_app();

    let commands = handle_key(&mut app, press(KeyCode::Char('s')));

    assert_eq!(app.picker().map(|p| p.kind), Some(PickerKind::Status));
    match commands {
        Some(Command::LoadStates { team_id }) if team_id == "t_pizza" => {}
        other => panic!("expected LoadStates for t_pizza, got {other:?}"),
    }
}

#[test]
fn comment_action_requires_an_opened_issue() {
    let mut app = list_app_with_issue();

    let commands = handle_key(&mut app, press(KeyCode::Char('c')));

    assert!(app.editor().is_none());
    assert!(commands.is_none());
}

#[test]
fn c_opens_the_comment_editor_once_issue_is_loaded() {
    let mut app = detail_app();

    let commands = handle_key(&mut app, press(KeyCode::Char('c')));

    assert!(app.editor().is_some());
    assert!(commands.is_none());
}

#[test]
fn enter_inserts_a_newline_and_does_not_submit() {
    let mut app = detail_app();
    handle_key(&mut app, press(KeyCode::Char('c')));

    handle_key(&mut app, press(KeyCode::Char('a')));
    let command = handle_key(&mut app, press(KeyCode::Enter));
    handle_key(&mut app, press(KeyCode::Char('b')));

    assert!(command.is_none());
    assert_eq!(app.editor().map(|e| e.text()), Some("a\nb".to_string()));
}

#[test]
fn ctrl_s_posts_the_multiline_comment_for_the_open_issue() {
    let mut app = detail_app();
    handle_key(&mut app, press(KeyCode::Char('c')));
    handle_key(&mut app, press(KeyCode::Char('a')));
    handle_key(&mut app, press(KeyCode::Enter));
    handle_key(&mut app, press(KeyCode::Char('b')));

    let command = handle_key(&mut app, ctrl('s'));

    match command {
        Some(Command::CreateComment {
            issue_id,
            body,
            parent_id,
        }) => {
            assert_eq!(issue_id, "i1");
            assert_eq!(body, "a\nb");
            assert_eq!(parent_id, None);
        }
        other => panic!("expected CreateComment, got {other:?}"),
    }
    assert!(app.editor().is_none());
}

#[test]
fn an_empty_comment_posts_nothing() {
    let mut app = detail_app();
    handle_key(&mut app, press(KeyCode::Char('c')));

    let command = handle_key(&mut app, ctrl('s'));

    assert!(command.is_none());
}

#[test]
fn comment_posted_refetches_the_thread_and_reveals_the_bottom() {
    let mut app = detail_app();

    let command = apply(&mut app, Message::CommentPosted { id: "i1".into() });

    assert!(app.detail_loading);
    match command {
        Some(Command::LoadDetail {
            id,
            reveal: Reveal::Bottom,
        }) if id == "i1" => {}
        other => panic!("expected LoadDetail for i1 revealing the bottom, got {other:?}"),
    }
}

#[test]
fn a_bottom_reveal_scrolls_to_the_new_comment() {
    let mut app = detail_app();

    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(sample_detail("i1", "DAN-1")),
            reveal: Reveal::Bottom,
        },
    );

    assert_eq!(app.scroll_position, usize::MAX);
}

#[test]
fn opening_a_detail_starts_at_the_top() {
    let mut app = detail_app();
    app.scroll_position = 42;

    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(sample_detail("i1", "DAN-1")),
            reveal: Reveal::Top,
        },
    );

    assert_eq!(app.scroll_position, 0);
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
    assert!(no_commands.is_none());

    let commands = handle_key(&mut app, press(KeyCode::Char('y')));
    assert!(app.confirm().is_none());
    match commands {
        Some(Command::UpdateIssue { id, update })
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
    assert!(commands.is_none());
}

#[test]
fn o_opens_url_from_highlighted_issue() {
    let mut app = list_app_with_issue();
    let commands = handle_key(&mut app, press(KeyCode::Char('o')));

    match commands {
        Some(Command::OpenUrl(url)) if url.contains("DAN2-7") => {}
        other => panic!("expected OpenUrl, got {other:?}"),
    }
}

#[test]
fn y_copies_url_from_highlighted_issue() {
    let mut app = list_app_with_issue();
    let commands = handle_key(&mut app, press(KeyCode::Char('y')));

    assert!(app.status.is_some());
    match commands {
        Some(Command::CopyToClipboard(url)) if url.contains("DAN2-7") => {}
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
    assert!(commands.is_none());
}

#[test]
fn open_and_yank_do_nothing_without_a_selected_issue() {
    let mut app = App::new();
    app.focus = Focus::MyWork;
    app.issues = vec![];
    app.list_state.select(None);

    for key in ['o', 'y'] {
        let commands = handle_key(&mut app, press(KeyCode::Char(key)));
        assert!(
            commands.is_none(),
            "{key} should not act without a selection"
        );
    }
}

#[test]
fn go_prefix_then_g_jumps_to_the_top() {
    let mut app = list_app_with_issues();
    app.list_state.select(Some(2));

    handle_key(&mut app, press(KeyCode::Char('g')));
    assert!(app.prefix().is_some());

    let commands = handle_key(&mut app, press(KeyCode::Char('g')));

    assert!(app.prefix().is_none());
    assert!(commands.is_none());
    assert_eq!(app.list_state.selected(), Some(0));
}

#[test]
fn capital_g_jumps_to_the_bottom() {
    let mut app = list_app_with_issues();
    app.list_state.select(Some(0));

    handle_key(&mut app, press(KeyCode::Char('G')));

    assert_eq!(app.list_state.selected(), Some(2));
}

#[test]
fn go_prefix_cancels_on_an_unbound_key() {
    let mut app = App::new();

    handle_key(&mut app, press(KeyCode::Char('g')));
    assert!(app.prefix().is_some());

    let commands = handle_key(&mut app, press(KeyCode::Char('z')));

    assert!(app.prefix().is_none());
    assert!(commands.is_none());
}

#[test]
fn gi_opens_a_jump_input_that_loads_the_referenced_issue() {
    let mut app = App::new();

    handle_key(&mut app, press(KeyCode::Char('g')));
    handle_key(&mut app, press(KeyCode::Char('i')));
    assert_eq!(app.input().map(|i| i.purpose), Some(InputPurpose::Jump));

    for c in "dan2-7".chars() {
        handle_key(&mut app, press(KeyCode::Char(c)));
    }
    let commands = handle_key(&mut app, press(KeyCode::Enter));

    assert!(app.focus.is_detail());
    assert!(app.detail_loading);
    assert!(app.input().is_none());
    match commands {
        Some(Command::LoadDetail { id, .. }) if id == "DAN2-7" => {}
        other => panic!("expected LoadDetail(DAN2-7), got {other:?}"),
    }
}

#[test]
fn input_backspace_edits_the_buffer() {
    let mut app = App::new();
    handle_key(&mut app, press(KeyCode::Char('g')));
    handle_key(&mut app, press(KeyCode::Char('i')));

    for c in "ovenX".chars() {
        handle_key(&mut app, press(KeyCode::Char(c)));
    }
    handle_key(&mut app, press(KeyCode::Backspace));

    assert_eq!(app.input().map(|i| i.buffer.as_str()), Some("oven"));
}

#[test]
fn esc_cancels_the_input_without_a_command() {
    let mut app = App::new();
    handle_key(&mut app, press(KeyCode::Char('g')));
    handle_key(&mut app, press(KeyCode::Char('s')));
    assert!(app.input().is_some());

    let commands = handle_key(&mut app, press(KeyCode::Esc));

    assert!(app.input().is_none());
    assert!(commands.is_none());
}

#[test]
fn slash_filters_the_current_list_in_place() {
    let mut app = list_app_with_issues();

    handle_key(&mut app, press(KeyCode::Char('/')));
    assert!(app.find().is_some());

    for c in "dan-2".chars() {
        handle_key(&mut app, press(KeyCode::Char(c)));
    }
    assert_eq!(app.list_state.selected(), Some(1));

    handle_key(&mut app, press(KeyCode::Enter));
    assert!(app.find().is_none());
    assert_eq!(app.find_query.as_deref(), Some("dan-2"));
}

#[test]
fn n_and_capital_n_cycle_matches() {
    let mut app = list_app_with_issues();
    app.issues.push(sample_issue("i4", "DAN-2B"));
    app.find_query = Some("dan-2".into());
    app.list_state.select(Some(0));

    handle_key(&mut app, press(KeyCode::Char('n')));
    assert_eq!(app.list_state.selected(), Some(1));

    handle_key(&mut app, press(KeyCode::Char('n')));
    assert_eq!(app.list_state.selected(), Some(3));

    handle_key(&mut app, press(KeyCode::Char('N')));
    assert_eq!(app.list_state.selected(), Some(1));
}

#[test]
fn esc_cancels_find_and_restores_selection() {
    let mut app = list_app_with_issues();
    app.list_state.select(Some(2));

    handle_key(&mut app, press(KeyCode::Char('/')));
    for c in "dan-1".chars() {
        handle_key(&mut app, press(KeyCode::Char(c)));
    }
    assert_eq!(app.list_state.selected(), Some(0));

    handle_key(&mut app, press(KeyCode::Esc));
    assert!(app.find().is_none());
    assert_eq!(app.list_state.selected(), Some(2));
}

#[test]
fn find_matches_on_state_name_and_esc_exits_search() {
    let mut app = list_app_with_issues();
    app.issues[1].state.name = "In Progress".into();

    handle_key(&mut app, press(KeyCode::Char('/')));
    for c in "in progress".chars() {
        handle_key(&mut app, press(KeyCode::Char(c)));
    }
    handle_key(&mut app, press(KeyCode::Enter));

    assert_eq!(app.find_query.as_deref(), Some("in progress"));
    assert_eq!(app.list_state.selected(), Some(1));

    handle_key(&mut app, press(KeyCode::Esc));
    assert!(app.find_query.is_none());
}

#[test]
fn gg_and_capital_g_navigate_inside_the_menu() {
    let mut app = detail_app();
    handle_key(&mut app, press(KeyCode::Char('?')));
    let first = app.menu().and_then(|m| m.selected_action());

    handle_key(&mut app, press(KeyCode::Char('G')));
    let last = app.menu().and_then(|m| m.selected_action());
    assert!(app.menu().is_some());
    assert_ne!(first, last);

    handle_key(&mut app, press(KeyCode::Char('g')));
    assert!(app.prefix().is_some());
    handle_key(&mut app, press(KeyCode::Char('g')));
    assert!(app.menu().is_some());
    assert_eq!(app.menu().and_then(|m| m.selected_action()), first);
}

#[test]
fn gs_searches_then_enter_opens_a_result() {
    let mut app = App::new();

    handle_key(&mut app, press(KeyCode::Char('g')));
    handle_key(&mut app, press(KeyCode::Char('s')));
    assert_eq!(app.input().map(|i| i.purpose), Some(InputPurpose::Search));

    for c in "oven".chars() {
        handle_key(&mut app, press(KeyCode::Char(c)));
    }
    let search = handle_key(&mut app, press(KeyCode::Enter));

    assert!(app.search().is_some());
    match search {
        Some(Command::Search(term)) if term == "oven" => {}
        other => panic!("expected Search(oven), got {other:?}"),
    }

    apply(
        &mut app,
        Message::SearchResults(vec![sample_issue("i9", "DAN2-7")]),
    );
    assert_eq!(app.search().map(|s| s.results.len()), Some(1));

    let open = handle_key(&mut app, press(KeyCode::Enter));

    assert!(app.focus.is_detail());
    assert!(app.search().is_none());
    match open {
        Some(Command::LoadDetail { id, .. }) if id == "i9" => {}
        other => panic!("expected LoadDetail(i9), got {other:?}"),
    }
}

#[test]
fn esc_from_a_search_result_returns_to_the_results() {
    let mut app = App::new();
    handle_key(&mut app, press(KeyCode::Char('g')));
    handle_key(&mut app, press(KeyCode::Char('s')));
    for c in "oven".chars() {
        handle_key(&mut app, press(KeyCode::Char(c)));
    }
    handle_key(&mut app, press(KeyCode::Enter));
    apply(
        &mut app,
        Message::SearchResults(vec![
            sample_issue("i8", "DAN-1"),
            sample_issue("i9", "DAN-2"),
        ]),
    );

    handle_key(&mut app, press(KeyCode::Enter));
    assert!(app.focus.is_detail());
    assert!(app.search().is_none());

    handle_key(&mut app, press(KeyCode::Esc));
    assert_eq!(app.search().map(|s| s.results.len()), Some(2));
}

#[test]
fn esc_from_a_list_opened_detail_goes_home_not_search() {
    let mut app = list_app_with_issue();
    app.search_return = Some(Search::new("oven".into()));

    handle_key(&mut app, press(KeyCode::Enter));
    assert!(app.search_return.is_none());

    handle_key(&mut app, press(KeyCode::Esc));
    assert_eq!(app.focus, Focus::MyWork);
    assert!(app.search().is_none());
}

#[test]
fn transient_status_clears_on_the_next_key() {
    let mut app = list_app_with_issue();
    app.status = Some(Status::Cancelled);

    handle_key(&mut app, press(KeyCode::Char('j')));

    assert!(app.status.is_none());
}

#[test]
fn history_boundary_sets_no_status() {
    let mut app = App::new();
    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(sample_detail("i1", "DAN-1")),
            reveal: Reveal::Top,
        },
    );

    let command = handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL),
    );

    assert!(command.is_none());
    assert!(app.status.is_none());
}

#[test]
fn opening_a_detail_keeps_the_source_panel_expanded() {
    let mut app = list_app_with_issue();

    handle_key(&mut app, press(KeyCode::Enter));

    assert_eq!(app.focus, Focus::Detail(LeftPanel::MyWork));
}

#[test]
fn opening_from_recently_viewed_keeps_that_panel_expanded() {
    let mut app = App::new();
    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(sample_detail("i1", "DAN-1")),
            reveal: Reveal::Top,
        },
    );

    handle_key(&mut app, press(KeyCode::Char('2')));
    assert_eq!(app.focus, Focus::Recent);

    handle_key(&mut app, press(KeyCode::Enter));

    assert_eq!(app.focus, Focus::Detail(LeftPanel::Recent));
}

#[test]
fn tab_and_shift_tab_walk_history_in_the_detail_pane() {
    let mut app = App::new();
    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(sample_detail("i1", "DAN-1")),
            reveal: Reveal::Top,
        },
    );
    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(sample_detail("i2", "DAN-2")),
            reveal: Reveal::Top,
        },
    );
    app.focus = Focus::Detail(LeftPanel::MyWork);

    let back = handle_key(&mut app, press(KeyCode::BackTab));
    match back {
        Some(Command::LoadDetail { id, .. }) if id == "i1" => {}
        other => panic!("expected Shift-Tab to load i1, got {other:?}"),
    }
    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(sample_detail("i1", "DAN-1")),
            reveal: Reveal::Top,
        },
    );

    let forward = handle_key(&mut app, press(KeyCode::Tab));
    match forward {
        Some(Command::LoadDetail { id, .. }) if id == "i2" => {}
        other => panic!("expected Tab to load i2, got {other:?}"),
    }
}

#[test]
fn tab_outside_the_detail_pane_still_cycles_panels() {
    let mut app = App::new();

    handle_key(&mut app, press(KeyCode::Tab));

    assert_eq!(app.focus, Focus::Recent);
}

#[test]
fn ctrl_d_and_ctrl_u_scroll_the_detail_by_half_a_page() {
    let mut app = detail_app();
    app.viewport = 20;
    app.scroll_position = 0;

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
    );
    assert_eq!(app.scroll_position, 10);

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
    );
    assert_eq!(app.scroll_position, 0);
}

#[test]
fn ctrl_d_pages_the_focused_list_without_wrapping() {
    let mut app = list_app_with_issues();
    app.viewport = 4;
    app.list_state.select(Some(0));

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
    );
    assert_eq!(app.list_state.selected(), Some(2));

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
    );
    assert_eq!(app.list_state.selected(), Some(2));
}

#[test]
fn opening_issues_records_history_and_ctrl_o_goes_back() {
    let mut app = App::new();

    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(sample_detail("i1", "DAN-1")),
            reveal: Reveal::Top,
        },
    );
    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(sample_detail("i2", "DAN-2")),
            reveal: Reveal::Top,
        },
    );

    assert_eq!(app.recently_viewed.len(), 2);
    assert_eq!(app.recently_viewed[0].id, "i2");
    assert_eq!(app.recently_viewed[1].id, "i1");

    let back = handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL),
    );
    match back {
        Some(Command::LoadDetail { id, .. }) if id == "i1" => {}
        other => panic!("expected Ctrl-o to load i1, got {other:?}"),
    }

    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(sample_detail("i1", "DAN-1")),
            reveal: Reveal::Top,
        },
    );
    assert_eq!(
        app.recently_viewed.len(),
        2,
        "re-viewing must not duplicate"
    );
    assert_eq!(app.recent_state.selected(), Some(1));
}

#[test]
fn enter_on_recently_viewed_reopens_the_issue() {
    let mut app = App::new();
    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(sample_detail("i1", "DAN-1")),
            reveal: Reveal::Top,
        },
    );
    app.detail = None;
    app.focus = Focus::Recent;
    app.recent_state.select(Some(0));

    let commands = handle_key(&mut app, press(KeyCode::Enter));

    assert!(app.focus.is_detail());
    match commands {
        Some(Command::LoadDetail { id, .. }) if id == "i1" => {}
        other => panic!("expected LoadDetail(i1), got {other:?}"),
    }
}

fn list_app_with_issue() -> App {
    let mut app = App::new();
    app.focus = Focus::MyWork;
    app.issues = vec![sample_issue("i1", "DAN2-7")];
    app.list_state.select(Some(0));
    app
}

fn list_app_with_issues() -> App {
    let mut app = App::new();
    app.focus = Focus::MyWork;
    app.issues = vec![
        sample_issue("i1", "DAN-1"),
        sample_issue("i2", "DAN-2"),
        sample_issue("i3", "DAN-3"),
    ];
    app.list_state.select(Some(0));
    app
}

fn detail_app() -> App {
    let mut app = list_app_with_issue();
    app.focus = Focus::Detail(LeftPanel::MyWork);
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
            state_type: linear_tui::api::StateType::Unstarted,
        },
        priority: linear_tui::api::Priority::None,
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
            state_type: linear_tui::api::StateType::Unstarted,
        },
        priority: linear_tui::api::Priority::None,
        assignee: None,
        labels: vec![],
        comments: vec![],
        branch_name: format!("dan/{}", identifier.to_lowercase()),
        team_id: "t_pizza".into(),
    }
}
