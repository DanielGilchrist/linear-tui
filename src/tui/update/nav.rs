use ratatui::widgets::ListState;

use super::feed::{
    access_active, access_feed, load_more, load_more_for_focus, prefetch_selected_view,
};
use super::issue::open_issue;
use crate::tui::app::{App, Zoom, SCROLL_STEP};
use crate::tui::feed::FeedKey;
use crate::tui::focus::{DetailView, Direction, Edge, Focus, LeftPanel, Nav};
use crate::tui::message::Command;
use crate::tui::overlay::Overlay;
use crate::tui::saved_views::ViewSurface;
use crate::tui::view::ViewKind;

pub(super) fn scrolled(position: usize, step: usize, direction: Direction) -> usize {
    match direction {
        Direction::Next => position.saturating_add(step),
        Direction::Prev => position.saturating_sub(step),
    }
}

pub(super) fn select_edge(state: &mut ListState, len: usize, edge: Edge) {
    if len == 0 {
        return;
    }

    state.select(Some(match edge {
        Edge::Bottom => len - 1,
        Edge::Top => 0,
    }));
}

pub(super) fn navigate_list(state: &mut ListState, len: usize, direction: Direction) {
    if len == 0 {
        return;
    }
    let index = match state.selected() {
        Some(current) => direction.wrap(current, len),
        None => 0,
    };
    state.select(Some(index));
}

pub(super) fn clamp_selection(state: &mut ListState, len: usize) {
    if len == 0 {
        state.select(Some(0));
    } else if state.selected().unwrap_or(0) >= len {
        state.select(Some(len - 1));
    }
}

pub(super) fn cycle_panel(app: &mut App, direction: Direction) -> Option<Command> {
    let leaving_view = app.focus == Focus::View;
    let panels = app.panels();
    let count = panels.len();
    let current = panels
        .iter()
        .position(|&p| p == app.focus.left())
        .unwrap_or(0);

    let next = direction.wrap(current, count + 1);

    app.focus = if next == count {
        Focus::Detail(app.focus.left(), DetailView::Reading)
    } else {
        panels[next].focus()
    };

    if leaving_view {
        app.close_view_surface();
    }

    prefetch_selected_view(app)
}

pub(super) fn jump_panel(app: &mut App, index: usize) -> Option<Command> {
    let leaving_view = app.focus == Focus::View;

    if index < app.panel_count() {
        app.focus = app.panel_at(index).focus();
    }

    if leaving_view && app.focus != Focus::View {
        app.close_view_surface();
    }

    prefetch_selected_view(app)
}

pub(super) fn ascend(app: &mut App) -> Option<Command> {
    if app.find_query.take().is_some() {
        return None;
    }

    if app.zoom == Zoom::Full {
        app.zoom = Zoom::Normal;
        return None;
    }

    match app.focus {
        Focus::Detail(panel, DetailView::Comments) => {
            app.focus = Focus::Detail(panel, DetailView::Reading);
        }
        Focus::Detail(LeftPanel::SavedViews, DetailView::Reading) => {
            match app.workspace.view_open {
                Some(_) => app.focus = Focus::View,
                None => leave_detail(app),
            }
        }
        Focus::Detail(_, DetailView::Reading) => leave_detail(app),
        Focus::View => {
            app.close_view_surface();
        }
        Focus::MyWork | Focus::Recent | Focus::SavedViews | Focus::Stub(_) => {
            app.focus = Focus::MyWork
        }
    }

    None
}

pub(super) fn leave_detail(app: &mut App) {
    match app.search_return.take() {
        Some(search) => app.overlay = Overlay::Search(search),
        None => app.focus = Focus::MyWork,
    }
}

pub(super) fn descend(app: &mut App) -> Option<Command> {
    let id = match app.focus {
        Focus::MyWork => match app.active_view().kind {
            ViewKind::Issues(_) => app.selected_issue().map(|i| i.id.clone()),
            ViewKind::Inbox => app.selected_notification().and_then(|n| n.issue_id.clone()),
        },
        Focus::Recent => app.selected_recent().map(|i| i.id.clone()),
        Focus::SavedViews => return open_view(app),
        Focus::View => app.view_selected_issue().map(|issue| issue.id.clone()),
        Focus::Stub(_) | Focus::Detail(..) => None,
    }?;

    open_issue(app, id)
}

pub(super) fn open_view(app: &mut App) -> Option<Command> {
    let view = app.workspace.saved_views.selected_view()?.clone();

    let command = access_feed(app, FeedKey::View(view.id.clone()));
    app.open_view_surface(ViewSurface::new(view));

    command
}

pub(super) fn cycle_view_group(app: &mut App) {
    let keep = app.view_selected_issue().map(|issue| issue.id.clone());
    if let Some(view) = &mut app.workspace.view_open {
        view.display.cycle_group();
    }
    reselect_view(app, keep);
}

pub(super) fn cycle_view_sort(app: &mut App) {
    let keep = app.view_selected_issue().map(|issue| issue.id.clone());
    if let Some(view) = &mut app.workspace.view_open {
        view.display.cycle_sort();
    }
    reselect_view(app, keep);
}

pub(super) fn reselect_view(app: &mut App, keep: Option<String>) {
    let pos = keep
        .and_then(|id| {
            let issues = app.view_issues()?;
            app.view_ordered()
                .iter()
                .position(|&index| issues[index].id == id)
        })
        .unwrap_or(0);

    if let Some(view) = &mut app.workspace.view_open {
        view.layout = ListState::default();
        view.state.select(Some(pos));
    }
}

pub(super) fn history_step(app: &mut App, direction: Direction) -> Option<Command> {
    if app.workspace.recently_viewed.is_empty() {
        return None;
    }

    let target = match (app.open_recent_pos(), direction) {
        (Some(pos), Direction::Next) => pos.checked_sub(1),
        (Some(pos), Direction::Prev) => Some(pos + 1),
        (None, _) => Some(0),
    };

    let issue = target.and_then(|index| app.workspace.recently_viewed.get(index))?;
    let id = issue.id.clone();
    open_issue(app, id)
}

pub(super) fn move_selection(app: &mut App, direction: Direction) -> Option<Command> {
    match app.nav() {
        Nav::List { state, len, .. } => navigate_list(state, len, direction),
        Nav::Scroll { position, .. } => *position = scrolled(*position, SCROLL_STEP, direction),
    }

    prefetch_selected_view(app).or_else(|| load_more_for_focus(app))
}

pub(super) fn scroll_half(app: &mut App, direction: Direction) -> Option<Command> {
    match app.nav() {
        Nav::List {
            state,
            len,
            viewport,
        } => {
            if len == 0 {
                return None;
            }

            let step = (viewport / 2).max(1);
            let current = state.selected().unwrap_or(0);
            let next = match direction {
                Direction::Next => (current + step).min(len - 1),
                Direction::Prev => current.saturating_sub(step),
            };

            state.select(Some(next));
        }
        Nav::Scroll { position, viewport } => {
            *position = scrolled(*position, (viewport / 2).max(1), direction)
        }
    }

    load_more_for_focus(app)
}

pub(super) fn jump_edge(app: &mut App, edge: Edge) -> Option<Command> {
    match &mut app.overlay {
        Overlay::Menu(menu) => {
            menu.jump_edge(edge);
            return None;
        }
        Overlay::Picker(picker) => {
            select_edge(&mut picker.state, picker.items.len(), edge);
            return None;
        }
        Overlay::Search(_) => {
            let (key, len) = match &app.overlay {
                Overlay::Search(search) => (
                    FeedKey::Search(search.query.clone()),
                    app.search_results(&search.query).len(),
                ),
                _ => return None,
            };
            let selected = if let Overlay::Search(search) = &mut app.overlay {
                select_edge(&mut search.state, len, edge);
                search.state.selected()
            } else {
                None
            };
            return load_more(app, &key, selected, len);
        }
        _ => {}
    }

    match app.nav() {
        Nav::List { state, len, .. } => select_edge(state, len, edge),
        Nav::Scroll { position, .. } => {
            *position = match edge {
                Edge::Bottom => usize::MAX,
                Edge::Top => 0,
            }
        }
    }

    load_more_for_focus(app)
}

pub(super) fn cycle_view(app: &mut App, direction: Direction) -> Option<Command> {
    match app.focus {
        Focus::MyWork => {
            let next = direction.wrap(app.active_view_index(), app.views.len());
            select_view(app, next)
        }
        Focus::Recent | Focus::SavedViews | Focus::View | Focus::Stub(_) | Focus::Detail(..) => {
            None
        }
    }
}

pub(super) fn select_view(app: &mut App, index: usize) -> Option<Command> {
    app.focus = Focus::MyWork;
    app.view_state.select(Some(index));
    app.list_state.select(Some(0));
    app.workspace.detail.bust();
    app.find_query = None;

    access_active(app)
}
