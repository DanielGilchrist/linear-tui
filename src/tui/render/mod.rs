use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::Text,
    widgets::ListState,
    Frame,
};

use super::action;
use super::app::{Active, App, Ui, Zoom};
use super::feed::{Feed, FeedKey, FeedStore};
use super::focus::{DetailView, Focus, LeftPanel, Scroll, PANELS};
use super::layout;
use super::overlay::{Menu, ModalOverlay, Overlay, Picker, PrefixUnder, Search};
use super::spinner::Spinner;
use super::view::{ViewKind, Views};
use super::workspace::WorkspaceData;
use crate::api::{IssueDetail, IssueSummary, Timestamp};

mod format;
mod overlays;
mod snapshot;
mod surfaces;
mod theme;
mod widgets;

pub use snapshot::{render_styled_to_string, render_to_string};

use surfaces::Viewport;
use theme::Emphasis;
use widgets::text_panel;

const LEFT_PCT: u16 = 38;
const COLLAPSED_PEEK: usize = 2;

pub fn detail_line_texts(
    detail: &IssueDetail,
    rendered: &super::workspace::RenderedDetail,
    now: Timestamp,
    selected: Option<usize>,
) -> Vec<String> {
    surfaces::detail::detail_text(detail, rendered, now, selected).line_texts()
}

pub fn render(app: &mut App, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(frame.area());

    let body = chunks[0];
    let footer = chunks[1];

    match app.ui.zoom {
        Zoom::Full => render_zoomed(app, frame, body),
        Zoom::Normal => {
            let [left, right] = layout::split_horizontal(body, LEFT_PCT);

            render_left(app, frame, left);
            render_right(app, frame, right);
        }
    }

    render_footer(app, frame, footer);
    let overlay_in_flight = app.overlay_in_flight();
    let mut overlay = app.take_overlay();
    render_overlay(
        &mut overlay,
        &app.workspace.feeds,
        OverlayProps {
            in_flight: overlay_in_flight,
            spinner: app.ui.spinner,
        },
        frame,
    );
    app.set_overlay(overlay);
}

fn render_zoomed(app: &mut App, frame: &mut Frame, area: Rect) {
    let Viewport(rows) = match app.focus().clone() {
        Focus::MyWork => render_panel(app, frame, area, LeftPanel::MyWork, Emphasis::Focused),
        Focus::Recent => render_panel(app, frame, area, LeftPanel::Recent, Emphasis::Focused),
        Focus::SavedViews => {
            render_panel(app, frame, area, LeftPanel::SavedViews, Emphasis::Focused)
        }
        Focus::View(_) => render_view_surface(app, frame, area, Emphasis::Focused),
        Focus::Teams => render_panel(app, frame, area, LeftPanel::Teams, Emphasis::Focused),
        Focus::Detail(..) => render_detail_pane(app, frame, area, Emphasis::Focused),
    };

    app.ui.viewport = rows;
}

#[derive(Clone, Copy)]
struct OverlayProps {
    in_flight: bool,
    spinner: Spinner,
}

fn render_overlay(
    overlay: &mut Overlay,
    feeds: &FeedStore,
    props: OverlayProps,
    frame: &mut Frame,
) {
    use ratatui::widgets::Clear;

    let frame_area = frame.area();
    let OverlayProps { in_flight, spinner } = props;

    match overlay {
        Overlay::Picker(picker) => render_picker(picker, in_flight, spinner, frame),
        Overlay::Confirm(confirm) => {
            let area = overlays::confirm::area(frame_area);
            frame.render_widget(Clear, area);
            overlays::confirm::render(confirm, frame, area);
        }
        Overlay::Menu(menu) => render_menu(menu, frame),
        Overlay::Input(input) => {
            let area = overlays::input::area(frame_area);
            frame.render_widget(Clear, area);
            overlays::input::render(input, frame, area);
        }
        Overlay::Editor(editor) => {
            let area = overlays::editor::area(frame_area);
            frame.render_widget(Clear, area);
            overlays::editor::render(editor, frame, area);
        }
        Overlay::Search(search) => render_search(search, feeds, spinner, frame),
        Overlay::Prefix(prefix) => match &mut prefix.under {
            PrefixUnder::Modal(ModalOverlay::Picker(picker)) => {
                render_picker(picker, in_flight, spinner, frame)
            }
            PrefixUnder::Modal(ModalOverlay::Menu(menu)) => render_menu(menu, frame),
            PrefixUnder::Modal(ModalOverlay::Search(search)) => {
                render_search(search, feeds, spinner, frame)
            }
            PrefixUnder::Browse => {
                let area = overlays::prefix::area(frame_area, prefix.keymap);

                frame.render_widget(Clear, area);
                overlays::prefix::render(prefix.keymap, prefix.title, frame, area);
            }
        },
        Overlay::Reactions(reactions) => {
            let area = overlays::reactions::area(reactions, frame_area);

            frame.render_widget(Clear, area);
            overlays::reactions::render(reactions, frame, area);
        }
        Overlay::Labels(labels) => {
            let area = overlays::labels::area(frame_area);

            frame.render_widget(Clear, area);
            overlays::labels::render(labels, spinner, frame, area);
        }
        Overlay::Workspaces(workspaces) => {
            let area = overlays::workspaces::area(frame_area);

            frame.render_widget(Clear, area);
            overlays::workspaces::render(workspaces, frame, area);
        }
        Overlay::Find(_) | Overlay::None => {}
    }
}

fn render_picker(picker: &mut Picker, in_flight: bool, spinner: Spinner, frame: &mut Frame) {
    let area = overlays::picker::area(frame.area());
    frame.render_widget(ratatui::widgets::Clear, area);
    overlays::picker::render(picker, in_flight, spinner, frame, area);
}

fn render_menu(menu: &mut Menu, frame: &mut Frame) {
    let area = overlays::menu::area(frame.area());
    frame.render_widget(ratatui::widgets::Clear, area);
    overlays::menu::render(menu, frame, area);
}

fn render_search(search: &mut Search, feeds: &FeedStore, spinner: Spinner, frame: &mut Frame) {
    let feed = feeds.get(&FeedKey::Search(search.query.clone()));
    let results: &[IssueSummary] = feed.map_or(&[], |feed| feed.items());
    let feed_data = overlays::search::SearchFeed {
        results,
        status: feed.map(Feed::status),
        appending: feed.is_some_and(|feed| feed.appending()),
    };

    let area = overlays::search::area(frame.area());
    frame.render_widget(ratatui::widgets::Clear, area);
    overlays::search::render(search, feed_data, spinner, frame, area);
}

fn selected_issue<'w>(
    workspace: &'w WorkspaceData,
    views: &Views,
    view_state: &ListState,
    list_state: &ListState,
) -> Option<&'w IssueSummary> {
    let view = views.active(view_state);

    match &view.kind {
        ViewKind::Issues(_) => list_state
            .selected()
            .and_then(|index| workspace.issues_for(view).get(index)),
        ViewKind::Inbox => None,
    }
}

fn work_preview<'a>(
    workspace: &'a WorkspaceData,
    views: &Views,
    view_state: &ListState,
    list_state: &ListState,
) -> surfaces::detail::Preview<'a> {
    use surfaces::detail::Preview;

    match &views.active(view_state).kind {
        ViewKind::Issues(_) => {
            Preview::Issue(selected_issue(workspace, views, view_state, list_state))
        }
        ViewKind::Inbox => Preview::Notification(
            list_state
                .selected()
                .and_then(|index| workspace.inbox.items().get(index)),
        ),
    }
}

fn ready_detail<'w>(
    workspace: &'w WorkspaceData,
    views: &Views,
    view_state: &ListState,
    list_state: &ListState,
) -> Option<&'w IssueDetail> {
    let detail = workspace.detail().value()?;
    let selected = selected_issue(workspace, views, view_state, list_state)?;

    (detail.id == selected.id).then_some(detail)
}

fn render_panel(
    app: &mut App,
    frame: &mut Frame,
    rect: Rect,
    panel: LeftPanel,
    emphasis: Emphasis,
) -> Viewport {
    match panel {
        LeftPanel::MyWork => {
            let App {
                workspace,
                ui:
                    Ui {
                        views,
                        view_state,
                        list_state,
                        spinner,
                        ..
                    },
                ..
            } = &mut *app;
            let active = view_state.selected().unwrap_or(0);
            let view = views.active(view_state);

            let content = match &view.kind {
                ViewKind::Issues(_) => surfaces::my_work::MyWorkContent::Issues {
                    issues: workspace.issues_for(view),
                },
                ViewKind::Inbox => surfaces::my_work::MyWorkContent::Inbox {
                    items: workspace.inbox.items(),
                },
            };

            let status = workspace.feed_status_for(view);
            let appending = workspace.appending_for(view);

            surfaces::my_work::render(
                frame,
                rect,
                surfaces::my_work::MyWorkProps {
                    views: views.as_slice(),
                    active,
                    content,
                    status,
                    appending,
                    list_state,
                    emphasis,
                    spinner: *spinner,
                },
            );
        }
        LeftPanel::Recent => surfaces::recent::render(
            frame,
            rect,
            &app.workspace.recently_viewed,
            &mut app.workspace.recent_state,
            emphasis,
        ),
        LeftPanel::SavedViews => {
            let spinner = app.ui.spinner;
            surfaces::saved_views::render(
                frame,
                rect,
                &mut app.workspace.saved_views,
                emphasis,
                spinner,
            )
        }
        LeftPanel::Teams => {
            let items = app.workspace.teams.names();
            surfaces::teams::render(
                frame,
                rect,
                "Teams",
                &items,
                &mut app.workspace.teams.state,
                emphasis,
            );
        }
    }

    Viewport((rect.height as usize).saturating_sub(2))
}

fn render_view_surface(
    app: &mut App,
    frame: &mut Frame,
    area: Rect,
    emphasis: Emphasis,
) -> Viewport {
    let spinner = app.ui.spinner;
    let now = app.now;
    let (view, feeds) = app.view_render_parts();

    match view {
        Some(view) => surfaces::view::render(frame, area, feeds, view, spinner, emphasis, now),
        None => Viewport((area.height as usize).saturating_sub(2)),
    }
}

fn render_detail_pane(
    app: &mut App,
    frame: &mut Frame,
    area: Rect,
    emphasis: Emphasis,
) -> Viewport {
    let selected = app.comment_cursor();
    let scroll = app.reading_scroll().unwrap_or_default();

    let preview = work_preview(
        &app.workspace,
        &app.ui.views,
        &app.ui.view_state,
        &app.ui.list_state,
    );

    let max = surfaces::detail::render_pane(
        frame,
        area,
        app.workspace.detail(),
        app.workspace.detail_markdown(),
        app.ui.spinner,
        preview,
        surfaces::detail::ReadingProps {
            now: app.now,
            selected,
            scroll,
            emphasis,
        },
    );

    app.ui.detail_scroll_max = max;

    Viewport((area.height as usize).saturating_sub(2))
}

fn render_my_work_right(app: &mut App, frame: &mut Frame, area: Rect) {
    match ready_detail(
        &app.workspace,
        &app.ui.views,
        &app.ui.view_state,
        &app.ui.list_state,
    ) {
        Some(detail) => {
            surfaces::detail::render_reading(
                frame,
                area,
                detail,
                app.workspace.detail_markdown(),
                surfaces::detail::ReadingProps {
                    now: app.now,
                    selected: None,
                    scroll: Scroll::Top,
                    emphasis: Emphasis::Blurred,
                },
            );
        }
        None => {
            let preview = work_preview(
                &app.workspace,
                &app.ui.views,
                &app.ui.view_state,
                &app.ui.list_state,
            );
            surfaces::detail::render_work_preview(frame, area, preview, Emphasis::Blurred);
        }
    }
}

fn render_left(app: &mut App, frame: &mut Frame, area: Rect) {
    let expanded = app.focus().left();
    let constraints: Vec<Constraint> = PANELS
        .iter()
        .map(|&panel| {
            if panel == expanded {
                Constraint::Min(5)
            } else {
                let rows = app.panel(panel).len.clamp(1, COLLAPSED_PEEK);
                Constraint::Length(rows as u16 + 2)
            }
        })
        .collect();

    let rects = Layout::vertical(constraints).split(area);

    for (rect, panel) in rects.iter().zip(PANELS) {
        let focused = app.focus().is_panel(panel);
        let Viewport(rows) = render_panel(app, frame, *rect, panel, Emphasis::of_focus(focused));

        if focused {
            app.ui.viewport = rows;
        }
    }
}

fn render_right(app: &mut App, frame: &mut Frame, area: Rect) {
    match app.focus() {
        Focus::Teams => {
            let selected = app.teams().selected().map_or("", |team| team.name.as_str());
            surfaces::teams::render_placeholder(frame, area, "Teams", selected);
        }
        Focus::Recent => surfaces::recent::render_preview(frame, area, app.selected_recent()),
        Focus::SavedViews => {
            let spinner = app.ui.spinner;
            let now = app.now;
            match app.workspace.saved_views.selected_view() {
                Some(view) => {
                    let id = view.id.clone();
                    let name = view.name.clone();
                    surfaces::saved_views::render_preview(
                        frame,
                        area,
                        &app.workspace.feeds,
                        &id,
                        &name,
                        spinner,
                        now,
                    );
                }
                None => text_panel(
                    frame,
                    area,
                    "Saved Views",
                    Text::from("No saved views"),
                    Emphasis::Blurred,
                ),
            }
        }
        Focus::View(_) => {
            app.ui.viewport = render_view_surface(app, frame, area, Emphasis::Focused).0;
        }
        Focus::MyWork => render_my_work_right(app, frame, area),
        Focus::Detail(..) => {
            app.ui.viewport = render_detail_pane(app, frame, area, Emphasis::Focused).0;
        }
    }
}

fn render_footer(app: &App, frame: &mut Frame, area: Rect) {
    surfaces::footer::render(frame, area, footer_state(app));
}

fn footer_state(app: &App) -> surfaces::footer::Footer {
    use surfaces::footer::{Footer, FooterLeft};

    if let Some(find) = find_bar_state(app) {
        return Footer::Find(find);
    }

    let workspace = match app.workspace.session.value() {
        Some(session) => format!("{} · @{} ", session.org_name, session.user.display_name),
        None => "connecting… ".to_string(),
    };

    let error = app
        .ui
        .status
        .as_ref()
        .filter(|status| status.is_error())
        .map(ToString::to_string);

    let left = match (app.session.active(), error) {
        (Active::None, _) => FooterLeft::Status {
            text: "Not connected  ·  press w to add a workspace".to_string(),
            is_error: false,
        },
        (Active::Unauthenticated { .. }, _) => FooterLeft::Status {
            text: "Session expired  ·  press w to sign in again".to_string(),
            is_error: true,
        },
        (_, Some(text)) => FooterLeft::Status {
            text,
            is_error: true,
        },
        (Active::Refreshing { .. }, None) => FooterLeft::Status {
            text: "Refreshing your session…".to_string(),
            is_error: false,
        },
        (Active::Authenticated { .. }, None) => match &app.ui.status {
            Some(status) => FooterLeft::Status {
                text: status.to_string(),
                is_error: status.is_error(),
            },
            None => FooterLeft::Hint {
                text: footer_hint(app),
            },
        },
    };

    Footer::Normal { left, workspace }
}

fn find_bar_state(app: &App) -> Option<surfaces::footer::FindBar> {
    use surfaces::footer::FindBar;

    match app.overlay() {
        Overlay::Find(find) => Some(FindBar::Typing {
            query: find.query.clone(),
            total: app.focused_matches(&find.query).len(),
        }),
        Overlay::None => {
            let query = app.ui.find_query.as_deref()?;
            let matches = app.focused_matches(query);

            if matches.is_empty() {
                return Some(FindBar::NoMatches {
                    query: query.to_string(),
                });
            }

            let position = app
                .focused_selection()
                .and_then(|selected| matches.iter().position(|&index| index == selected))
                .map(|index| index + 1)
                .unwrap_or(0);

            Some(FindBar::Matches {
                query: query.to_string(),
                position,
                total: matches.len(),
            })
        }
        _ => None,
    }
}

fn footer_hint(app: &App) -> String {
    match app.overlay() {
        Overlay::Menu(_) => return action::MENU.hint_bar(action::MENU_HINTS),
        Overlay::Confirm(_) => return action::CONFIRM.hint_bar(action::CONFIRM_HINTS),
        Overlay::Picker(picker) if picker.searchable() => {
            return action::PICKER.hint_bar(action::SEARCHABLE_PICKER_HINTS)
        }
        Overlay::Picker(_) | Overlay::Search(_) => {
            return action::PICKER.hint_bar(action::PICKER_HINTS)
        }
        Overlay::Prefix(prefix) => return format!("{}   esc cancel", prefix.keymap.summary()),
        Overlay::Input(_) => return action::INPUT.hint_bar(action::INPUT_HINTS),
        Overlay::Editor(_) => return action::EDITOR.hint_bar(action::EDITOR_HINTS),
        Overlay::Reactions(_) => return action::REACTIONS.hint_bar(action::REACTIONS_HINTS),
        Overlay::Labels(_) => return action::LABELS.hint_bar(action::LABELS_HINTS),
        Overlay::Workspaces(_) => return action::WORKSPACES.hint_bar(action::WORKSPACES_HINTS),
        Overlay::Find(_) | Overlay::None => {}
    }

    let specs = match app.focus() {
        Focus::MyWork => action::MY_WORK_HINTS,
        Focus::Recent => action::RECENT_HINTS,
        Focus::SavedViews => action::SAVED_VIEWS_HINTS,
        Focus::View(_) => action::VIEW_HINTS,
        Focus::Teams => action::TEAMS_HINTS,
        Focus::Detail(detail) => match detail.view {
            DetailView::Reading { .. } => action::DETAIL_HINTS,
            DetailView::Comments { .. } => action::COMMENTS_HINTS,
        },
    };
    action::BROWSE.hint_bar(specs)
}
