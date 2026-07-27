use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::Text,
    widgets::ListState,
    Frame,
};

use super::action;
use super::app::{App, Zoom};
use super::feed::{Feed, FeedKey, FeedStore};
use super::focus::{DetailView, Focus, LeftPanel};
use super::layout;
use super::overlay::{Overlay, PrefixUnder};
use super::spinner::Spinner;
use super::view::{View, ViewKind};
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

pub fn render(app: &mut App, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(frame.area());

    let body = chunks[0];
    let footer = chunks[1];

    match app.zoom {
        Zoom::Full => render_zoomed(app, frame, body),
        Zoom::Normal => {
            let [left, right] = layout::split_horizontal(body, LEFT_PCT);

            render_left(app, frame, left);
            render_right(app, frame, right);
        }
    }

    render_footer(app, frame, footer);
    render_overlay(&mut app.overlay, &app.workspace.feeds, app.spinner, frame);

    app.time_refresh_due = earliest_time_refresh(app);
}

fn earliest_time_refresh(app: &App) -> Option<Timestamp> {
    let now = app.now;

    let issue_stamps = app
        .workspace
        .recently_viewed
        .iter()
        .chain(
            app.workspace
                .feeds
                .iter()
                .flat_map(|(_, feed)| feed.items()),
        )
        .map(|issue| issue.updated_at);

    let comment_stamps = app
        .workspace
        .detail()
        .value()
        .into_iter()
        .flat_map(|detail| detail.comments.iter().map(|comment| comment.created_at));

    issue_stamps
        .chain(comment_stamps)
        .filter_map(|stamp| stamp.next_change(now))
        .min()
}

fn render_zoomed(app: &mut App, frame: &mut Frame, area: Rect) {
    let Viewport(rows) = match app.focus {
        Focus::MyWork => render_panel(app, frame, area, LeftPanel::MyWork, Emphasis::Focused),
        Focus::Recent => render_panel(app, frame, area, LeftPanel::Recent, Emphasis::Focused),
        Focus::SavedViews => {
            render_panel(app, frame, area, LeftPanel::SavedViews, Emphasis::Focused)
        }
        Focus::View => render_view_surface(app, frame, area, Emphasis::Focused),
        Focus::Stub(index) => {
            render_panel(app, frame, area, LeftPanel::Stub(index), Emphasis::Focused)
        }
        Focus::Detail(..) => render_detail_pane(app, frame, area, Emphasis::Focused),
    };

    app.viewport = rows;
}

fn render_overlay(overlay: &mut Overlay, feeds: &FeedStore, spinner: Spinner, frame: &mut Frame) {
    use ratatui::widgets::Clear;

    let frame_area = frame.area();

    match overlay {
        Overlay::Picker(picker) => {
            let area = overlays::picker::area(frame_area);
            frame.render_widget(Clear, area);
            overlays::picker::render(picker, spinner, frame, area);
        }
        Overlay::Confirm(confirm) => {
            let area = overlays::confirm::area(frame_area);
            frame.render_widget(Clear, area);
            overlays::confirm::render(confirm, frame, area);
        }
        Overlay::Menu(menu) => {
            let area = overlays::menu::area(frame_area);
            frame.render_widget(Clear, area);
            overlays::menu::render(menu, frame, area);
        }
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
        Overlay::Search(search) => {
            let feed = feeds.get(&FeedKey::Search(search.query.clone()));
            let results: &[IssueSummary] = feed.map_or(&[], |feed| feed.items());
            let feed_data = overlays::search::SearchFeed {
                results,
                status: feed.map(Feed::status),
                appending: feed.is_some_and(|feed| feed.appending()),
            };

            let area = overlays::search::area(frame_area);
            frame.render_widget(Clear, area);

            overlays::search::render(search, feed_data, spinner, frame, area);
        }
        Overlay::Prefix(prefix) => match &mut prefix.under {
            PrefixUnder::Modal(modal) => render_overlay(modal, feeds, spinner, frame),
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
        Overlay::Find(_) | Overlay::None => {}
    }
}

fn active_view<'a>(views: &'a [View], view_state: &ListState) -> Option<&'a View> {
    let index = view_state
        .selected()
        .unwrap_or(0)
        .min(views.len().saturating_sub(1));

    views.get(index)
}

fn selected_issue<'w>(
    workspace: &'w WorkspaceData,
    views: &[View],
    view_state: &ListState,
    list_state: &ListState,
) -> Option<&'w IssueSummary> {
    let view = active_view(views, view_state)?;

    match &view.kind {
        ViewKind::Issues(_) => list_state
            .selected()
            .and_then(|index| workspace.issues_for(view).get(index)),
        ViewKind::Inbox => None,
    }
}

fn work_preview<'a>(
    workspace: &'a WorkspaceData,
    views: &'a [View],
    view_state: &ListState,
    list_state: &ListState,
) -> surfaces::detail::Preview<'a> {
    use surfaces::detail::Preview;

    match active_view(views, view_state).map(|view| &view.kind) {
        Some(ViewKind::Issues(_)) => {
            Preview::Issue(selected_issue(workspace, views, view_state, list_state))
        }
        Some(ViewKind::Inbox) => Preview::Notification(
            list_state
                .selected()
                .and_then(|index| workspace.inbox.items().get(index)),
        ),
        None => Preview::Issue(None),
    }
}

fn ready_detail<'w>(
    workspace: &'w WorkspaceData,
    views: &[View],
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
                views,
                view_state,
                list_state,
                spinner,
                ..
            } = &mut *app;
            let active = view_state.selected().unwrap_or(0);
            let Some(view) = active_view(views, view_state) else {
                return Viewport((rect.height as usize).saturating_sub(2));
            };

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
                    views,
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
            let spinner = app.spinner;
            surfaces::saved_views::render(
                frame,
                rect,
                &mut app.workspace.saved_views,
                emphasis,
                spinner,
            )
        }
        LeftPanel::Stub(index) => {
            let stub = &mut app.stubs[index];
            surfaces::stub::render(
                frame,
                rect,
                &stub.title,
                &stub.items,
                &mut stub.state,
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
    let spinner = app.spinner;
    let now = app.now;
    let feeds = &app.workspace.feeds;

    match app.workspace.view_open.as_mut() {
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
    let selected = match &app.focus {
        Focus::Detail(detail) if detail.view == DetailView::Comments => {
            app.comment_state.selected()
        }
        _ => None,
    };

    let preview = work_preview(&app.workspace, &app.views, &app.view_state, &app.list_state);

    surfaces::detail::render_pane(
        frame,
        area,
        app.workspace.detail(),
        app.workspace.detail_markdown(),
        app.spinner,
        preview,
        surfaces::detail::ReadingProps {
            now: app.now,
            selected,
            scroll_position: &mut app.scroll_position,
            scroll_state: &mut app.scroll_state,
            emphasis,
        },
    );

    Viewport((area.height as usize).saturating_sub(2))
}

fn render_my_work_right(app: &mut App, frame: &mut Frame, area: Rect) {
    match ready_detail(&app.workspace, &app.views, &app.view_state, &app.list_state) {
        Some(detail) => surfaces::detail::render_reading(
            frame,
            area,
            detail,
            app.workspace.detail_markdown(),
            surfaces::detail::ReadingProps {
                now: app.now,
                selected: None,
                scroll_position: &mut app.scroll_position,
                scroll_state: &mut app.scroll_state,
                emphasis: Emphasis::Blurred,
            },
        ),
        None => {
            let preview =
                work_preview(&app.workspace, &app.views, &app.view_state, &app.list_state);
            surfaces::detail::render_work_preview(frame, area, preview, Emphasis::Blurred);
        }
    }
}

fn render_left(app: &mut App, frame: &mut Frame, area: Rect) {
    let panels = app.panels();
    let expanded = app.focus.left();
    let constraints: Vec<Constraint> = panels
        .iter()
        .map(|&panel| {
            if panel == expanded {
                Constraint::Min(5)
            } else {
                let rows = app.panel_len(&panel.focus()).clamp(1, COLLAPSED_PEEK);
                Constraint::Length(rows as u16 + 2)
            }
        })
        .collect();

    let rects = Layout::vertical(constraints).split(area);

    for (rect, panel) in rects.iter().zip(panels) {
        let focused = panel.focus() == app.focus;
        let Viewport(rows) = render_panel(app, frame, *rect, panel, Emphasis::of_focus(focused));

        if focused {
            app.viewport = rows;
        }
    }
}

fn render_right(app: &mut App, frame: &mut Frame, area: Rect) {
    match app.focus {
        Focus::Stub(index) => {
            let stub = &app.stubs[index];
            let selected = stub
                .state
                .selected()
                .and_then(|i| stub.items.get(i))
                .map(String::as_str)
                .unwrap_or("");
            surfaces::stub::render_placeholder(frame, area, &stub.title, selected);
        }
        Focus::Recent => surfaces::recent::render_preview(frame, area, app.selected_recent()),
        Focus::SavedViews => {
            let spinner = app.spinner;
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
        Focus::View => {
            app.viewport = render_view_surface(app, frame, area, Emphasis::Focused).0;
        }
        Focus::MyWork => render_my_work_right(app, frame, area),
        Focus::Detail(..) => {
            app.viewport = render_detail_pane(app, frame, area, Emphasis::Focused).0;
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

    let workspace = match &app.workspace.session {
        Some(session) => format!("{} · @{} ", session.org_name, session.user.display_name),
        None => "connecting… ".to_string(),
    };

    let left = match &app.status {
        Some(status) => FooterLeft::Status {
            text: status.to_string(),
            is_error: status.is_error(),
        },
        None => FooterLeft::Hint {
            text: footer_hint(app),
        },
    };

    Footer::Normal { left, workspace }
}

fn find_bar_state(app: &App) -> Option<surfaces::footer::FindBar> {
    use surfaces::footer::FindBar;

    match &app.overlay {
        Overlay::Find(find) => Some(FindBar::Typing {
            query: find.query.clone(),
            total: app.focused_matches(&find.query).len(),
        }),
        Overlay::None => {
            let query = app.find_query.as_deref()?;
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
    match &app.overlay {
        Overlay::Menu(_) => return action::MENU.hint_bar(action::MENU_HINTS),
        Overlay::Confirm(_) => return action::CONFIRM.hint_bar(action::CONFIRM_HINTS),
        Overlay::Picker(_) | Overlay::Search(_) => {
            return action::PICKER.hint_bar(action::PICKER_HINTS)
        }
        Overlay::Prefix(prefix) => return format!("{}   esc cancel", prefix.keymap.summary()),
        Overlay::Input(_) => return action::INPUT.hint_bar(action::INPUT_HINTS),
        Overlay::Editor(_) => return action::EDITOR.hint_bar(action::EDITOR_HINTS),
        Overlay::Reactions(_) => return action::REACTIONS.hint_bar(action::REACTIONS_HINTS),
        Overlay::Find(_) | Overlay::None => {}
    }

    let specs = match &app.focus {
        Focus::MyWork => action::MY_WORK_HINTS,
        Focus::Recent => action::RECENT_HINTS,
        Focus::SavedViews => action::SAVED_VIEWS_HINTS,
        Focus::View => action::VIEW_HINTS,
        Focus::Stub(_) => action::STUB_HINTS,
        Focus::Detail(detail) => match detail.view {
            DetailView::Reading => action::DETAIL_HINTS,
            DetailView::Comments => action::COMMENTS_HINTS,
        },
    };
    action::BROWSE.hint_bar(specs)
}
