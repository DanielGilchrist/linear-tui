use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use linear_tui::api::fixture::FixtureClient;
use linear_tui::api::{
    CommentId, Cursor, IssueId, IssueRef, IssueSummary, Label, LabelId, Page, Reaction, ReactionId,
    ReactionTarget, Rgb, StateId, Team, TeamId, Timestamp, UserId, ViewId,
};
use linear_tui::api::{Credential, IssueUpdate, LinearApi, OAuthToken, Priority};
use linear_tui::store::Account;
use linear_tui::tui::app::{App, AuthState, RECENT_CAP};
use linear_tui::tui::cache::{CacheStatus, Remote};
use linear_tui::tui::event::Redraw;
use linear_tui::tui::feed::{Feed, FeedKey, FeedRequest};
use linear_tui::tui::focus::{DetailFocus, DetailView, Focus, LeftPanel, Origin, Reveal, Scroll};
use linear_tui::tui::message::{
    ApiCommand, Commands, Effect, Effects, FailureTarget, Message, PlatformCommand, RequestError,
    RuntimeCommand, StoreCommand,
};
use linear_tui::tui::overlay::{Compose, InputPurpose, Overlay, PickerKind};
use linear_tui::tui::render_to_string;
use linear_tui::tui::status::Status;
use linear_tui::tui::update::{apply as apply_all, handle_key as handle_key_all, tick};
use linear_tui::tui::view::ViewKind;

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
}

fn only(commands: Commands) -> Option<Effect> {
    match commands {
        Commands::Effects(effects) => {
            let mut iter = effects.into_iter();
            let first = iter.next();
            assert!(
                iter.next().is_none(),
                "expected at most one effect in the step"
            );

            first
        }
        Commands::Runtime(command) => panic!("expected effects, got a runtime step {command:?}"),
    }
}

fn effects(commands: Commands) -> Effects {
    match commands {
        Commands::Effects(effects) => effects,
        Commands::Runtime(command) => panic!("expected effects, got a runtime step {command:?}"),
    }
}

fn handle_key(app: &mut App, key: KeyEvent) -> Option<Effect> {
    only(handle_key_all(app, key))
}

fn apply(app: &mut App, message: Message) -> Option<Effect> {
    only(apply_all(app, message))
}

fn edit(app: &mut App, field: char) -> Option<Effect> {
    handle_key(app, press(KeyCode::Char('e')));
    handle_key(app, press(KeyCode::Char(field)))
}

fn signed_in() -> App {
    let mut app = App::new();
    app.session.upsert_account(Account {
        workspace_key: "ws".into(),
        org_name: "Test".into(),
        credential: Credential::PersonalKey("k".into()),
    });
    assert!(app.session.activate("ws"));

    app
}

fn scroll_line(app: &App) -> usize {
    app.reading_scroll()
        .and_then(|scroll| scroll.line())
        .expect("a resolved reading scroll line")
}

#[test]
fn w_opens_the_workspace_selector() {
    let mut app = App::new();

    handle_key(&mut app, press(KeyCode::Char('w')));

    assert!(matches!(app.overlay(), Overlay::Workspaces(_)));
}

#[test]
fn adding_a_key_requests_account_validation() {
    let mut app = App::new();

    handle_key(&mut app, press(KeyCode::Char('w')));
    // rows: browser, key, env var. Step past browser to "Add with an API key".
    handle_key(&mut app, press(KeyCode::Char('j')));
    handle_key(&mut app, press(KeyCode::Enter));
    assert_eq!(
        app.input().map(|input| input.purpose.clone()),
        Some(InputPurpose::AddWorkspaceKey)
    );

    handle_key(&mut app, press(KeyCode::Char('k')));
    let command = handle_key_all(&mut app, press(KeyCode::Enter));
    match command {
        Commands::Runtime(RuntimeCommand::AddAccount { credential }) => {
            assert_eq!(credential, Credential::PersonalKey("k".into()))
        }
        other => panic!("expected AddAccount, got {other:?}"),
    }
}

#[test]
fn account_added_is_stored_and_switched_to() {
    let mut app = App::new();
    let account = Account {
        workspace_key: "acme".into(),
        org_name: "Acme".into(),
        credential: Credential::PersonalKey("k".into()),
    };

    let command = apply_all(
        &mut app,
        Message::AccountAdded {
            account: Box::new(account),
        },
    );

    assert!(app
        .session
        .accounts()
        .iter()
        .any(|account| account.workspace_key == "acme"));
    match command {
        Commands::Runtime(RuntimeCommand::SwitchWorkspace(account)) => {
            assert_eq!(account.workspace_key, "acme")
        }
        other => panic!("expected SwitchWorkspace, got {other:?}"),
    }
}

#[test]
fn tick_is_idle_when_nothing_loads_or_expires() {
    let mut app = App::new();
    let now = app.now;

    assert_eq!(tick(&mut app, now), Redraw::Skipped);
}

#[test]
fn tick_advances_the_spinner_while_loading() {
    let mut app = App::new();
    app.open_detail_focus(DetailFocus {
        issue: IssueRef::Id(IssueId::from_raw("i1")),
        origin: Origin::Panel(LeftPanel::MyWork),
        view: DetailView::reading(),
        summary: None,
    });
    app.workspace.begin_detail();

    let before = app.ui.spinner.glyph();
    let now = app.now;

    assert_eq!(tick(&mut app, now), Redraw::Needed);
    assert_ne!(app.ui.spinner.glyph(), before);
}

#[test]
fn tick_redraws_when_a_timestamp_comes_due() {
    let mut app = App::new();
    app.now = Timestamp::from_epoch(30);
    seed_active(&mut app, vec![stamped_issue("i1", "DAN-1", 0)]);

    assert_eq!(tick(&mut app, Timestamp::from_epoch(59)), Redraw::Skipped);
    assert_eq!(tick(&mut app, Timestamp::from_epoch(60)), Redraw::Needed);
}

#[tokio::test]
async fn in_progress_filter_returns_only_started() {
    let client = FixtureClient::sample();
    let page = client
        .issues(&linear_tui::api::IssueFilter::in_progress_mine(), None)
        .await
        .unwrap();
    assert_eq!(page.items.len(), 3);
    assert!(page
        .items
        .iter()
        .all(|i| i.state.state_type == linear_tui::api::StateType::Started));
}

#[test]
fn bracket_cycles_to_next_view_and_requests_load() {
    let mut app = App::new();

    let commands = handle_key(&mut app, press(KeyCode::Char(']')));

    assert_eq!(app.active_view_index(), 1);
    assert!(app.focus().is_panel(LeftPanel::MyWork));
    match commands {
        Some(Effect::Api(ApiCommand::LoadFeed {
            request: FeedRequest::Refresh,
            ..
        })) => {}
        other => panic!("expected a feed refresh for view 1, got {other:?}"),
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
    assert!(app.prefix().is_some());
    assert!(commands.is_none());

    let command = handle_key(&mut app, press(KeyCode::Char('s')));
    assert!(matches!(
        command,
        Some(Effect::Api(ApiCommand::LoadStates { .. }))
    ));
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
    assert!(app.focus().is_panel(LeftPanel::SavedViews));
    assert!(commands.is_none());

    handle_key(&mut app, press(KeyCode::Char('4')));
    assert!(app.focus().is_panel(LeftPanel::Teams));
}

#[test]
fn focusing_teams_loads_them_once() {
    let mut app = App::new();

    match handle_key(&mut app, press(KeyCode::Char('4'))) {
        Some(Effect::Api(ApiCommand::LoadTeams)) => {}
        other => panic!("expected the teams panel to load, got {other:?}"),
    }

    handle_key(&mut app, press(KeyCode::Char('1')));

    assert!(
        handle_key(&mut app, press(KeyCode::Char('4'))).is_none(),
        "a cell already in flight is not requested twice"
    );
}

#[test]
fn a_loaded_teams_panel_has_a_selection() {
    let mut app = App::new();
    handle_key(&mut app, press(KeyCode::Char('4')));

    render_to_string(&mut app, 80, 24);
    app.workspace.teams.state.select(None);

    apply(
        &mut app,
        Message::TeamsLoaded {
            teams: vec![
                Team {
                    id: TeamId::from_raw("t_donut"),
                    name: "Donuts".into(),
                    key: "DAN".into(),
                    triage_enabled: false,
                },
                Team {
                    id: TeamId::from_raw("t_pizza"),
                    name: "Pizza".into(),
                    key: "DAN2".into(),
                    triage_enabled: true,
                },
            ],
        },
    );

    assert_eq!(app.teams().state.selected(), Some(0));
}

#[test]
fn recent_history_restores_a_selection_after_an_empty_render() {
    let mut app = App::new();
    handle_key(&mut app, press(KeyCode::Char('2')));

    render_to_string(&mut app, 80, 24);
    app.workspace.recent_state.select(None);

    apply(
        &mut app,
        Message::RecentLoaded(vec![sample_issue("i1", "DAN-1")]),
    );

    assert_eq!(app.workspace.recent_state.selected(), Some(0));
}

#[test]
fn navigating_the_teams_panel_moves_the_selection() {
    let mut app = App::new();
    handle_key(&mut app, press(KeyCode::Char('4')));

    apply(
        &mut app,
        Message::TeamsLoaded {
            teams: vec![
                Team {
                    id: TeamId::from_raw("t_donut"),
                    name: "Donuts".into(),
                    key: "DAN".into(),
                    triage_enabled: false,
                },
                Team {
                    id: TeamId::from_raw("t_pizza"),
                    name: "Pizza".into(),
                    key: "DAN2".into(),
                    triage_enabled: true,
                },
            ],
        },
    );
    assert_eq!(app.teams().state.selected(), Some(0));

    handle_key(&mut app, press(KeyCode::Char('j')));

    assert_eq!(app.teams().state.selected(), Some(1));
    assert_eq!(
        app.teams().selected().map(|team| team.name.as_str()),
        Some("Pizza")
    );
}

fn teams_app() -> App {
    let mut app = App::new();
    handle_key(&mut app, press(KeyCode::Char('4')));
    apply(
        &mut app,
        Message::TeamsLoaded {
            teams: vec![
                Team {
                    id: TeamId::from_raw("t_pizza"),
                    name: "Pizza".into(),
                    key: "DAN2".into(),
                    triage_enabled: true,
                },
                Team {
                    id: TeamId::from_raw("t_donut"),
                    name: "Donuts".into(),
                    key: "DAN".into(),
                    triage_enabled: false,
                },
            ],
        },
    );

    app
}

fn feed_filter(command: Option<Effect>) -> linear_tui::api::IssueFilter {
    match command {
        Some(Effect::Api(ApiCommand::LoadFeed {
            key: FeedKey::Issues(filter),
            ..
        })) => filter,
        other => panic!("expected a team-scoped feed load, got {other:?}"),
    }
}

#[test]
fn teams_are_ordered_by_key_so_a_long_list_is_stable() {
    let app = teams_app();

    assert_eq!(
        app.teams()
            .list()
            .iter()
            .map(|team| team.key.as_str())
            .collect::<Vec<_>>(),
        vec!["DAN", "DAN2"]
    );
}

#[test]
fn entering_a_team_opens_its_active_issues() {
    let mut app = teams_app();

    let filter = feed_filter(handle_key(&mut app, press(KeyCode::Enter)));

    assert!(app.focus().is_view());
    assert_eq!(app.view().map(|view| view.name()), Some("Donuts"));
    assert_eq!(filter.team, Some(TeamId::from_raw("t_donut")));
    assert_eq!(
        filter.state_types_in,
        vec![
            linear_tui::api::StateType::Unstarted,
            linear_tui::api::StateType::Started
        ],
        "a team opens on the browser, not a narrow mode"
    );
}

#[test]
fn cycling_a_team_surface_switches_mode_and_loads_that_feed() {
    let mut app = teams_app();
    let opened = feed_filter(handle_key(&mut app, press(KeyCode::Enter)));

    let backlog = feed_filter(handle_key(&mut app, press(KeyCode::Char(']'))));

    assert_ne!(backlog, opened, "each mode is its own feed key");
    assert_eq!(backlog.team, Some(TeamId::from_raw("t_donut")));
    assert_eq!(
        backlog.state_types_in,
        vec![linear_tui::api::StateType::Backlog]
    );
}

#[test]
fn a_cached_team_mode_is_not_refetched_when_you_cycle_back() {
    let mut app = teams_app();
    let active = feed_filter(handle_key(&mut app, press(KeyCode::Enter)));

    apply(
        &mut app,
        Message::FeedLoaded {
            key: FeedKey::Issues(active.clone()),
            request: FeedRequest::Refresh,
            page: Page::single(vec![sample_issue("i1", "DAN-1")]),
        },
    );

    handle_key(&mut app, press(KeyCode::Char(']')));
    let back = handle_key(&mut app, press(KeyCode::Char('[')));

    assert!(
        back.is_none(),
        "the mode's feed is already cached, so cycling back is free"
    );
    assert_eq!(app.view_len(), 1);
}

#[test]
fn escaping_a_team_surface_returns_to_the_teams_panel() {
    let mut app = teams_app();
    handle_key(&mut app, press(KeyCode::Enter));

    handle_key(&mut app, press(KeyCode::Esc));

    assert!(
        app.focus().is_panel(LeftPanel::Teams),
        "a team surface belongs to the teams panel, not saved views"
    );
}

#[test]
fn a_detail_opened_from_a_team_returns_to_that_team() {
    let mut app = teams_app();
    let filter = feed_filter(handle_key(&mut app, press(KeyCode::Enter)));

    apply(
        &mut app,
        Message::FeedLoaded {
            key: FeedKey::Issues(filter),
            request: FeedRequest::Refresh,
            page: Page::single(vec![sample_issue("i1", "DAN-1")]),
        },
    );
    handle_key(&mut app, press(KeyCode::Enter));
    assert!(matches!(app.focus(), Focus::Detail(..)));

    handle_key(&mut app, press(KeyCode::Esc));

    assert!(app.focus().is_view());
    assert_eq!(app.view().map(|view| view.name()), Some("Donuts"));
}

#[test]
fn reload_on_teams_reloads_teams_not_the_active_feed() {
    let mut app = App::new();

    handle_key(&mut app, press(KeyCode::Char('4')));
    apply(
        &mut app,
        Message::TeamsLoaded {
            teams: vec![Team {
                id: TeamId::from_raw("t_donut"),
                name: "Donuts".into(),
                key: "DAN".into(),
                triage_enabled: false,
            }],
        },
    );

    match handle_key(&mut app, press(KeyCode::Char('r'))) {
        Some(Effect::Api(ApiCommand::LoadTeams)) => {}
        other => panic!("expected a teams reload, got {other:?}"),
    }
}

#[test]
fn tab_cycles_from_my_work_into_the_stack() {
    let mut app = App::new();

    handle_key(&mut app, press(KeyCode::Tab));

    assert!(app.focus().is_panel(LeftPanel::Recent));
}

#[test]
fn views_loaded_prefetches_the_selected_view() {
    let mut app = App::new();
    app.focus_panel(LeftPanel::SavedViews);

    let command = apply(
        &mut app,
        Message::CustomViewsLoaded(vec![
            saved_view("v1", "Urgent"),
            saved_view("v2", "Menu ideas"),
        ]),
    );

    match command {
        Some(Effect::Api(ApiCommand::LoadFeed {
            key: FeedKey::View(id),
            ..
        })) if id.as_str() == "v1" => {}
        other => panic!("expected a prefetch for v1, got {other:?}"),
    }
}

#[test]
fn moving_the_selection_prefetches_the_next_view() {
    let mut app = saved_views_app();

    let command = handle_key(&mut app, press(KeyCode::Char('j')));

    match command {
        Some(Effect::Api(ApiCommand::LoadFeed {
            key: FeedKey::View(id),
            ..
        })) if id.as_str() == "v2" => {}
        other => panic!("expected a prefetch for v2, got {other:?}"),
    }
}

#[test]
fn entering_a_view_focuses_the_view_surface() {
    let mut app = saved_views_app();

    handle_key(&mut app, press(KeyCode::Enter));

    assert!(app.focus().is_view());
    assert_eq!(
        app.view().map(|view| view.key()),
        Some(FeedKey::View(ViewId::from_raw("v1")))
    );
}

#[test]
fn entering_an_issue_from_the_view_opens_the_detail() {
    let mut app = saved_views_app();
    handle_key(&mut app, press(KeyCode::Enter));
    load_view_feed(&mut app, "v1", vec![sample_issue("i1", "DAN2-7")]);

    let command = handle_key(&mut app, press(KeyCode::Enter));

    // the view stays open underneath so esc can return to it
    assert!(app.view().is_some());
    assert!(matches!(
        app.focus(),
        Focus::Detail(DetailFocus {
            origin: Origin::View(_),
            view: DetailView::Reading { .. },
            ..
        })
    ));
    match command {
        Some(Effect::Api(ApiCommand::LoadDetail {
            target,
            reveal: Reveal::Top,
        })) if target.as_str() == "i1" => {}
        other => panic!("expected LoadDetail for i1, got {other:?}"),
    }
}

#[test]
fn esc_closes_the_view_back_to_the_panel() {
    let mut app = saved_views_app();
    handle_key(&mut app, press(KeyCode::Enter));
    assert!(app.focus().is_view());

    handle_key(&mut app, press(KeyCode::Esc));

    assert!(app.view().is_none());
    assert!(app.focus().is_panel(LeftPanel::SavedViews));
}

#[test]
fn esc_from_a_view_opened_detail_returns_to_the_view() {
    let mut app = saved_views_app();
    handle_key(&mut app, press(KeyCode::Enter));
    load_view_feed(&mut app, "v1", vec![sample_issue("i1", "DAN2-7")]);
    handle_key(&mut app, press(KeyCode::Enter));
    assert!(matches!(app.focus(), Focus::Detail(..)));

    handle_key(&mut app, press(KeyCode::Esc));

    assert!(app.focus().is_view());
    assert_eq!(
        app.view().map(|view| view.key()),
        Some(FeedKey::View(ViewId::from_raw("v1")))
    );
}

#[test]
fn z_toggles_zoom_and_esc_unzooms_before_closing() {
    let mut app = saved_views_app();
    handle_key(&mut app, press(KeyCode::Enter));

    handle_key(&mut app, press(KeyCode::Char('z')));
    assert_eq!(app.ui.zoom, linear_tui::tui::app::Zoom::Full);

    // esc unzooms first, keeping the view open
    handle_key(&mut app, press(KeyCode::Esc));
    assert_eq!(app.ui.zoom, linear_tui::tui::app::Zoom::Normal);
    assert!(app.view().is_some());

    // a second esc closes it
    handle_key(&mut app, press(KeyCode::Esc));
    assert!(app.view().is_none());
    assert!(app.focus().is_panel(LeftPanel::SavedViews));
}

#[test]
fn zoom_works_on_the_my_work_list() {
    let mut app = list_app_with_issue();

    handle_key(&mut app, press(KeyCode::Char('z')));
    assert_eq!(app.ui.zoom, linear_tui::tui::app::Zoom::Full);
    assert!(app.focus().is_panel(LeftPanel::MyWork));

    handle_key(&mut app, press(KeyCode::Esc));
    assert_eq!(app.ui.zoom, linear_tui::tui::app::Zoom::Normal);
    assert!(app.focus().is_panel(LeftPanel::MyWork));
}

#[test]
fn the_display_prefix_cycles_group_and_sort() {
    let mut app = saved_views_app();
    handle_key(&mut app, press(KeyCode::Enter));

    // v then g cycles group status -> priority
    handle_key(&mut app, press(KeyCode::Char('v')));
    handle_key(&mut app, press(KeyCode::Char('g')));
    assert_eq!(
        app.view().unwrap().display.group,
        linear_tui::tui::display::GroupBy::Priority
    );

    // v then s cycles sort manual -> priority
    handle_key(&mut app, press(KeyCode::Char('v')));
    handle_key(&mut app, press(KeyCode::Char('s')));
    assert_eq!(
        app.view().unwrap().display.sort,
        linear_tui::tui::display::SortBy::Priority
    );
}

#[test]
fn status_acts_on_the_highlighted_view_issue() {
    let mut app = saved_views_app();
    handle_key(&mut app, press(KeyCode::Enter));
    load_view_feed(&mut app, "v1", vec![sample_issue("i1", "DAN2-7")]);

    let command = edit(&mut app, 's');

    assert_eq!(app.picker().map(|p| &p.kind), Some(&PickerKind::Status));
    match command {
        Some(Effect::Api(ApiCommand::LoadStates { team_id })) if team_id.as_str() == "t_pizza" => {}
        other => panic!("expected LoadStates for the highlighted issue, got {other:?}"),
    }
}

#[test]
fn the_panel_starts_in_a_loading_state() {
    let mut app = App::new();
    linear_tui::tui::update::initial_commands(&mut app);
    assert!(app.workspace.saved_views.views.in_flight());
}

#[test]
fn views_load_prefetches_even_when_focus_is_elsewhere() {
    let mut app = App::new();
    assert!(app.focus().is_panel(LeftPanel::MyWork));

    let command = apply(
        &mut app,
        Message::CustomViewsLoaded(vec![saved_view("v1", "Urgent")]),
    );

    match command {
        Some(Effect::Api(ApiCommand::LoadFeed {
            key: FeedKey::View(id),
            ..
        })) if id.as_str() == "v1" => {}
        other => panic!("expected a prefetch for v1 from MyWork, got {other:?}"),
    }
}

#[test]
fn a_failed_views_fetch_clears_the_panel_loading_flag() {
    let mut app = App::new();
    app.workspace.saved_views.views.begin();
    assert!(app.workspace.saved_views.views.in_flight());

    apply(
        &mut app,
        Message::Failed {
            target: FailureTarget::CustomViews,
            error: RequestError::Other("boom".into()),
        },
    );

    assert!(!app.workspace.saved_views.views.in_flight());
    assert!(matches!(app.ui.status, Some(Status::Error(_))));
}

#[test]
fn a_failed_view_issues_fetch_is_recorded_and_can_be_retried() {
    let mut app = saved_views_app();
    handle_key(&mut app, press(KeyCode::Enter));

    apply(
        &mut app,
        Message::Failed {
            target: FailureTarget::Feed(FeedKey::View(ViewId::from_raw("v1"))),
            error: RequestError::Other("boom".into()),
        },
    );

    assert!(matches!(
        app.workspace
            .feeds
            .get(&FeedKey::View(ViewId::from_raw("v1")))
            .map(|feed| feed.status()),
        Some(CacheStatus::Failed(_))
    ));
    assert!(matches!(app.ui.status, Some(Status::Error(_))));

    // r on the open view refetches rather than leaving a permanent spinner
    let command = handle_key(&mut app, press(KeyCode::Char('r')));
    match command {
        Some(Effect::Api(ApiCommand::LoadFeed {
            key: FeedKey::View(id),
            ..
        })) if id.as_str() == "v1" => {}
        other => panic!("expected a retry for v1, got {other:?}"),
    }
}

#[test]
fn a_failed_view_is_refetched_on_revisit_from_the_panel() {
    let mut app = saved_views_app();
    handle_key(&mut app, press(KeyCode::Enter));
    apply(
        &mut app,
        Message::Failed {
            target: FailureTarget::Feed(FeedKey::View(ViewId::from_raw("v1"))),
            error: RequestError::Other("boom".into()),
        },
    );
    handle_key(&mut app, press(KeyCode::Esc));

    // move off v1 and back: prefetch must refetch the Failed entry
    handle_key(&mut app, press(KeyCode::Char('j')));
    let command = handle_key(&mut app, press(KeyCode::Char('k')));

    match command {
        Some(Effect::Api(ApiCommand::LoadFeed {
            key: FeedKey::View(id),
            ..
        })) if id.as_str() == "v1" => {}
        other => panic!("expected a refetch for the failed v1, got {other:?}"),
    }
}

#[test]
fn reloading_a_shrunk_view_keeps_the_selection_in_range() {
    let mut app = saved_views_app();
    handle_key(&mut app, press(KeyCode::Enter));
    load_view_feed(
        &mut app,
        "v1",
        vec![
            sample_issue("i1", "DAN-1"),
            sample_issue("i2", "DAN-2"),
            sample_issue("i3", "DAN-3"),
        ],
    );
    // select the last issue
    handle_key(&mut app, press(KeyCode::Char('j')));
    handle_key(&mut app, press(KeyCode::Char('j')));

    // the view now returns a single issue
    load_view_feed(&mut app, "v1", vec![sample_issue("i1", "DAN-1")]);

    assert!(
        app.view_selected_issue().is_some(),
        "selection stranded out of range after the view shrank"
    );
}

#[tokio::test]
async fn the_fixture_serves_distinct_issues_per_view() {
    let client = FixtureClient::sample();

    let urgent = client
        .custom_view_issues(&ViewId::from_raw("v_urgent"), None)
        .await
        .unwrap()
        .items;
    let oven = client
        .custom_view_issues(&ViewId::from_raw("v_oven"), None)
        .await
        .unwrap()
        .items;

    assert_ne!(urgent.len(), oven.len());
    assert!(oven
        .iter()
        .all(|issue| issue.id.as_str() == "i1" || issue.id.as_str() == "i3"));
}

#[test]
fn tabbing_away_from_a_view_closes_the_surface() {
    let mut app = saved_views_app();
    handle_key(&mut app, press(KeyCode::Enter));
    assert!(app.focus().is_view());

    handle_key(&mut app, press(KeyCode::Tab));

    assert!(
        app.view().is_none(),
        "zombie view surface survived tab-away"
    );
    assert!(!app.focus().is_view());
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

    assert_eq!(app.workspace.recently_viewed.len(), 2);
}

#[test]
fn revealing_an_index_past_the_list_end_clamps_to_the_last_row() {
    let mut app = list_app_with_issues();
    let len = app.active_issues().len();
    assert_eq!(len, 3);

    app.reveal_focused(Some(len));

    assert_eq!(app.ui.list_state.selected(), Some(len - 1));

    app.reveal_focused(Some(99));

    assert_eq!(app.ui.list_state.selected(), Some(len - 1));
}

#[test]
fn recent_history_loaded_after_a_descend_merges_instead_of_vanishing() {
    let mut app = App::new();

    app.record_recent(sample_issue("i9", "DAN-9"));

    apply(
        &mut app,
        Message::RecentLoaded(vec![
            sample_issue("i1", "DAN-1"),
            sample_issue("i9", "DAN-9"),
            sample_issue("i2", "DAN-2"),
        ]),
    );

    let identifiers: Vec<&str> = app
        .workspace
        .recently_viewed
        .iter()
        .map(|issue| issue.identifier.as_str())
        .collect();

    assert_eq!(identifiers, vec!["DAN-9", "DAN-1", "DAN-2"]);
    assert_eq!(app.workspace.recent_state.selected(), Some(0));
}

#[test]
fn merging_recent_history_stays_within_the_cap() {
    let mut app = App::new();

    app.record_recent(sample_issue("live", "DAN-0"));

    let loaded: Vec<IssueSummary> = (0..RECENT_CAP + 10)
        .map(|n| sample_issue(&format!("i{n}"), &format!("DAN-{n}")))
        .collect();

    apply(&mut app, Message::RecentLoaded(loaded));

    assert_eq!(app.workspace.recently_viewed.len(), RECENT_CAP);
    assert_eq!(app.workspace.recently_viewed[0].identifier, "DAN-0");
}

#[test]
fn clearing_recently_viewed_confirms_first() {
    let mut app = App::new();
    app.open_detail_focus(DetailFocus::reading(
        IssueId::from_raw("i1"),
        Origin::Panel(LeftPanel::MyWork),
    ));
    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(sample_detail("i1", "DAN-1")),
            reveal: Reveal::Top,
        },
    );
    handle_key(&mut app, press(KeyCode::Char('2')));
    assert!(app.focus().is_panel(LeftPanel::Recent));

    handle_key(&mut app, press(KeyCode::Char('x')));
    assert!(app.confirm().is_some());

    let command = handle_key(&mut app, press(KeyCode::Char('y')));
    match command {
        Some(Effect::Store(StoreCommand::ClearRecent)) => {}
        other => panic!("expected ClearRecent, got {other:?}"),
    }

    apply(&mut app, Message::RecentCleared);
    assert!(app.workspace.recently_viewed.is_empty());
    assert!(
        !app.active_feed_status().in_flight(),
        "confirming a non-fetch command must not leave the view spinner stuck"
    );
}

#[test]
fn clearing_does_nothing_off_the_recent_panel() {
    let mut app = list_app_with_issue();
    app.workspace.recently_viewed = vec![sample_issue("i1", "DAN-1")];

    handle_key(&mut app, press(KeyCode::Char('x')));

    assert!(app.confirm().is_none());
}

#[test]
fn brackets_do_nothing_off_my_work() {
    let mut app = App::new();
    app.focus_panel(LeftPanel::Teams);

    let commands = handle_key(&mut app, press(KeyCode::Char(']')));

    assert_eq!(app.active_view_index(), 0);
    assert!(commands.is_none());
}

#[test]
fn enter_on_issue_opens_detail() {
    let mut app = list_app_with_issue();

    let commands = handle_key(&mut app, press(KeyCode::Enter));

    assert!(matches!(app.focus(), Focus::Detail(..)));
    assert!(app.workspace.detail().in_flight());
    match commands {
        Some(Effect::Api(ApiCommand::LoadDetail { target, .. })) if target.as_str() == "i1" => {}
        other => panic!("expected LoadDetail(i1), got {other:?}"),
    }
}

#[test]
fn esc_from_a_detail_returns_to_the_panel_it_was_opened_from() {
    let mut app = list_app_with_issue();

    handle_key(&mut app, press(KeyCode::Enter));
    assert!(matches!(
        app.focus(),
        Focus::Detail(DetailFocus {
            origin: Origin::Panel(LeftPanel::MyWork),
            ..
        })
    ));

    handle_key(&mut app, press(KeyCode::Esc));
    assert!(app.focus().is_panel(LeftPanel::MyWork));
}

#[test]
fn detail_actions_do_nothing_in_the_detail_pane_when_no_issue_is_open() {
    let mut app = list_app_with_issue();

    handle_key(&mut app, press(KeyCode::BackTab));
    assert!(matches!(app.focus(), Focus::Detail(..)));

    handle_key(&mut app, press(KeyCode::Char('+')));
    assert!(matches!(app.overlay(), Overlay::None));
}

#[test]
fn cycling_into_the_detail_pane_opens_the_selected_issue() {
    let mut app = detail_app();

    handle_key(&mut app, press(KeyCode::Esc));
    assert!(app.focus().is_panel(LeftPanel::MyWork));
    assert!(
        app.workspace.detail().value().is_some(),
        "leaving must not touch the cached detail"
    );

    handle_key(&mut app, press(KeyCode::BackTab));
    match app.focus() {
        Focus::Detail(detail) => assert_eq!(detail.issue.as_str(), "i1"),
        _ => panic!("expected cycling into the pane to open the selection"),
    }

    handle_key(&mut app, press(KeyCode::Char('+')));
    match app.overlay() {
        Overlay::Reactions(reactions) => {
            assert_eq!(
                reactions.target,
                ReactionTarget::Issue(IssueId::from_raw("i1"))
            );
        }
        _ => panic!("expected reactions to target the reopened issue"),
    }
}

#[test]
fn detail_actions_target_the_open_issue_after_tabbing_away_and_back() {
    let mut app = detail_app();

    handle_key(&mut app, press(KeyCode::Char('1')));
    assert!(app.focus().is_panel(LeftPanel::MyWork));

    handle_key(&mut app, press(KeyCode::BackTab));
    assert!(matches!(app.focus(), Focus::Detail(..)));

    handle_key(&mut app, press(KeyCode::Char('+')));
    match app.overlay() {
        Overlay::Reactions(reactions) => {
            assert_eq!(
                reactions.target,
                ReactionTarget::Issue(IssueId::from_raw("i1"))
            );
        }
        _ => panic!("expected the reactions overlay"),
    }
}

#[test]
fn esc_from_a_detail_opened_from_recent_returns_to_recent() {
    let mut app = App::new();
    app.open_detail_focus(DetailFocus::reading(
        IssueId::from_raw("i1"),
        Origin::Panel(LeftPanel::MyWork),
    ));
    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(sample_detail("i1", "DAN-1")),
            reveal: Reveal::Top,
        },
    );

    handle_key(&mut app, press(KeyCode::Char('2')));
    assert!(app.focus().is_panel(LeftPanel::Recent));

    handle_key(&mut app, press(KeyCode::Enter));
    assert!(matches!(
        app.focus(),
        Focus::Detail(DetailFocus {
            origin: Origin::Panel(LeftPanel::Recent),
            ..
        })
    ));

    handle_key(&mut app, press(KeyCode::Esc));
    assert!(app.focus().is_panel(LeftPanel::Recent));
}

#[test]
fn react_in_reading_targets_the_issue() {
    let mut app = detail_app();

    handle_key(&mut app, press(KeyCode::Char('+')));

    match app.overlay() {
        Overlay::Reactions(reactions) => {
            assert_eq!(
                reactions.target,
                ReactionTarget::Issue(IssueId::from_raw("i1"))
            );
        }
        _ => panic!("expected the reactions overlay"),
    }
}

#[test]
fn react_in_comments_targets_the_selected_comment() {
    let mut app = detail_app_with_comments();
    handle_key(&mut app, press(KeyCode::Char('m')));

    handle_key(&mut app, press(KeyCode::Char('+')));

    match app.overlay() {
        Overlay::Reactions(reactions) => {
            assert_eq!(
                reactions.target,
                ReactionTarget::Comment(CommentId::from_raw("c1"))
            );
        }
        _ => panic!("expected the reactions overlay"),
    }
}

#[test]
fn toggling_reactions_deletes_your_own_and_creates_a_new_one() {
    let mut app = list_app_with_issue();
    app.open_detail_focus(DetailFocus {
        issue: IssueRef::Id(IssueId::from_raw("i1")),
        origin: Origin::Panel(LeftPanel::MyWork),
        view: DetailView::reading(),
        summary: None,
    });
    let mut detail = sample_detail("i1", "DAN2-7");
    detail.reactions = vec![Reaction {
        id: ReactionId::from_raw("rx"),
        emoji: "+1".into(),
        mine: true,
    }];
    app.workspace.set_detail(detail, app.now);

    // Open, step up into the Current row (your +1), toggle it off.
    handle_key(&mut app, press(KeyCode::Char('+')));
    handle_key(&mut app, press(KeyCode::Char('k')));
    let deleted = handle_key(&mut app, press(KeyCode::Enter));
    match deleted {
        Some(Effect::Api(ApiCommand::DeleteReaction {
            reaction_id,
            issue_id,
        })) => {
            assert_eq!(reaction_id.as_str(), "rx");
            assert_eq!(issue_id.as_str(), "i1");
        }
        other => panic!("expected DeleteReaction, got {other:?}"),
    }

    // Reopen: the highlight starts on the first Add item (heart), toggle it on.
    handle_key(&mut app, press(KeyCode::Char('+')));
    let created = handle_key(&mut app, press(KeyCode::Enter));
    match created {
        Some(Effect::Api(ApiCommand::CreateReaction {
            target,
            emoji,
            issue_id,
        })) => {
            assert_eq!(target, ReactionTarget::Issue(IssueId::from_raw("i1")));
            assert_eq!(emoji, "heart");
            assert_eq!(issue_id.as_str(), "i1");
        }
        other => panic!("expected CreateReaction, got {other:?}"),
    }
}

#[test]
fn a_custom_reaction_you_made_is_removable_from_the_current_section() {
    let mut app = list_app_with_issue();
    app.open_detail_focus(DetailFocus {
        issue: IssueRef::Id(IssueId::from_raw("i1")),
        origin: Origin::Panel(LeftPanel::MyWork),
        view: DetailView::reading(),
        summary: None,
    });
    let mut detail = sample_detail("i1", "DAN2-7");
    detail.reactions = vec![Reaction {
        id: ReactionId::from_raw("re"),
        emoji: "eggplant".into(),
        mine: true,
    }];
    app.workspace.set_detail(detail, app.now);

    // The custom reaction sits in the Current row: step up, toggle it off.
    handle_key(&mut app, press(KeyCode::Char('+')));
    handle_key(&mut app, press(KeyCode::Char('k')));
    let removed = handle_key(&mut app, press(KeyCode::Enter));
    match removed {
        Some(Effect::Api(ApiCommand::DeleteReaction { reaction_id, .. })) => {
            assert_eq!(reaction_id.as_str(), "re")
        }
        other => panic!("expected DeleteReaction for the custom reaction, got {other:?}"),
    }
}

#[test]
fn custom_reaction_routes_through_the_input_overlay() {
    let mut app = detail_app();

    handle_key(&mut app, press(KeyCode::Char('+')));
    handle_key(&mut app, press(KeyCode::Char('c')));

    match app.overlay() {
        Overlay::Input(input) => assert_eq!(
            input.purpose,
            InputPurpose::CustomReaction {
                issue_id: IssueId::from_raw("i1"),
                target: ReactionTarget::Issue(IssueId::from_raw("i1"))
            }
        ),
        _ => panic!("expected the input overlay"),
    }

    handle_key(&mut app, press(KeyCode::Char('🚀')));
    let command = handle_key(&mut app, press(KeyCode::Enter));
    match command {
        Some(Effect::Api(ApiCommand::CreateReaction { emoji, .. })) => assert_eq!(emoji, "🚀"),
        other => panic!("expected CreateReaction, got {other:?}"),
    }
}

#[test]
fn a_reaction_for_a_stale_issue_reports_rather_than_swallowing() {
    let mut app = detail_app();
    handle_key(&mut app, press(KeyCode::Char('+')));

    // The detail pane moves on to another issue while the overlay is open.
    app.workspace
        .set_detail(sample_detail("i2", "DAN-2"), app.now);
    app.refocus_detail_issue(IssueRef::Id(IssueId::from_raw("i2")));

    let command = handle_key(&mut app, press(KeyCode::Enter));

    assert!(command.is_none(), "a mismatched target must not fire");
    assert_eq!(app.ui.status, Some(Status::NeedHighlightedIssue));
}

#[test]
fn reaction_toggled_reloads_the_detail() {
    let mut app = detail_app();

    let command = apply(
        &mut app,
        Message::ReactionToggled {
            id: IssueId::from_raw("i1"),
        },
    );
    match command {
        Some(Effect::Api(ApiCommand::LoadDetail { target, reveal })) => {
            assert_eq!(target.as_str(), "i1");
            assert_eq!(reveal, Reveal::Keep);
        }
        other => panic!("expected LoadDetail, got {other:?}"),
    }
    assert!(app.workspace.detail().in_flight());
}

#[test]
fn reveal_keep_preserves_the_comment_selection() {
    let mut app = detail_app_with_comments();
    handle_key(&mut app, press(KeyCode::Char('m')));
    app.reveal_focused(Some(2));

    let detail = app.workspace.detail().value().cloned().expect("detail");
    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(detail),
            reveal: Reveal::Keep,
        },
    );

    assert_eq!(app.comment_cursor(), Some(2));
}

#[test]
fn a_fresh_cached_view_is_not_refetched() {
    let mut app = App::new();
    let filter = linear_tui::api::IssueFilter::in_progress_mine();
    app.workspace.feeds.insert(
        FeedKey::Issues(filter),
        Feed::ready(Page::single(vec![sample_issue("i1", "DAN-1")]), app.now),
    );

    let command = handle_key(&mut app, press(KeyCode::Char(']')));

    assert!(command.is_none(), "a fresh cached feed must not refetch");
    assert_eq!(app.active_issues().len(), 1);
}

#[test]
fn a_stale_cached_view_revalidates_but_keeps_its_rows() {
    let mut app = App::new();
    let filter = linear_tui::api::IssueFilter::in_progress_mine();
    app.workspace.feeds.insert(
        FeedKey::Issues(filter),
        Feed::ready(
            Page::single(vec![sample_issue("i1", "DAN-1")]),
            Timestamp::from_epoch(0),
        ),
    );
    app.now = Timestamp::from_epoch(5 * 60);

    let command = handle_key(&mut app, press(KeyCode::Char(']')));

    assert!(matches!(
        command,
        Some(Effect::Api(ApiCommand::LoadFeed {
            request: FeedRequest::Refresh,
            ..
        }))
    ));
    assert_eq!(app.active_issues().len(), 1);
}

#[test]
fn a_cold_cached_view_busts_and_full_loads() {
    let mut app = App::new();
    let filter = linear_tui::api::IssueFilter::in_progress_mine();
    app.workspace.feeds.insert(
        FeedKey::Issues(filter),
        Feed::ready(
            Page::single(vec![sample_issue("i1", "DAN-1")]),
            Timestamp::from_epoch(0),
        ),
    );
    app.now = Timestamp::from_epoch(24 * 60 * 60);

    let command = handle_key(&mut app, press(KeyCode::Char(']')));

    assert!(matches!(
        command,
        Some(Effect::Api(ApiCommand::LoadFeed {
            request: FeedRequest::Refresh,
            ..
        }))
    ));
    assert!(
        app.active_issues().is_empty(),
        "a cold feed must clear its stale rows rather than flash them"
    );
}

#[test]
fn scrolling_near_the_end_loads_the_next_page() {
    let mut app = App::new();
    app.focus_my_work();
    let key = app.active_feed_key().unwrap();
    let items: Vec<_> = (0..12)
        .map(|n| sample_issue(&format!("i{n}"), &format!("DAN-{n}")))
        .collect();
    app.workspace.feeds.insert(
        key,
        Feed::ready(
            Page {
                items,
                next: Some(Cursor("cursor-1".into())),
            },
            app.now,
        ),
    );
    app.ui.list_state.select(Some(5));

    let command = handle_key(&mut app, press(KeyCode::Char('j')));

    match command {
        Some(Effect::Api(ApiCommand::LoadFeed {
            key: FeedKey::Issues(_),
            request: FeedRequest::LoadMore { after },
        })) if after == Cursor("cursor-1".into()) => {}
        other => panic!("expected a LoadMore for the next page, got {other:?}"),
    }
}

#[test]
fn jumping_to_the_bottom_of_a_truncated_feed_loads_the_next_page() {
    let mut app = App::new();
    app.focus_my_work();
    let key = app.active_feed_key().unwrap();
    let items: Vec<_> = (0..12)
        .map(|n| sample_issue(&format!("i{n}"), &format!("DAN-{n}")))
        .collect();
    app.workspace.feeds.insert(
        key,
        Feed::ready(
            Page {
                items,
                next: Some(Cursor("cursor-1".into())),
            },
            app.now,
        ),
    );
    app.ui.list_state.select(Some(0));

    let command = handle_key(&mut app, press(KeyCode::Char('G')));

    assert_eq!(app.ui.list_state.selected(), Some(11));
    match command {
        Some(Effect::Api(ApiCommand::LoadFeed {
            key: FeedKey::Issues(_),
            request: FeedRequest::LoadMore { after },
        })) if after == Cursor("cursor-1".into()) => {}
        other => panic!("expected G to load the next page, got {other:?}"),
    }
}

#[test]
fn feed_results_land_only_in_their_own_key() {
    let mut app = App::new();
    apply(
        &mut app,
        Message::FeedLoaded {
            key: FeedKey::Issues(linear_tui::api::IssueFilter::in_progress_mine()),
            request: FeedRequest::Refresh,
            page: Page::single(vec![sample_issue("i1", "ENG-1")]),
        },
    );

    assert!(app.active_issues().is_empty());
}

#[test]
fn reconnect_reissues_an_in_flight_feed_rather_than_orphaning_it() {
    let mut app = App::new();
    app.focus_my_work();
    let key = app.active_feed_key().unwrap();
    app.workspace
        .feeds
        .get_or_default(&key)
        .begin(&FeedRequest::Refresh);
    assert!(app.workspace.feeds.get(&key).unwrap().in_flight());

    let commands = linear_tui::tui::update::reconnect(&mut app);

    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Effect::Api(ApiCommand::LoadFeed { key: k, .. }) if k == &key)),
        "an in-flight feed must be re-requested after reconnect, not left spinning forever"
    );
}

#[test]
fn two_reloads_while_the_session_is_failed_issue_one_request() {
    let mut app = list_app_with_issue();
    app.workspace.session.fail("boom".into());

    let first = effects(handle_key_all(&mut app, press(KeyCode::Char('r'))));
    assert!(
        first
            .iter()
            .any(|command| matches!(command, Effect::Api(ApiCommand::LoadSession))),
        "a failed session cell is retried on reload"
    );

    let second = effects(handle_key_all(&mut app, press(KeyCode::Char('r'))));
    assert!(
        !second
            .iter()
            .any(|command| matches!(command, Effect::Api(ApiCommand::LoadSession))),
        "the retry is already in flight, so a second reload must not duplicate it"
    );
}

#[test]
fn a_reconnect_leaves_no_cell_in_flight_without_a_pending_reply() {
    let mut app = App::new();
    app.workspace.begin_detail();
    assert!(app.workspace.detail().in_flight());

    linear_tui::tui::update::reconnect(&mut app);

    assert!(
        !app.workspace.detail().in_flight(),
        "nothing reloads the detail after reconnect, so it must settle rather than orphan"
    );
}

#[test]
fn reconnect_reissues_a_loading_detail() {
    let mut app = detail_app();
    app.workspace.begin_detail();

    let commands = linear_tui::tui::update::reconnect(&mut app);

    assert!(
        commands.iter().any(|command| matches!(
            command,
            Effect::Api(ApiCommand::LoadDetail {
                reveal: Reveal::Keep,
                ..
            })
        )),
        "a focused detail must be re-requested after the cancel that dropped it"
    );
    assert!(app.workspace.detail().in_flight());
}

#[test]
fn reconnect_reissues_a_loading_view_feed() {
    let mut app = saved_views_app();
    handle_key(&mut app, press(KeyCode::Enter));

    let key = app.view().expect("a view surface is open").key();
    app.workspace
        .feeds
        .get_or_default(&key)
        .begin(&FeedRequest::Refresh);

    let commands = linear_tui::tui::update::reconnect(&mut app);

    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Effect::Api(ApiCommand::LoadFeed { key: k, .. }) if k == &key)),
        "the focused view's feed must be re-requested after reconnect"
    );
}

#[test]
fn a_reconnect_unwedges_a_searching_picker() {
    let mut app = detail_app();
    edit(&mut app, 'a');
    handle_key(&mut app, press(KeyCode::Char('/')));
    handle_key(&mut app, press(KeyCode::Char('d')));
    handle_key(&mut app, press(KeyCode::Enter));
    assert!(app.overlay_in_flight());

    linear_tui::tui::update::reconnect(&mut app);

    assert!(
        !app.overlay_in_flight(),
        "a cancelled search must settle rather than spin forever"
    );
}

#[test]
fn an_unbound_key_inside_a_picker_keeps_an_async_error_visible() {
    let mut app = detail_app();
    edit(&mut app, 's');

    apply(
        &mut app,
        Message::Failed {
            target: FailureTarget::States {
                team_id: TeamId::from_raw("t_pizza"),
            },
            error: RequestError::Other("boom".into()),
        },
    );
    assert!(matches!(app.ui.status, Some(Status::Error(_))));

    handle_key(&mut app, press(KeyCode::F(5)));

    assert!(
        matches!(app.ui.status, Some(Status::Error(_))),
        "an unbound key has no opinion on the status, so it must not erase one"
    );
}

#[test]
fn a_background_feed_refresh_does_not_clear_an_error_status() {
    let mut app = list_app_with_issue();
    let key = app.active_feed_key().expect("an active feed");
    app.ui.status = Some(Status::Error("boom".into()));

    apply(
        &mut app,
        Message::FeedLoaded {
            key,
            request: FeedRequest::Refresh,
            page: Page::single(vec![sample_issue("i1", "DAN-1")]),
        },
    );

    assert!(
        matches!(app.ui.status, Some(Status::Error(_))),
        "a background refresh must not wipe an unread error"
    );
}

#[test]
fn cancelling_a_picker_replaces_a_stale_status() {
    let mut app = detail_app();

    handle_key(&mut app, press(KeyCode::Char('y')));
    assert_eq!(app.ui.status, Some(Status::CopiedUrl));

    edit(&mut app, 's');
    handle_key(&mut app, press(KeyCode::Esc));

    assert_eq!(
        app.ui.status,
        Some(Status::Cancelled),
        "closing a task-like overlay reports Cancelled rather than leaving a stale success"
    );
}

#[test]
fn a_picker_opened_onto_an_in_flight_cell_shows_loading() {
    let mut app = detail_app();

    edit(&mut app, 's');
    handle_key(&mut app, press(KeyCode::Esc));

    let reopened = edit(&mut app, 's');

    assert!(
        reopened.is_none(),
        "the states cell is already in flight, so no second request goes out"
    );
    assert!(
        app.overlay_in_flight(),
        "the picker reads the in-flight cell rather than a bool of its own"
    );
}

#[test]
fn switching_the_workspace_clears_the_ui_pointing_at_the_old_one() {
    let filter = linear_tui::api::IssueFilter::assigned_to_me();
    let mut app = detail_app();
    assert!(app.workspace.detail().value().is_some());

    handle_key(&mut app, press(KeyCode::Char('c')));
    assert!(
        app.editor().is_some(),
        "an editor targeting the old workspace's issue is open when the switch arrives"
    );

    app.workspace
        .recently_viewed
        .push(sample_issue("i9", "DAN-9"));
    app.workspace.feeds.insert(
        FeedKey::Issues(filter.clone()),
        Feed::ready(Page::single(vec![sample_issue("i1", "DAN-1")]), app.now),
    );

    app.session.upsert_account(Account {
        workspace_key: "acme".into(),
        org_name: "Acme".into(),
        credential: Credential::PersonalKey("k".into()),
    });
    assert!(app.session.activate("acme"));
    app.reset_workspace();

    assert!(app.workspace.session.value().is_none());
    assert!(app.workspace.detail().value().is_none());
    assert!(app.workspace.recently_viewed.is_empty());
    assert!(app.workspace.feeds.get(&FeedKey::Issues(filter)).is_none());

    assert!(
        matches!(app.overlay(), Overlay::None),
        "an overlay holding the old workspace's issue id must not survive the switch"
    );
    assert!(app.focus().is_panel(LeftPanel::MyWork));
    assert!(app.view().is_none());
}

#[test]
fn esc_from_detail_focuses_my_work() {
    let mut app = detail_app();

    handle_key(&mut app, press(KeyCode::Esc));

    assert!(app.focus().is_panel(LeftPanel::MyWork));
}

#[test]
fn status_action_requires_an_opened_issue() {
    let mut app = list_app_with_issue();

    let commands = edit(&mut app, 's');

    assert!(app.picker().is_none());
    assert!(commands.is_none());
}

#[test]
fn s_opens_status_picker_once_issue_is_loaded() {
    let mut app = detail_app();

    let commands = edit(&mut app, 's');

    assert_eq!(app.picker().map(|p| &p.kind), Some(&PickerKind::Status));
    match commands {
        Some(Effect::Api(ApiCommand::LoadStates { team_id })) if team_id.as_str() == "t_pizza" => {}
        other => panic!("expected LoadStates for t_pizza, got {other:?}"),
    }
}

#[test]
fn m_enters_comments_mode_and_selects_the_first_comment() {
    let mut app = detail_app_with_comments();

    let command = handle_key(&mut app, press(KeyCode::Char('m')));

    assert!(matches!(
        app.focus(),
        Focus::Detail(DetailFocus {
            origin: Origin::Panel(LeftPanel::MyWork),
            view: DetailView::Comments { .. },
            ..
        })
    ));
    assert_eq!(app.comment_cursor(), Some(0));
    assert!(command.is_none());
}

#[test]
fn m_reports_when_there_are_no_comments() {
    let mut app = detail_app();

    handle_key(&mut app, press(KeyCode::Char('m')));

    assert!(matches!(
        app.focus(),
        Focus::Detail(DetailFocus {
            origin: Origin::Panel(LeftPanel::MyWork),
            view: DetailView::Reading { .. },
            ..
        })
    ));
    assert_eq!(app.ui.status, Some(Status::NoComments));
}

#[test]
fn esc_in_comments_mode_returns_to_reading_then_leaves() {
    let mut app = detail_app_with_comments();
    handle_key(&mut app, press(KeyCode::Char('m')));

    handle_key(&mut app, press(KeyCode::Esc));
    assert!(matches!(
        app.focus(),
        Focus::Detail(DetailFocus {
            origin: Origin::Panel(LeftPanel::MyWork),
            view: DetailView::Reading { .. },
            ..
        })
    ));

    handle_key(&mut app, press(KeyCode::Esc));
    assert!(app.focus().is_panel(LeftPanel::MyWork));
}

#[test]
fn j_moves_the_comment_selection_in_comments_mode() {
    let mut app = detail_app_with_comments();
    handle_key(&mut app, press(KeyCode::Char('m')));

    handle_key(&mut app, press(KeyCode::Char('j')));

    assert_eq!(app.comment_cursor(), Some(1));
}

#[test]
fn r_replies_to_a_reply_using_the_thread_root_as_parent() {
    let mut app = detail_app_with_comments();
    handle_key(&mut app, press(KeyCode::Char('m')));
    handle_key(&mut app, press(KeyCode::Char('j')));

    handle_key(&mut app, press(KeyCode::Char('r')));

    let editor = app.editor().expect("reply editor open");
    assert!(matches!(&editor.compose, Compose::Reply { parent_id } if parent_id.as_str() == "c1"));
}

#[test]
fn r_replies_to_a_root_using_its_own_id_as_parent() {
    let mut app = detail_app_with_comments();
    handle_key(&mut app, press(KeyCode::Char('m')));

    handle_key(&mut app, press(KeyCode::Char('r')));

    let editor = app.editor().expect("reply editor open");
    assert!(matches!(&editor.compose, Compose::Reply { parent_id } if parent_id.as_str() == "c1"));
}

#[test]
fn submitting_a_reply_posts_with_the_thread_parent() {
    let mut app = detail_app_with_comments();
    handle_key(&mut app, press(KeyCode::Char('m')));
    handle_key(&mut app, press(KeyCode::Char('r')));
    handle_key(&mut app, press(KeyCode::Char('o')));
    handle_key(&mut app, press(KeyCode::Char('k')));

    let command = handle_key(&mut app, ctrl('s'));

    match command {
        Some(Effect::Api(ApiCommand::CreateComment {
            issue_id,
            body,
            parent_id: Some(parent),
            ..
        })) => {
            assert_eq!(issue_id.as_str(), "i1");
            assert_eq!(body, "ok");
            assert_eq!(parent.as_str(), "c1");
        }
        other => panic!("expected a threaded CreateComment, got {other:?}"),
    }
}

#[test]
fn commenting_from_a_view_targets_the_selected_issue_not_the_stale_detail() {
    let mut app = saved_views_app();
    handle_key(&mut app, press(KeyCode::Enter));
    load_view_feed(&mut app, "v1", vec![sample_issue("i2", "DAN2-8")]);

    app.workspace
        .set_detail(sample_detail("i1", "DAN2-7"), app.now);

    handle_key(&mut app, press(KeyCode::Char('c')));
    handle_key(&mut app, press(KeyCode::Char('k')));

    let command = handle_key(&mut app, ctrl('s'));

    match command {
        Some(Effect::Api(ApiCommand::CreateComment {
            issue_id,
            body,
            parent_id: None,
            ..
        })) => {
            assert_eq!(issue_id.as_str(), "i2");
            assert_eq!(body, "k");
        }
        other => panic!("expected CreateComment for i2, got {other:?}"),
    }
}

#[test]
fn e_opens_the_edit_editor_prefilled_with_my_comment_body() {
    let mut app = detail_app_with_comments();
    handle_key(&mut app, press(KeyCode::Char('m')));

    let command = handle_key(&mut app, press(KeyCode::Char('e')));

    let editor = app.editor().expect("edit editor open");
    assert!(matches!(&editor.compose, Compose::Edit { comment_id } if comment_id.as_str() == "c1"));
    assert_eq!(editor.text(), "root comment");
    match command {
        Some(Effect::Api(ApiCommand::LoadMembers { team_id })) if team_id.as_str() == "t_pizza" => {
        }
        other => panic!("expected LoadMembers for the mention popup, got {other:?}"),
    }
}

#[test]
fn e_refuses_to_edit_someone_elses_comment() {
    let mut app = detail_app_with_comments();
    let mut detail = app.workspace.detail().value().cloned().expect("detail");
    detail.comments[0].is_mine = false;
    app.workspace.set_detail(detail, app.now);
    handle_key(&mut app, press(KeyCode::Char('m')));

    let command = handle_key(&mut app, press(KeyCode::Char('e')));

    assert!(app.editor().is_none());
    assert_eq!(app.ui.status, Some(Status::NotYourComment));
    assert!(command.is_none());
}

#[test]
fn submitting_an_edit_updates_the_comment() {
    let mut app = detail_app_with_comments();
    handle_key(&mut app, press(KeyCode::Char('m')));
    handle_key(&mut app, press(KeyCode::Char('e')));
    handle_key(&mut app, press(KeyCode::Char('!')));

    let command = handle_key(&mut app, ctrl('s'));

    match command {
        Some(Effect::Api(ApiCommand::UpdateComment {
            issue_id,
            comment_id,
            body,
            ..
        })) => {
            assert_eq!(issue_id.as_str(), "i1");
            assert_eq!(comment_id.as_str(), "c1");
            assert_eq!(body, "root comment!");
        }
        other => panic!("expected UpdateComment, got {other:?}"),
    }
    assert!(app.editor().is_none());
}

#[test]
fn comment_edited_refetches_the_thread_from_the_top() {
    let mut app = detail_app();

    let command = apply(
        &mut app,
        Message::CommentEdited {
            id: IssueId::from_raw("i1"),
        },
    );

    assert!(app.workspace.detail().in_flight());
    match command {
        Some(Effect::Api(ApiCommand::LoadDetail {
            target,
            reveal: Reveal::Top,
        })) if target.as_str() == "i1" => {}
        other => panic!("expected LoadDetail for i1 revealing the top, got {other:?}"),
    }
}

#[test]
fn d_confirms_before_deleting_my_comment() {
    let mut app = detail_app_with_comments();
    handle_key(&mut app, press(KeyCode::Char('m')));

    let no_command = handle_key(&mut app, press(KeyCode::Char('d')));
    assert!(no_command.is_none());

    let confirm = app.confirm().expect("delete confirm open");
    match &confirm.command {
        Effect::Api(ApiCommand::DeleteComment {
            issue_id,
            comment_id,
        }) => {
            assert_eq!(issue_id.as_str(), "i1");
            assert_eq!(comment_id.as_str(), "c1");
        }
        other => panic!("expected DeleteComment, got {other:?}"),
    }

    let command = handle_key(&mut app, press(KeyCode::Char('y')));
    assert!(app.confirm().is_none());
    match command {
        Some(Effect::Api(ApiCommand::DeleteComment {
            issue_id,
            comment_id,
        })) if issue_id.as_str() == "i1" && comment_id.as_str() == "c1" => {}
        other => panic!("expected DeleteComment on confirm, got {other:?}"),
    }
}

#[test]
fn d_refuses_to_delete_someone_elses_comment() {
    let mut app = detail_app_with_comments();
    let mut detail = app.workspace.detail().value().cloned().expect("detail");
    detail.comments[0].is_mine = false;
    app.workspace.set_detail(detail, app.now);
    handle_key(&mut app, press(KeyCode::Char('m')));

    let command = handle_key(&mut app, press(KeyCode::Char('d')));

    assert!(app.confirm().is_none());
    assert_eq!(app.ui.status, Some(Status::NotYourComment));
    assert!(command.is_none());
}

#[test]
fn ctrl_d_still_pages_in_comments_mode() {
    let mut app = detail_app_with_comments();
    handle_key(&mut app, press(KeyCode::Char('m')));

    let command = handle_key(&mut app, ctrl('d'));

    assert!(app.confirm().is_none());
    assert!(command.is_none());
    assert_ne!(app.comment_cursor(), Some(0));
}

#[test]
fn comment_deleted_stays_in_comments_and_refetches() {
    let mut app = detail_app_with_comments();
    handle_key(&mut app, press(KeyCode::Char('m')));

    let command = apply(
        &mut app,
        Message::CommentDeleted {
            id: IssueId::from_raw("i1"),
        },
    );

    assert!(app.workspace.detail().in_flight());
    assert!(matches!(
        app.focus(),
        Focus::Detail(DetailFocus {
            origin: Origin::Panel(LeftPanel::MyWork),
            view: DetailView::Comments { .. },
            ..
        })
    ));
    match command {
        Some(Effect::Api(ApiCommand::LoadDetail {
            target,
            reveal: Reveal::Top,
        })) if target.as_str() == "i1" => {}
        other => panic!("expected LoadDetail for i1 revealing the top, got {other:?}"),
    }
}

#[test]
fn a_shrunk_thread_clamps_the_comment_selection() {
    let mut app = detail_app_with_comments();
    handle_key(&mut app, press(KeyCode::Char('m')));
    app.reveal_focused(Some(2));

    let mut detail = app.workspace.detail().value().cloned().expect("detail");
    detail.comments.pop();
    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(detail),
            reveal: Reveal::Top,
        },
    );

    assert_eq!(app.comment_cursor(), Some(1));
    assert!(matches!(
        app.focus(),
        Focus::Detail(DetailFocus {
            origin: Origin::Panel(LeftPanel::MyWork),
            view: DetailView::Comments { .. },
            ..
        })
    ));
}

#[test]
fn deleting_the_last_comment_falls_back_to_reading() {
    let mut app = detail_app_with_comments();
    handle_key(&mut app, press(KeyCode::Char('m')));

    let mut detail = app.workspace.detail().value().cloned().expect("detail");
    detail.comments.clear();
    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(detail),
            reveal: Reveal::Top,
        },
    );

    assert!(matches!(
        app.focus(),
        Focus::Detail(DetailFocus {
            origin: Origin::Panel(LeftPanel::MyWork),
            view: DetailView::Reading { .. },
            ..
        })
    ));
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

    let command = handle_key(&mut app, press(KeyCode::Char('c')));

    assert!(app.editor().is_some());
    match command {
        Some(Effect::Api(ApiCommand::LoadMembers { team_id })) if team_id.as_str() == "t_pizza" => {
        }
        other => panic!("expected LoadMembers for the mention popup, got {other:?}"),
    }
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
        Some(Effect::Api(ApiCommand::CreateComment {
            issue_id,
            body,
            parent_id,
            ..
        })) => {
            assert_eq!(issue_id.as_str(), "i1");
            assert_eq!(body, "a\nb");
            assert_eq!(parent_id, None);
        }
        other => panic!("expected CreateComment, got {other:?}"),
    }
    assert!(app.editor().is_none());
}

#[test]
fn mention_autocomplete_inserts_the_profile_url() {
    let mut app = detail_app();
    handle_key(&mut app, press(KeyCode::Char('c')));
    apply(
        &mut app,
        Message::MembersLoaded {
            team_id: TeamId::from_raw("t_pizza"),
            members: vec![member("danniieelg"), member("sam")],
        },
    );

    handle_key(&mut app, press(KeyCode::Char('@')));
    handle_key(&mut app, press(KeyCode::Char('d')));
    assert!(app.editor().is_some_and(|e| e.mention().is_some()));

    handle_key(&mut app, press(KeyCode::Enter));

    let editor = app.editor().expect("editor open");
    assert!(editor.mention().is_none());
    assert_eq!(
        editor.text(),
        "https://linear.app/dans-donuts/profiles/danniieelg"
    );
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

    let command = apply(
        &mut app,
        Message::CommentPosted {
            id: IssueId::from_raw("i1"),
        },
    );

    assert!(app.workspace.detail().in_flight());
    match command {
        Some(Effect::Api(ApiCommand::LoadDetail {
            target,
            reveal: Reveal::Bottom,
        })) if target.as_str() == "i1" => {}
        other => panic!("expected LoadDetail for i1 revealing the bottom, got {other:?}"),
    }
}

#[test]
fn posting_from_comments_mode_stays_in_comments_and_reveals_the_new_comment() {
    let mut app = detail_app_with_comments();
    handle_key(&mut app, press(KeyCode::Char('m')));

    let command = apply(
        &mut app,
        Message::CommentPosted {
            id: IssueId::from_raw("i1"),
        },
    );

    assert!(app.workspace.detail().in_flight());
    assert!(matches!(
        app.focus(),
        Focus::Detail(DetailFocus {
            origin: Origin::Panel(LeftPanel::MyWork),
            view: DetailView::Comments { .. },
            ..
        })
    ));
    match command {
        Some(Effect::Api(ApiCommand::LoadDetail {
            target,
            reveal: Reveal::NewestComment,
        })) if target.as_str() == "i1" => {}
        other => panic!("expected LoadDetail for i1 revealing the newest comment, got {other:?}"),
    }
}

#[test]
fn newest_comment_reveal_selects_the_new_comment() {
    let mut app = detail_app_with_comments();
    handle_key(&mut app, press(KeyCode::Char('m')));
    app.reveal_focused(Some(0));

    let mut detail = app.workspace.detail().value().cloned().expect("detail");
    detail.comments.push(linear_tui::api::Comment {
        id: CommentId::from_raw("c_new"),
        parent_id: None,
        author: Some("dan".into()),
        is_mine: true,
        body: "fresh".into(),
        created_at: linear_tui::api::Timestamp::from("2026-07-16T12:00:00Z"),
        reactions: vec![],
    });
    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(detail),
            reveal: Reveal::NewestComment,
        },
    );

    assert_eq!(app.comment_cursor(), Some(3));
}

#[test]
fn a_bottom_reveal_scrolls_to_the_new_comment() {
    let mut app = detail_app();

    app.open_detail_focus(DetailFocus::reading(
        IssueId::from_raw("i1"),
        Origin::Panel(LeftPanel::MyWork),
    ));
    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(sample_detail("i1", "DAN-1")),
            reveal: Reveal::Bottom,
        },
    );

    assert_eq!(app.reading_scroll(), Some(Scroll::Bottom));
}

#[test]
fn find_step_after_a_bottom_reveal_is_render_independent() {
    let mut without_frame = tall_reading_app();
    let mut with_frame = tall_reading_app();

    render_to_string(&mut without_frame, 60, 12);
    render_to_string(&mut with_frame, 60, 12);
    without_frame.ui.find_query = Some("needle".into());
    with_frame.ui.find_query = Some("needle".into());

    handle_key(&mut without_frame, press(KeyCode::Char('G')));
    handle_key(&mut with_frame, press(KeyCode::Char('G')));

    render_to_string(&mut with_frame, 60, 12);

    handle_key(&mut without_frame, press(KeyCode::Char('n')));
    handle_key(&mut with_frame, press(KeyCode::Char('n')));

    assert_eq!(
        scroll_line(&without_frame),
        scroll_line(&with_frame),
        "find_step must land in the same place whether or not a frame rendered after G"
    );
}

#[test]
fn a_bottom_reveal_does_not_hand_find_a_max_sentinel() {
    let mut app = detail_app();
    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(sample_detail("i1", "DAN-1")),
            reveal: Reveal::Bottom,
        },
    );
    assert_eq!(app.reading_scroll(), Some(Scroll::Bottom));

    assert_ne!(
        app.focused_selection(),
        Some(usize::MAX),
        "a bottom reveal must not masquerade as a concrete line index for find_step"
    );
}

#[test]
fn opening_a_detail_starts_at_the_top() {
    let mut app = detail_app();

    app.open_detail_focus(DetailFocus::reading(
        IssueId::from_raw("i1"),
        Origin::Panel(LeftPanel::MyWork),
    ));
    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(sample_detail("i1", "DAN-1")),
            reveal: Reveal::Top,
        },
    );

    assert_eq!(app.reading_scroll(), Some(Scroll::Top));
}

#[test]
fn picker_enter_opens_confirmation_then_applies() {
    let mut app = detail_app();
    edit(&mut app, 's');
    apply(
        &mut app,
        Message::StatesLoaded {
            team_id: TeamId::from_raw("t_pizza"),
            states: vec![state_option("s_done", "Done")],
        },
    );

    let no_commands = handle_key(&mut app, press(KeyCode::Enter));
    assert!(app.picker().is_none());
    assert!(app.confirm().is_some());
    assert!(no_commands.is_none());

    let commands = handle_key(&mut app, press(KeyCode::Char('y')));
    assert!(app.confirm().is_none());
    match commands {
        Some(Effect::Api(ApiCommand::UpdateIssue {
            id,
            update: IssueUpdate::Status(state_id),
        })) if id.as_str() == "i1" && state_id.as_str() == "s_done" => {}
        other => panic!("expected UpdateIssue with status, got {other:?}"),
    }
}

#[test]
fn confirmation_cancel_does_not_write() {
    let mut app = detail_app();
    edit(&mut app, 's');
    apply(
        &mut app,
        Message::StatesLoaded {
            team_id: TeamId::from_raw("t_pizza"),
            states: vec![state_option("s_done", "Done")],
        },
    );
    handle_key(&mut app, press(KeyCode::Enter));

    let commands = handle_key(&mut app, press(KeyCode::Char('n')));

    assert!(app.confirm().is_none());
    assert!(commands.is_none());
}

#[test]
fn priority_picker_sets_the_priority() {
    let mut app = detail_app();
    edit(&mut app, 'p');
    assert_eq!(app.picker().map(|p| &p.kind), Some(&PickerKind::Priority));

    handle_key(&mut app, press(KeyCode::Enter));
    assert!(app.confirm().is_some());

    let command = handle_key(&mut app, press(KeyCode::Char('y')));
    match command {
        Some(Effect::Api(ApiCommand::UpdateIssue {
            update: IssueUpdate::Priority(priority),
            ..
        })) => assert_eq!(priority, Priority::Urgent),
        other => panic!("expected UpdateIssue priority, got {other:?}"),
    }
}

fn label(id: &str, name: &str) -> Label {
    Label {
        id: LabelId::from_raw(id),
        name: name.into(),
        colour: Rgb::parse_hex("#ffffff"),
    }
}

#[test]
fn labels_overlay_toggles_a_batch_then_submits_once() {
    let mut app = detail_app();
    let command = edit(&mut app, 'l');

    assert!(matches!(
        command,
        Some(Effect::Api(ApiCommand::SearchLabels { query })) if query.is_empty()
    ));
    assert!(app.overlay_in_flight());

    apply(
        &mut app,
        Message::LabelsFound {
            query: String::new(),
            labels: vec![label("lbl_oven", "oven"), label("lbl_bug", "bug")],
        },
    );

    assert!(!app.overlay_in_flight());

    let overlay = app.labels().expect("labels overlay");
    assert_eq!(overlay.results().len(), 2);

    assert!(handle_key(&mut app, press(KeyCode::Char(' '))).is_none());
    handle_key(&mut app, press(KeyCode::Down));
    assert!(handle_key(&mut app, press(KeyCode::Char(' '))).is_none());

    assert!(
        app.labels()
            .is_some_and(|l| l.is_selected(&LabelId::from_raw("lbl_oven"))
                && l.is_selected(&LabelId::from_raw("lbl_bug"))),
        "both toggles held while the overlay stays open"
    );

    let command = handle_key(&mut app, press(KeyCode::Enter));
    match command {
        Some(Effect::Api(ApiCommand::UpdateIssue {
            update: IssueUpdate::Labels(ids),
            ..
        })) => assert_eq!(
            ids,
            vec![LabelId::from_raw("lbl_oven"), LabelId::from_raw("lbl_bug")]
        ),
        other => panic!("expected UpdateIssue labels, got {other:?}"),
    }

    assert!(app.labels().is_none(), "submit closes the overlay");
}

#[test]
fn labels_typing_reissues_the_search() {
    let mut app = detail_app();
    edit(&mut app, 'l');
    apply(
        &mut app,
        Message::LabelsFound {
            query: String::new(),
            labels: vec![label("lbl_oven", "oven")],
        },
    );

    let command = handle_key(&mut app, press(KeyCode::Char('b')));

    assert!(matches!(
        command,
        Some(Effect::Api(ApiCommand::SearchLabels { query })) if query == "b"
    ));
    assert!(app.overlay_in_flight());
    assert!(app.labels().is_some_and(|l| l.results().is_empty()));
}

#[test]
fn assign_picker_can_unassign() {
    let mut app = detail_app();
    edit(&mut app, 'a');

    handle_key(&mut app, press(KeyCode::Enter));
    assert!(app.confirm().is_some());

    let command = handle_key(&mut app, press(KeyCode::Char('y')));
    match command {
        Some(Effect::Api(ApiCommand::UpdateIssue {
            id,
            update: IssueUpdate::Assignee(None),
        })) if id.as_str() == "i1" => {}
        other => panic!("expected an unassign UpdateIssue, got {other:?}"),
    }
}

#[test]
fn a_warm_status_picker_opens_from_cache_without_refetching() {
    let mut app = detail_app();

    let first = edit(&mut app, 's');
    assert!(matches!(
        first,
        Some(Effect::Api(ApiCommand::LoadStates { .. }))
    ));
    apply(
        &mut app,
        Message::StatesLoaded {
            team_id: TeamId::from_raw("t_pizza"),
            states: vec![state_option("s_done", "Done")],
        },
    );
    handle_key(&mut app, press(KeyCode::Esc));

    let second = edit(&mut app, 's');
    assert!(second.is_none(), "a fresh states cache must not refetch");
    assert!(!app.overlay_in_flight());

    let picker = app.picker().expect("picker open");
    assert!(!picker.items.is_empty());
}

#[test]
fn a_failed_detail_fetch_marks_the_cell_and_shows_an_error() {
    let mut app = list_app_with_issue();
    handle_key(&mut app, press(KeyCode::Enter));
    assert!(app.workspace.detail().in_flight());

    apply(
        &mut app,
        Message::Failed {
            target: FailureTarget::Detail,
            error: RequestError::Other("boom".into()),
        },
    );

    assert!(matches!(
        app.workspace.detail().status(),
        CacheStatus::Failed(_)
    ));
    assert!(matches!(app.ui.status, Some(Status::Error(_))));
}

#[test]
fn a_failed_states_fetch_stops_the_spinner_and_retries_next_time() {
    let mut app = detail_app();

    edit(&mut app, 's');
    assert!(app.overlay_in_flight());

    apply(
        &mut app,
        Message::Failed {
            target: FailureTarget::States {
                team_id: TeamId::from_raw("t_pizza"),
            },
            error: RequestError::Other("boom".into()),
        },
    );

    assert!(matches!(
        app.workspace
            .states
            .get(&TeamId::from_raw("t_pizza"))
            .map(Remote::status),
        Some(CacheStatus::Failed(_))
    ));
    assert!(!app.overlay_in_flight());
    assert!(matches!(app.ui.status, Some(Status::Error(_))));

    handle_key(&mut app, press(KeyCode::Esc));
    let retry = edit(&mut app, 's');
    match retry {
        Some(Effect::Api(ApiCommand::LoadStates { team_id })) if team_id.as_str() == "t_pizza" => {}
        other => panic!("expected a retry LoadStates after failure, got {other:?}"),
    }
}

#[test]
fn the_assign_picker_offers_yourself_without_fetching_everyone() {
    let mut app = detail_app();
    app.workspace.session = Remote::ready(session("dan"), app.now);

    let command = edit(&mut app, 'a');

    assert!(
        command.is_none(),
        "opening the assign picker must not fetch the whole account"
    );

    let labels: Vec<&str> = app
        .picker()
        .expect("assign picker open")
        .items
        .iter()
        .map(|item| item.label.as_str())
        .collect();

    assert_eq!(labels, vec!["Unassigned", "dan"]);
}

#[test]
fn slash_in_the_assign_picker_searches_for_people() {
    let mut app = detail_app();
    edit(&mut app, 'a');

    handle_key(&mut app, press(KeyCode::Char('/')));
    for c in "cha".chars() {
        handle_key(&mut app, press(KeyCode::Char(c)));
    }
    let command = handle_key(&mut app, press(KeyCode::Enter));

    match command {
        Some(Effect::Api(ApiCommand::SearchUsers { query })) if query == "cha" => {}
        other => panic!("expected SearchUsers(cha), got {other:?}"),
    }
    assert!(app.overlay_in_flight());

    apply(
        &mut app,
        Message::UsersFound {
            query: "cha".into(),
            users: vec![member("charlieh")],
        },
    );

    assert!(!app.overlay_in_flight());

    let picker = app.picker().expect("assign picker open");
    assert_eq!(
        picker
            .items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        vec!["charlieh"]
    );
}

#[test]
fn results_for_a_stale_search_are_ignored() {
    let mut app = detail_app();
    edit(&mut app, 'a');
    handle_key(&mut app, press(KeyCode::Char('/')));
    for c in "cha".chars() {
        handle_key(&mut app, press(KeyCode::Char(c)));
    }
    handle_key(&mut app, press(KeyCode::Enter));

    apply(
        &mut app,
        Message::UsersFound {
            query: "ch".into(),
            users: vec![member("someone-else")],
        },
    );

    assert!(
        app.overlay_in_flight(),
        "the current search is still in flight"
    );

    let picker = app.picker().expect("assign picker open");
    assert!(picker.items.is_empty(), "a superseded search must not fill");
}

#[test]
fn a_failed_user_search_stops_the_picker_spinner() {
    let mut app = detail_app();
    edit(&mut app, 'a');
    handle_key(&mut app, press(KeyCode::Char('/')));
    handle_key(&mut app, press(KeyCode::Char('d')));
    handle_key(&mut app, press(KeyCode::Enter));
    assert!(app.overlay_in_flight());

    apply(
        &mut app,
        Message::Failed {
            target: FailureTarget::UserSearch,
            error: RequestError::Other("boom".into()),
        },
    );

    assert!(!app.overlay_in_flight());
    assert!(matches!(app.ui.status, Some(Status::Error(_))));
}

#[test]
fn o_opens_url_from_highlighted_issue() {
    let mut app = list_app_with_issue();
    let commands = handle_key(&mut app, press(KeyCode::Char('o')));

    match commands {
        Some(Effect::Platform(PlatformCommand::OpenUrl(url))) if url.contains("DAN2-7") => {}
        other => panic!("expected OpenUrl, got {other:?}"),
    }
}

#[test]
fn y_copies_url_from_highlighted_issue() {
    let mut app = list_app_with_issue();
    let commands = handle_key(&mut app, press(KeyCode::Char('y')));

    assert!(app.ui.status.is_some());
    match commands {
        Some(Effect::Platform(PlatformCommand::CopyToClipboard(url))) if url.contains("DAN2-7") => {
        }
        other => panic!("expected CopyToClipboard, got {other:?}"),
    }
}

#[test]
fn esc_closes_picker_without_updating() {
    let mut app = detail_app();
    edit(&mut app, 'a');
    assert!(app.picker().is_some());

    let commands = handle_key(&mut app, press(KeyCode::Esc));

    assert!(app.picker().is_none());
    assert!(commands.is_none());
}

#[test]
fn open_and_yank_do_nothing_without_a_selected_issue() {
    let mut app = App::new();
    app.focus_my_work();
    app.ui.list_state.select(None);

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
    app.ui.list_state.select(Some(2));

    handle_key(&mut app, press(KeyCode::Char('g')));
    assert!(app.prefix().is_some());

    let commands = handle_key(&mut app, press(KeyCode::Char('g')));

    assert!(app.prefix().is_none());
    assert!(commands.is_none());
    assert_eq!(app.ui.list_state.selected(), Some(0));
}

#[test]
fn capital_g_jumps_to_the_bottom() {
    let mut app = list_app_with_issues();
    app.ui.list_state.select(Some(0));

    handle_key(&mut app, press(KeyCode::Char('G')));

    assert_eq!(app.ui.list_state.selected(), Some(2));
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
    assert_eq!(
        app.input().map(|i| i.purpose.clone()),
        Some(InputPurpose::Jump)
    );

    for c in "dan2-7".chars() {
        handle_key(&mut app, press(KeyCode::Char(c)));
    }
    let commands = handle_key(&mut app, press(KeyCode::Enter));

    assert!(matches!(app.focus(), Focus::Detail(..)));
    assert!(app.workspace.detail().in_flight());
    assert!(app.input().is_none());
    match commands {
        Some(Effect::Api(ApiCommand::LoadDetail { target, .. })) if target.as_str() == "DAN2-7" => {
        }
        other => panic!("expected LoadDetail(DAN2-7), got {other:?}"),
    }
}

#[test]
fn a_detail_fetched_by_identifier_is_applied_and_reanchored_to_its_id() {
    let mut app = App::new();

    handle_key(&mut app, press(KeyCode::Char('g')));
    handle_key(&mut app, press(KeyCode::Char('i')));
    for c in "dan2-7".chars() {
        handle_key(&mut app, press(KeyCode::Char(c)));
    }
    handle_key(&mut app, press(KeyCode::Enter));

    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(sample_detail("i1", "DAN2-7")),
            reveal: Reveal::Top,
        },
    );

    assert!(
        app.open_detail().is_some(),
        "a detail asked for by identifier must still be presented"
    );
    match app.focus() {
        Focus::Detail(detail) => assert_eq!(detail.issue, IssueRef::Id(IssueId::from_raw("i1"))),
        other => panic!("expected detail focus, got {other:?}"),
    }
}

#[test]
fn a_detail_for_an_issue_no_longer_open_is_dropped() {
    let mut app = detail_app();

    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(sample_detail("i9", "DAN-9")),
            reveal: Reveal::Top,
        },
    );

    assert_eq!(
        app.open_detail().map(|detail| detail.id.as_str()),
        Some("i1"),
        "a late response for another issue must not replace the open one"
    );
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
    assert_eq!(app.ui.list_state.selected(), Some(1));

    handle_key(&mut app, press(KeyCode::Enter));
    assert!(app.find().is_none());
    assert_eq!(app.ui.find_query.as_deref(), Some("dan-2"));
}

#[test]
fn slash_finds_comments_in_comments_mode() {
    let mut app = detail_app_with_comments();
    handle_key(&mut app, press(KeyCode::Char('m')));
    assert_eq!(app.comment_cursor(), Some(0));

    handle_key(&mut app, press(KeyCode::Char('/')));
    assert!(app.find().is_some(), "/ must open find in comments mode");

    for c in "another".chars() {
        handle_key(&mut app, press(KeyCode::Char(c)));
    }

    assert_eq!(app.comment_cursor(), Some(2));
}

#[test]
fn slash_scrolls_the_reading_pane_to_a_match() {
    let mut app = detail_app_with_comments();
    assert_eq!(scroll_line(&app), 0);

    handle_key(&mut app, press(KeyCode::Char('/')));
    assert!(app.find().is_some(), "/ must open find in the reading pane");

    for c in "another root".chars() {
        handle_key(&mut app, press(KeyCode::Char(c)));
    }

    assert!(
        scroll_line(&app) > 0,
        "the reading pane should scroll to the matching line"
    );

    let matched = scroll_line(&app);
    handle_key(&mut app, press(KeyCode::Esc));
    assert_eq!(
        scroll_line(&app),
        0,
        "esc should restore the original scroll position"
    );
    assert!(matched > 0);
}

#[test]
fn n_steps_between_matches_in_the_reading_pane() {
    let mut app = detail_app_with_comments();
    app.ui.find_query = Some("root".into());

    handle_key(&mut app, press(KeyCode::Char('n')));
    let first = scroll_line(&app);
    assert!(first > 0, "expected to land on the first match");

    handle_key(&mut app, press(KeyCode::Char('n')));
    assert!(
        scroll_line(&app) > first,
        "n should advance to the next match"
    );

    handle_key(&mut app, press(KeyCode::Char('N')));
    assert_eq!(scroll_line(&app), first, "N should step back");
}

#[test]
fn n_and_capital_n_cycle_matches() {
    let mut app = list_app_with_issues();
    seed_active(
        &mut app,
        vec![
            sample_issue("i1", "DAN-1"),
            sample_issue("i2", "DAN-2"),
            sample_issue("i3", "DAN-3"),
            sample_issue("i4", "DAN-2B"),
        ],
    );
    app.ui.find_query = Some("dan-2".into());
    app.ui.list_state.select(Some(0));

    handle_key(&mut app, press(KeyCode::Char('n')));
    assert_eq!(app.ui.list_state.selected(), Some(1));

    handle_key(&mut app, press(KeyCode::Char('n')));
    assert_eq!(app.ui.list_state.selected(), Some(3));

    handle_key(&mut app, press(KeyCode::Char('N')));
    assert_eq!(app.ui.list_state.selected(), Some(1));
}

#[test]
fn esc_cancels_find_and_restores_selection() {
    let mut app = list_app_with_issues();
    app.ui.list_state.select(Some(2));

    handle_key(&mut app, press(KeyCode::Char('/')));
    for c in "dan-1".chars() {
        handle_key(&mut app, press(KeyCode::Char(c)));
    }
    assert_eq!(app.ui.list_state.selected(), Some(0));

    handle_key(&mut app, press(KeyCode::Esc));
    assert!(app.find().is_none());
    assert_eq!(app.ui.list_state.selected(), Some(2));
}

#[test]
fn find_matches_on_state_name_and_esc_exits_search() {
    let mut app = list_app_with_issues();
    seed_active(
        &mut app,
        vec![
            sample_issue("i1", "DAN-1"),
            {
                let mut issue = sample_issue("i2", "DAN-2");
                issue.state.name = "In Progress".into();
                issue
            },
            sample_issue("i3", "DAN-3"),
        ],
    );

    handle_key(&mut app, press(KeyCode::Char('/')));
    for c in "in progress".chars() {
        handle_key(&mut app, press(KeyCode::Char(c)));
    }
    handle_key(&mut app, press(KeyCode::Enter));

    assert_eq!(app.ui.find_query.as_deref(), Some("in progress"));
    assert_eq!(app.ui.list_state.selected(), Some(1));

    handle_key(&mut app, press(KeyCode::Esc));
    assert!(app.ui.find_query.is_none());
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
    assert_eq!(
        app.input().map(|i| i.purpose.clone()),
        Some(InputPurpose::Search)
    );

    for c in "oven".chars() {
        handle_key(&mut app, press(KeyCode::Char(c)));
    }
    let search = handle_key(&mut app, press(KeyCode::Enter));

    assert!(app.search().is_some());
    match search {
        Some(Effect::Api(ApiCommand::LoadFeed {
            key: FeedKey::Search(term),
            request: FeedRequest::Refresh,
        })) if term == "oven" => {}
        other => panic!("expected a search feed load for oven, got {other:?}"),
    }

    apply(
        &mut app,
        Message::FeedLoaded {
            key: FeedKey::Search("oven".into()),
            request: FeedRequest::Refresh,
            page: Page::single(vec![sample_issue("i9", "DAN2-7")]),
        },
    );
    assert_eq!(search_len(&app, "oven"), 1);

    let open = handle_key(&mut app, press(KeyCode::Enter));

    assert!(matches!(app.focus(), Focus::Detail(..)));
    assert!(app.search().is_none());
    match open {
        Some(Effect::Api(ApiCommand::LoadDetail { target, .. })) if target.as_str() == "i9" => {}
        other => panic!("expected LoadDetail(i9), got {other:?}"),
    }
}

#[test]
fn reloading_a_detail_opened_from_search_refreshes_my_work_not_recent() {
    let mut app = App::new();
    handle_key(&mut app, press(KeyCode::Char('g')));
    handle_key(&mut app, press(KeyCode::Char('s')));
    for c in "oven".chars() {
        handle_key(&mut app, press(KeyCode::Char(c)));
    }
    handle_key(&mut app, press(KeyCode::Enter));

    apply(
        &mut app,
        Message::FeedLoaded {
            key: FeedKey::Search("oven".into()),
            request: FeedRequest::Refresh,
            page: Page::single(vec![sample_issue("i9", "DAN2-7")]),
        },
    );
    handle_key(&mut app, press(KeyCode::Enter));
    assert!(matches!(app.focus(), Focus::Detail(..)));

    let active = app.active_feed_key().expect("an active MyWork feed");
    let commands = effects(handle_key_all(&mut app, press(KeyCode::Char('r'))));

    let feeds: Vec<&FeedKey> = commands
        .iter()
        .filter_map(|command| match command {
            Effect::Api(ApiCommand::LoadFeed { key, .. }) => Some(key),
            _ => None,
        })
        .collect();

    assert_eq!(
        feeds,
        vec![&active],
        "a search-opened detail reloads the MyWork feed its origin resolves to"
    );
    assert!(commands
        .iter()
        .any(|command| matches!(command, Effect::Api(ApiCommand::LoadDetail { .. }))));
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
        Message::FeedLoaded {
            key: FeedKey::Search("oven".into()),
            request: FeedRequest::Refresh,
            page: Page::single(vec![
                sample_issue("i8", "DAN-1"),
                sample_issue("i9", "DAN-2"),
            ]),
        },
    );

    handle_key(&mut app, press(KeyCode::Enter));
    assert!(matches!(app.focus(), Focus::Detail(..)));
    assert!(app.search().is_none());

    handle_key(&mut app, press(KeyCode::Esc));
    assert!(app.search().is_some());
    assert_eq!(search_len(&app, "oven"), 2);
}

#[test]
fn esc_from_a_list_opened_detail_goes_home_not_search() {
    let mut app = list_app_with_issue();

    handle_key(&mut app, press(KeyCode::Enter));
    assert!(matches!(app.focus(), Focus::Detail(..)));

    handle_key(&mut app, press(KeyCode::Esc));
    assert!(app.focus().is_panel(LeftPanel::MyWork));
    assert!(app.search().is_none());
}

#[test]
fn esc_from_a_detail_searched_inside_a_view_returns_to_the_search() {
    let mut app = saved_views_app();
    handle_key(&mut app, press(KeyCode::Enter));
    assert!(app.focus().is_view());

    handle_key(&mut app, press(KeyCode::Char('g')));
    handle_key(&mut app, press(KeyCode::Char('s')));
    for c in "oven".chars() {
        handle_key(&mut app, press(KeyCode::Char(c)));
    }
    handle_key(&mut app, press(KeyCode::Enter));
    apply(
        &mut app,
        Message::FeedLoaded {
            key: FeedKey::Search("oven".into()),
            request: FeedRequest::Refresh,
            page: Page::single(vec![sample_issue("i8", "DAN-1")]),
        },
    );
    assert!(app.search().is_some());

    handle_key(&mut app, press(KeyCode::Enter));
    assert!(matches!(app.focus(), Focus::Detail(..)));

    handle_key(&mut app, press(KeyCode::Esc));
    assert!(
        app.search().is_some(),
        "escaping a detail opened from a search inside a view must return to the search results"
    );
}

#[test]
fn transient_status_clears_on_the_next_key() {
    let mut app = list_app_with_issue();
    app.ui.status = Some(Status::Cancelled);

    handle_key(&mut app, press(KeyCode::Char('j')));

    assert!(app.ui.status.is_none());
}

#[test]
fn history_boundary_sets_no_status() {
    let mut app = App::new();
    app.open_detail_focus(DetailFocus::reading(
        IssueId::from_raw("i1"),
        Origin::Panel(LeftPanel::MyWork),
    ));
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
    assert!(app.ui.status.is_none());
}

#[test]
fn opening_a_detail_keeps_the_source_panel_expanded() {
    let mut app = list_app_with_issue();

    handle_key(&mut app, press(KeyCode::Enter));

    assert!(matches!(
        app.focus(),
        Focus::Detail(DetailFocus {
            origin: Origin::Panel(LeftPanel::MyWork),
            view: DetailView::Reading { .. },
            ..
        })
    ));
}

#[test]
fn opening_from_recently_viewed_keeps_that_panel_expanded() {
    let mut app = App::new();
    app.open_detail_focus(DetailFocus::reading(
        IssueId::from_raw("i1"),
        Origin::Panel(LeftPanel::MyWork),
    ));
    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(sample_detail("i1", "DAN-1")),
            reveal: Reveal::Top,
        },
    );

    handle_key(&mut app, press(KeyCode::Char('2')));
    assert!(app.focus().is_panel(LeftPanel::Recent));

    handle_key(&mut app, press(KeyCode::Enter));

    assert!(matches!(
        app.focus(),
        Focus::Detail(DetailFocus {
            origin: Origin::Panel(LeftPanel::Recent),
            view: DetailView::Reading { .. },
            ..
        })
    ));
}

#[test]
fn tab_and_shift_tab_walk_history_in_the_detail_pane() {
    let mut app = App::new();
    app.open_detail_focus(DetailFocus::reading(
        IssueId::from_raw("i1"),
        Origin::Panel(LeftPanel::MyWork),
    ));
    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(sample_detail("i1", "DAN-1")),
            reveal: Reveal::Top,
        },
    );
    app.open_detail_focus(DetailFocus::reading(
        IssueId::from_raw("i2"),
        Origin::Panel(LeftPanel::MyWork),
    ));
    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(sample_detail("i2", "DAN-2")),
            reveal: Reveal::Top,
        },
    );

    let back = handle_key(&mut app, press(KeyCode::BackTab));
    match back {
        Some(Effect::Api(ApiCommand::LoadDetail { target, .. })) if target.as_str() == "i1" => {}
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
        Some(Effect::Api(ApiCommand::LoadDetail { target, .. })) if target.as_str() == "i2" => {}
        other => panic!("expected Tab to load i2, got {other:?}"),
    }
}

#[test]
fn tab_outside_the_detail_pane_still_cycles_panels() {
    let mut app = App::new();

    handle_key(&mut app, press(KeyCode::Tab));

    assert!(app.focus().is_panel(LeftPanel::Recent));
}

#[test]
fn ctrl_d_and_ctrl_u_scroll_the_detail_by_half_a_page() {
    let mut app = detail_app();
    app.ui.viewport = 20;

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
    );
    assert_eq!(scroll_line(&app), 10);

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
    );
    assert_eq!(scroll_line(&app), 0);
}

#[test]
fn ctrl_d_pages_the_focused_list_without_wrapping() {
    let mut app = list_app_with_issues();
    app.ui.viewport = 4;
    app.ui.list_state.select(Some(0));

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
    );
    assert_eq!(app.ui.list_state.selected(), Some(2));

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
    );
    assert_eq!(app.ui.list_state.selected(), Some(2));
}

#[test]
fn opening_issues_records_history_and_ctrl_o_goes_back() {
    let mut app = App::new();

    app.open_detail_focus(DetailFocus::reading(
        IssueId::from_raw("i1"),
        Origin::Panel(LeftPanel::MyWork),
    ));
    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(sample_detail("i1", "DAN-1")),
            reveal: Reveal::Top,
        },
    );
    app.open_detail_focus(DetailFocus::reading(
        IssueId::from_raw("i2"),
        Origin::Panel(LeftPanel::MyWork),
    ));
    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(sample_detail("i2", "DAN-2")),
            reveal: Reveal::Top,
        },
    );

    assert_eq!(app.workspace.recently_viewed.len(), 2);
    assert_eq!(app.workspace.recently_viewed[0].id.as_str(), "i2");
    assert_eq!(app.workspace.recently_viewed[1].id.as_str(), "i1");

    let back = handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL),
    );
    match back {
        Some(Effect::Api(ApiCommand::LoadDetail { target, .. })) if target.as_str() == "i1" => {}
        other => panic!("expected Ctrl-o to load i1, got {other:?}"),
    }

    app.open_detail_focus(DetailFocus::reading(
        IssueId::from_raw("i1"),
        Origin::Panel(LeftPanel::MyWork),
    ));
    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(sample_detail("i1", "DAN-1")),
            reveal: Reveal::Top,
        },
    );
    assert_eq!(
        app.workspace.recently_viewed.len(),
        2,
        "re-viewing must not duplicate"
    );
    assert_eq!(app.workspace.recent_state.selected(), Some(1));
}

#[test]
fn enter_on_recently_viewed_reopens_the_issue() {
    let mut app = App::new();
    app.open_detail_focus(DetailFocus::reading(
        IssueId::from_raw("i1"),
        Origin::Panel(LeftPanel::MyWork),
    ));
    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(sample_detail("i1", "DAN-1")),
            reveal: Reveal::Top,
        },
    );
    app.workspace.bust_detail();
    app.focus_panel(LeftPanel::Recent);
    app.workspace.recent_state.select(Some(0));

    let commands = handle_key(&mut app, press(KeyCode::Enter));

    assert!(matches!(app.focus(), Focus::Detail(..)));
    match commands {
        Some(Effect::Api(ApiCommand::LoadDetail { target, .. })) if target.as_str() == "i1" => {}
        other => panic!("expected LoadDetail(i1), got {other:?}"),
    }
}

fn search_len(app: &App, term: &str) -> usize {
    app.workspace
        .feeds
        .get(&FeedKey::Search(term.to_string()))
        .map_or(0, |feed| feed.items().len())
}

fn seed_active(app: &mut App, issues: Vec<IssueSummary>) {
    let ViewKind::Issues(filter) = &app.active_view().kind else {
        return;
    };
    let key = FeedKey::Issues(filter.clone());
    app.workspace
        .feeds
        .insert(key, Feed::ready(Page::single(issues), app.now));
}

#[test]
fn states_for_another_team_do_not_fill_the_status_picker() {
    let mut app = detail_app();
    edit(&mut app, 's');
    assert_eq!(app.picker().map(|p| &p.kind), Some(&PickerKind::Status));
    assert!(app.overlay_in_flight());

    apply(
        &mut app,
        Message::StatesLoaded {
            team_id: TeamId::from_raw("t_other"),
            states: vec![state_option("s_done", "Done")],
        },
    );

    assert!(app.overlay_in_flight());

    let picker = app.picker().expect("picker still open");
    assert!(
        picker.items.is_empty(),
        "states for another team must not fill this issue's picker"
    );
}

#[test]
fn the_spinner_ticks_while_a_status_picker_fetches() {
    let mut app = detail_app();
    edit(&mut app, 's');
    assert!(app.overlay_in_flight());
    assert!(
        !app.workspace.detail().in_flight(),
        "the background detail is settled, so only the picker can drive the spinner"
    );

    let before = app.ui.spinner.glyph();
    let now = app.now;

    assert_eq!(tick(&mut app, now), Redraw::Needed);
    assert_ne!(app.ui.spinner.glyph(), before);
}

#[test]
fn members_for_another_team_do_not_fill_the_editor() {
    let mut app = detail_app();
    handle_key(&mut app, press(KeyCode::Char('c')));
    assert!(app.editor().is_some());

    apply(
        &mut app,
        Message::MembersLoaded {
            team_id: TeamId::from_raw("t_other"),
            members: vec![member("sam")],
        },
    );

    assert!(
        app.editor().expect("editor open").candidates("").is_empty(),
        "members for another team must not fill this issue's editor"
    );
}

#[test]
fn a_failed_comment_post_reopens_the_editor_with_the_draft() {
    let mut app = detail_app();
    handle_key(&mut app, press(KeyCode::Char('c')));
    for c in "hello".chars() {
        handle_key(&mut app, press(KeyCode::Char(c)));
    }

    let command = handle_key(&mut app, ctrl('s')).expect("the post goes out");
    let Effect::Api(posted) = command else {
        panic!("expected an api command");
    };
    assert!(app.editor().is_none(), "the editor closes while posting");

    apply(
        &mut app,
        Message::Failed {
            target: posted.failure_target(),
            error: RequestError::Other("boom".into()),
        },
    );

    assert_eq!(
        app.editor().map(|editor| editor.text()),
        Some("hello".to_string()),
        "a rejected draft comes back rather than being thrown away"
    );
    assert!(matches!(app.ui.status, Some(Status::Error(_))));
}

#[test]
fn a_comment_rejected_with_a_401_reopens_the_editor_without_wedging_members() {
    let mut app = detail_app();
    let team = TeamId::from_raw("t_pizza");

    app.session.upsert_account(Account {
        workspace_key: "ws".into(),
        org_name: "Dan".into(),
        credential: Credential::OAuth(OAuthToken::new(
            "access".into(),
            Some("refresh".into()),
            None,
        )),
    });
    assert!(app.session.activate("ws"));

    handle_key(&mut app, press(KeyCode::Char('c')));
    apply(
        &mut app,
        Message::Failed {
            target: FailureTarget::Members {
                team_id: team.clone(),
            },
            error: RequestError::Other("members boom".into()),
        },
    );

    for c in "hello".chars() {
        handle_key(&mut app, press(KeyCode::Char(c)));
    }

    let posted = match handle_key(&mut app, ctrl('s')) {
        Some(Effect::Api(command)) => command,
        other => panic!("expected the post to go out, got {other:?}"),
    };

    let command = apply_all(
        &mut app,
        Message::Failed {
            target: posted.failure_target(),
            error: RequestError::Unauthorised("401".into()),
        },
    );

    assert_eq!(
        app.editor().map(|editor| editor.text()),
        Some("hello".to_string())
    );
    assert!(matches!(
        command,
        Commands::Runtime(RuntimeCommand::RefreshToken { .. })
    ));
    assert!(
        !app.workspace
            .members
            .get(&team)
            .is_some_and(Remote::in_flight),
        "a runtime step cannot carry the paired LoadMembers, so the arm must not begin the cell"
    );
}

#[test]
fn a_failed_comment_post_is_dropped_once_the_user_has_moved_on() {
    let mut app = detail_app();
    handle_key(&mut app, press(KeyCode::Char('c')));
    handle_key(&mut app, press(KeyCode::Char('h')));

    let command = handle_key(&mut app, ctrl('s')).expect("the post goes out");
    let Effect::Api(posted) = command else {
        panic!("expected an api command");
    };

    handle_key(&mut app, press(KeyCode::Char('?')));

    apply(
        &mut app,
        Message::Failed {
            target: posted.failure_target(),
            error: RequestError::Other("boom".into()),
        },
    );

    assert!(app.editor().is_none(), "the user has moved on");
    assert!(matches!(app.ui.status, Some(Status::Error(_))));
}

#[test]
fn a_members_reply_landing_mid_mention_resets_the_selection() {
    let mut app = detail_app();
    handle_key(&mut app, press(KeyCode::Char('c')));
    apply(
        &mut app,
        Message::MembersLoaded {
            team_id: TeamId::from_raw("t_pizza"),
            members: vec![member("dan"), member("sam")],
        },
    );

    handle_key(&mut app, press(KeyCode::Char('@')));
    handle_key(&mut app, press(KeyCode::Down));
    assert_eq!(mention_selection(&app), Some(1));

    apply(
        &mut app,
        Message::MembersLoaded {
            team_id: TeamId::from_raw("t_pizza"),
            members: vec![member("sam"), member("dan")],
        },
    );

    assert_eq!(
        mention_selection(&app),
        Some(0),
        "a swapped candidate list must not leave the old index pointing at a new person"
    );
}

fn mention_selection(app: &App) -> Option<usize> {
    app.editor()?.mention()?.state.selected()
}

#[test]
fn acting_during_a_detail_fetch_targets_the_descended_issue() {
    let mut app = App::new();
    app.focus_my_work();
    seed_active(&mut app, vec![sample_issue("i2", "DAN-2")]);
    app.ui.list_state.select(Some(0));
    app.workspace.recently_viewed = vec![sample_issue("i1", "DAN-1")];
    app.focus_panel(LeftPanel::Recent);
    app.workspace.recent_state.select(Some(0));

    handle_key(&mut app, press(KeyCode::Enter));
    assert!(matches!(app.focus(), Focus::Detail(..)));
    assert!(app.open_detail().is_none(), "the detail is still loading");

    let command = handle_key(&mut app, press(KeyCode::Char('y')));
    match command {
        Some(Effect::Platform(PlatformCommand::CopyToClipboard(url))) => assert!(
            url.contains("DAN-1"),
            "acting must target the descended issue, not another panel's selection, got {url}"
        ),
        other => panic!("expected a yank for the descended issue, got {other:?}"),
    }
}

#[test]
fn a_reply_to_a_reply_is_shown_and_actionable() {
    let mut app = detail_app();
    let mut detail = app.workspace.detail().value().cloned().expect("detail");
    detail.comments = vec![
        comment("c1", None, "root"),
        comment("c1a", Some("c1"), "reply"),
        comment("c1a1", Some("c1a"), "reply to a reply"),
    ];
    app.workspace.set_detail(detail, app.now);

    let threaded = app.open_detail().expect("detail").threaded_comments();
    assert_eq!(threaded.len(), 3, "a reply to a reply must be shown");
    assert_eq!(threaded[2].depth, 2);
    assert_eq!(threaded[2].comment.id.as_str(), "c1a1");

    handle_key(&mut app, press(KeyCode::Char('m')));
    handle_key(&mut app, press(KeyCode::Char('G')));
    handle_key(&mut app, press(KeyCode::Char('r')));

    let editor = app
        .editor()
        .expect("reply editor open for the deepest comment");
    assert!(matches!(&editor.compose, Compose::Reply { parent_id } if parent_id.as_str() == "c1a"));
}

#[test]
fn updating_an_issue_revalidates_the_list_in_place_without_blanking_it() {
    let mut app = App::new();
    app.focus_my_work();
    let key = app.active_feed_key().unwrap();
    app.workspace.feeds.insert(
        key.clone(),
        Feed::ready(Page::single(vec![sample_issue("i1", "DAN-1")]), app.now),
    );

    apply(
        &mut app,
        Message::IssueUpdated {
            id: IssueId::from_raw("i1"),
        },
    );

    assert_eq!(
        app.active_issues().len(),
        1,
        "invalidation must revalidate in place, not blank the visible list"
    );
    assert!(
        app.workspace.feeds.get(&key).unwrap().in_flight(),
        "the feed should be revalidating"
    );
}

#[test]
fn a_second_refresh_does_not_race_a_first_still_in_flight() {
    let mut app = list_app_with_issue();

    let first = handle_key(&mut app, press(KeyCode::Char('r')));
    assert!(matches!(
        first,
        Some(Effect::Api(ApiCommand::LoadFeed {
            request: FeedRequest::Refresh,
            ..
        }))
    ));

    let second = handle_key(&mut app, press(KeyCode::Char('r')));
    assert!(
        second.is_none(),
        "a second refresh while one is in flight must not put a second request on the wire"
    );
}

#[test]
fn updating_another_issue_leaves_the_open_detail_untouched() {
    let mut app = detail_app();
    assert!(!app.workspace.detail().in_flight());

    let command = effects(apply_all(
        &mut app,
        Message::IssueUpdated {
            id: IssueId::from_raw("i2"),
        },
    ));

    assert!(
        !app.workspace.detail().in_flight(),
        "updating a different issue must not revalidate the open detail"
    );
    assert!(
        !reloads_detail(&command),
        "an unrelated update must not fetch the open detail"
    );
}

#[test]
fn a_comment_result_for_another_issue_does_not_reload_the_open_detail() {
    let mut app = detail_app();

    let command = effects(apply_all(
        &mut app,
        Message::CommentPosted {
            id: IssueId::from_raw("i2"),
        },
    ));

    assert!(!app.workspace.detail().in_flight());
    assert!(!reloads_detail(&command));
}

#[test]
fn a_stale_detail_reply_settles_the_revalidating_cell() {
    let mut app = detail_app();
    app.workspace.begin_detail();
    handle_key(&mut app, press(KeyCode::Esc));
    assert!(app.focus().is_panel(LeftPanel::MyWork));
    assert!(app.workspace.detail().in_flight());

    apply(
        &mut app,
        Message::DetailLoaded {
            detail: Box::new(sample_detail("i1", "DAN2-7")),
            reveal: Reveal::Top,
        },
    );

    assert!(
        !app.workspace.detail().in_flight(),
        "an in-flight cell must settle even when its reply no longer matches the focus"
    );
}

fn reloads_detail(effects: &Effects) -> bool {
    effects
        .iter()
        .any(|effect| matches!(effect, Effect::Api(ApiCommand::LoadDetail { .. })))
}

fn load_view_feed(app: &mut App, id: &str, issues: Vec<IssueSummary>) {
    apply(
        app,
        Message::FeedLoaded {
            key: FeedKey::View(ViewId::from_raw(id)),
            request: FeedRequest::Refresh,
            page: Page::single(issues),
        },
    );
}

fn list_app_with_issue() -> App {
    let mut app = App::new();
    app.focus_my_work();
    seed_active(&mut app, vec![sample_issue("i1", "DAN2-7")]);
    app.ui.list_state.select(Some(0));
    app
}

fn list_app_with_issues() -> App {
    let mut app = App::new();
    app.focus_my_work();
    seed_active(
        &mut app,
        vec![
            sample_issue("i1", "DAN-1"),
            sample_issue("i2", "DAN-2"),
            sample_issue("i3", "DAN-3"),
        ],
    );
    app.ui.list_state.select(Some(0));
    app
}

fn tall_reading_app() -> App {
    let mut app = detail_app();
    let mut detail = app.workspace.detail().value().cloned().expect("detail");
    detail.description = Some(vec!["needle"; 40].join("\n"));
    app.workspace.set_detail(detail, app.now);
    app
}

fn detail_app() -> App {
    let mut app = list_app_with_issue();
    app.open_detail_focus(DetailFocus {
        issue: IssueRef::Id(IssueId::from_raw("i1")),
        origin: Origin::Panel(LeftPanel::MyWork),
        view: DetailView::reading(),
        summary: None,
    });
    app.workspace
        .set_detail(sample_detail("i1", "DAN2-7"), app.now);
    app
}

fn saved_view(id: &str, name: &str) -> linear_tui::api::SavedView {
    linear_tui::api::SavedView {
        id: ViewId::from_raw(id),
        name: name.into(),
    }
}

fn saved_views_app() -> App {
    let mut app = App::new();
    app.focus_panel(LeftPanel::SavedViews);
    apply(
        &mut app,
        Message::CustomViewsLoaded(vec![
            saved_view("v1", "Urgent"),
            saved_view("v2", "Menu ideas"),
        ]),
    );
    app
}

fn sample_issue(id: &str, identifier: &str) -> linear_tui::api::IssueSummary {
    linear_tui::api::IssueSummary {
        id: IssueId::from_raw(id),
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
        team_id: TeamId::from_raw("t_pizza"),
        updated_at: linear_tui::api::Timestamp::default(),
    }
}

fn stamped_issue(id: &str, identifier: &str, updated_at: i64) -> linear_tui::api::IssueSummary {
    linear_tui::api::IssueSummary {
        updated_at: Timestamp::from_epoch(updated_at),
        ..sample_issue(id, identifier)
    }
}

fn sample_detail(id: &str, identifier: &str) -> linear_tui::api::IssueDetail {
    linear_tui::api::IssueDetail {
        id: IssueId::from_raw(id),
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
        reactions: vec![],
        branch_name: format!("dan/{}", identifier.to_lowercase()),
        team_id: TeamId::from_raw("t_pizza"),
        updated_at: linear_tui::api::Timestamp::default(),
    }
}

fn comment(id: &str, parent: Option<&str>, body: &str) -> linear_tui::api::Comment {
    linear_tui::api::Comment {
        id: CommentId::from_raw(id),
        parent_id: parent.map(CommentId::from_raw),
        author: Some("dan".into()),
        is_mine: true,
        body: body.into(),
        created_at: linear_tui::api::Timestamp::from("2026-07-16T09:00:00Z"),
        reactions: vec![],
    }
}

#[test]
fn an_auth_failure_prompts_reauth_until_the_session_recovers() {
    let mut app = signed_in();

    apply(
        &mut app,
        Message::Failed {
            target: FailureTarget::CustomViews,
            error: RequestError::Unauthorised("Linear returned HTTP 401".into()),
        },
    );
    assert_eq!(
        app.session.auth(),
        AuthState::Unauthenticated,
        "an auth failure should prompt re-auth"
    );

    apply(&mut app, Message::SessionLoaded(session("dan")));
    assert_eq!(
        app.session.auth(),
        AuthState::Authenticated,
        "a successful session means auth is healthy again"
    );
}

#[test]
fn a_non_auth_failure_leaves_us_authenticated() {
    let mut app = signed_in();

    apply(
        &mut app,
        Message::Failed {
            target: FailureTarget::CustomViews,
            error: RequestError::Other("Linear returned HTTP 500".into()),
        },
    );

    assert_eq!(app.session.auth(), AuthState::Authenticated);
}

#[test]
fn a_failed_session_load_is_retryable_rather_than_ephemeral() {
    assert!(
        matches!(
            ApiCommand::LoadSession.failure_target(),
            FailureTarget::Session
        ),
        "a session load must route its failure to a named, retryable target"
    );

    let mut app = list_app_with_issue();
    assert!(app.workspace.session.value().is_none());

    apply(
        &mut app,
        Message::Failed {
            target: ApiCommand::LoadSession.failure_target(),
            error: RequestError::Other("Linear returned HTTP 500".into()),
        },
    );
    assert!(app.workspace.session.is_failed());
    assert!(matches!(app.ui.status, Some(Status::Error(_))));

    let command = effects(handle_key_all(&mut app, press(KeyCode::Char('r'))));
    let retried = command
        .iter()
        .any(|c| matches!(c, Effect::Api(ApiCommand::LoadSession)));
    assert!(
        retried,
        "a failed session must be retried by reload, not stranded on 'connecting…'"
    );
}

fn oauth_app(refresh_token: Option<&str>, expires_at: Option<i64>) -> App {
    let mut app = App::new();
    app.now = Timestamp::from_epoch(1_000);
    app.session.upsert_account(Account {
        workspace_key: "ws".into(),
        org_name: "Dan".into(),
        credential: Credential::OAuth(OAuthToken::new(
            "access".into(),
            refresh_token.map(String::from),
            expires_at,
        )),
    });
    assert!(app.session.activate("ws"));
    app
}

fn auth_failure() -> Message {
    Message::Failed {
        target: FailureTarget::CustomViews,
        error: RequestError::Unauthorised("Linear returned HTTP 401".into()),
    }
}

#[test]
fn an_auth_failure_with_a_refresh_token_refreshes_rather_than_prompting() {
    let mut app = oauth_app(Some("refresh"), None);

    let command = apply_all(&mut app, auth_failure());

    assert!(matches!(app.session.auth(), AuthState::Refreshing { .. }));
    assert!(matches!(
        command,
        Commands::Runtime(RuntimeCommand::RefreshToken { .. })
    ));
}

#[test]
fn a_token_refreshed_after_a_workspace_switch_updates_the_old_account_without_reconnecting() {
    let mut app = oauth_app(Some("refresh"), None);
    app.session.upsert_account(Account {
        workspace_key: "other".into(),
        org_name: "Other".into(),
        credential: Credential::PersonalKey("k".into()),
    });
    app.session.begin_refresh(app.now);
    assert!(app.session.activate("other"));

    let command = apply_all(
        &mut app,
        Message::TokenRefreshed {
            workspace_key: "ws".into(),
            credential: Credential::OAuth(OAuthToken::new(
                "new-access".into(),
                Some("new-refresh".into()),
                Some(9_999),
            )),
        },
    );

    assert!(
        command.is_empty(),
        "a refresh for a workspace we have left must not reconnect"
    );

    let refreshed = app
        .session
        .accounts()
        .iter()
        .find(|account| account.workspace_key == "ws")
        .expect("the old account is still known");

    assert!(
        matches!(&refreshed.credential, Credential::OAuth(token) if token.access_token == "new-access"),
        "the new token belongs to the workspace that asked for it"
    );
    assert_eq!(
        app.session.active_workspace(),
        Some("other"),
        "the active workspace is untouched"
    );
}

#[test]
fn a_refresh_failure_for_an_inactive_workspace_does_not_expire_the_active_one() {
    let mut app = oauth_app(Some("refresh"), None);
    app.session.upsert_account(Account {
        workspace_key: "other".into(),
        org_name: "Other".into(),
        credential: Credential::PersonalKey("k".into()),
    });
    assert!(app.session.activate("other"));

    apply(
        &mut app,
        Message::RefreshFailed {
            workspace_key: "ws".into(),
        },
    );

    assert_eq!(app.session.auth(), AuthState::Authenticated);
}

#[test]
fn further_auth_failures_while_refreshing_do_not_refresh_again() {
    let mut app = oauth_app(Some("refresh"), None);
    apply_all(&mut app, auth_failure());

    let command = apply(&mut app, auth_failure());

    assert!(matches!(app.session.auth(), AuthState::Refreshing { .. }));
    assert!(command.is_none());
}

#[test]
fn an_auth_failure_without_a_refresh_token_prompts_reauth() {
    let mut app = oauth_app(None, None);

    apply(&mut app, auth_failure());

    assert_eq!(app.session.auth(), AuthState::Unauthenticated);
}

#[test]
fn a_refreshed_token_updates_the_account_and_reconnects() {
    let mut app = oauth_app(Some("refresh"), None);
    app.session.begin_refresh(app.now);

    let command = apply_all(
        &mut app,
        Message::TokenRefreshed {
            workspace_key: "ws".into(),
            credential: Credential::OAuth(OAuthToken::new(
                "new-access".into(),
                Some("new-refresh".into()),
                Some(9_999),
            )),
        },
    );

    assert_eq!(app.session.auth(), AuthState::Authenticated);
    assert!(matches!(
        command,
        Commands::Runtime(RuntimeCommand::Reconnect)
    ));
    match &app.session.accounts()[0].credential {
        Credential::OAuth(token) => assert_eq!(token.access_token, "new-access"),
        other => panic!("expected OAuth, got {other:?}"),
    }
}

#[test]
fn a_failed_refresh_prompts_reauth() {
    let mut app = oauth_app(Some("refresh"), None);
    app.session.begin_refresh(app.now);

    apply(
        &mut app,
        Message::RefreshFailed {
            workspace_key: "ws".into(),
        },
    );

    assert_eq!(app.session.auth(), AuthState::Unauthenticated);
}

#[test]
fn a_refresh_whose_reply_never_lands_expires_on_tick_and_forces_a_repaint() {
    let mut app = oauth_app(Some("refresh"), None);
    app.session.begin_refresh(Timestamp::from_epoch(0));
    assert!(matches!(app.session.auth(), AuthState::Refreshing { .. }));

    let redraw = tick(&mut app, Timestamp::from_epoch(10_000));

    assert_eq!(
        app.session.auth(),
        AuthState::Unauthenticated,
        "a refresh with no settling reply must not stay Refreshing forever"
    );
    assert_eq!(
        redraw,
        Redraw::Needed,
        "expiring a stuck refresh must repaint so 'press w' appears without a keypress"
    );
}

#[test]
fn proactive_refresh_fires_only_when_the_token_is_near_expiry() {
    let mut not_expiring = oauth_app(Some("refresh"), Some(1_000 + 3_600));
    assert!(not_expiring.maybe_refresh_token().is_empty());
    assert_eq!(not_expiring.session.auth(), AuthState::Authenticated);

    let mut expiring = oauth_app(Some("refresh"), Some(1_000 + 30));
    let command = expiring.maybe_refresh_token();
    assert!(matches!(
        command,
        Commands::Runtime(RuntimeCommand::RefreshToken { .. })
    ));
    assert!(matches!(
        expiring.session.auth(),
        AuthState::Refreshing { .. }
    ));
}

#[test]
fn an_async_error_survives_an_unrelated_keystroke() {
    let mut app = list_app_with_issue();

    apply(
        &mut app,
        Message::Failed {
            target: FailureTarget::CustomViews,
            error: RequestError::Other("boom".into()),
        },
    );
    assert!(matches!(app.ui.status, Some(Status::Error(_))));

    handle_key(&mut app, press(KeyCode::Insert));

    assert!(
        matches!(app.ui.status, Some(Status::Error(_))),
        "a no-op keystroke must not wipe an async error"
    );
}

fn session(name: &str) -> linear_tui::api::Session {
    linear_tui::api::Session {
        user: linear_tui::api::User {
            is_me: true,
            ..member(name)
        },
        org_name: "Dan's Donuts".into(),
        org_url_key: "dans-donuts".into(),
    }
}

fn member(name: &str) -> linear_tui::api::User {
    linear_tui::api::User {
        id: UserId::from_raw(format!("u_{name}")),
        name: name.into(),
        display_name: name.into(),
        url: format!("https://linear.app/dans-donuts/profiles/{name}"),
        is_me: false,
    }
}

fn state_option(id: &str, name: &str) -> linear_tui::api::StateOption {
    linear_tui::api::StateOption {
        id: StateId::from_raw(id),
        name: name.into(),
        state_type: linear_tui::api::StateType::Completed,
    }
}

fn detail_app_with_comments() -> App {
    let mut app = detail_app();
    let mut detail = app.workspace.detail().value().cloned().expect("detail");
    detail.comments = vec![
        comment("c1", None, "root comment"),
        comment("c1a", Some("c1"), "a reply"),
        comment("c2", None, "another root"),
    ];
    app.workspace.set_detail(detail, app.now);
    app
}
