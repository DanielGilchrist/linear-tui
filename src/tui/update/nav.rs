use std::num::NonZeroUsize;

use ratatui::widgets::ListState;

use super::feed::{
    access_active, access_feed, access_focused_panel, load_more, load_more_for_focus,
    prefetch_selected_view,
};
use super::issue::open_issue;
use crate::api::{IssueId, IssueRef, IssueSummary};
use crate::tui::app::{App, Zoom};
use crate::tui::feed::FeedKey;
use crate::tui::focus::{
    select_edge, DetailView, Direction, Edge, Focus, LeftPanel, Origin, PANELS,
};
use crate::tui::message::Effects;
use crate::tui::overlay::Overlay;
use crate::tui::saved_views::ViewSurface;
use crate::tui::view::ViewKind;

pub(super) fn clamp_selection(state: &mut ListState, len: usize) {
    let selected = state.selected().unwrap_or(0);

    state.select(Some(selected.min(len.saturating_sub(1))));
}

pub(super) fn cycle_panel(app: &mut App, direction: Direction) -> Effects {
    let leaving_view = app.focus().is_view();
    let count = PANELS.len();
    let current = PANELS
        .iter()
        .position(|&p| p == app.focus().left())
        .unwrap_or(0);

    let slots = NonZeroUsize::MIN.saturating_add(count);
    let mut next = direction.wrap(current, slots);

    let opening = if next == count {
        let open = selected_open(app);

        if open.is_none() {
            next = direction.wrap(next, slots);
        }
        open
    } else {
        None
    };

    let command = match opening {
        Some((target, summary)) => {
            let origin = app.take_origin();
            open_issue(app, target, summary, origin)
        }
        None => {
            if let Some(panel) = PANELS.get(next) {
                app.focus_panel(*panel);
            }
            Effects::default()
        }
    };

    if leaving_view {
        app.close_view_surface();
    }

    command.or_else(|| access_focused_panel(app))
}

fn selected_open(app: &App) -> Option<(IssueRef, Option<IssueSummary>)> {
    match app.focus() {
        Focus::MyWork => match app.active_view().kind {
            ViewKind::Issues(_) => app.selected_issue().map(with_summary),
            ViewKind::Inbox => app
                .selected_notification()
                .and_then(|notification| notification.issue_id.clone())
                .map(|id| (id.into(), None)),
        },
        Focus::Recent => app.selected_recent().map(with_summary),
        Focus::View(_) => app.view_selected_issue().map(with_summary),
        Focus::SavedViews | Focus::Teams | Focus::Detail(..) => None,
    }
}

fn with_summary(issue: &IssueSummary) -> (IssueRef, Option<IssueSummary>) {
    (issue.id.clone().into(), Some(issue.clone()))
}

pub(super) fn jump_panel(app: &mut App, index: usize) -> Effects {
    let leaving_view = app.focus().is_view();

    match app.panel_at(index) {
        Some(panel) => app.focus_panel(panel),
        None => return Effects::default(),
    }

    if leaving_view && !app.focus().is_view() {
        app.close_view_surface();
    }

    access_focused_panel(app)
}

pub(super) fn ascend(app: &mut App) -> Effects {
    if app.ui.find_query.take().is_some() {
        return Effects::default();
    }

    if app.ui.zoom == Zoom::Full {
        app.ui.zoom = Zoom::Normal;
        return Effects::default();
    }

    match app.focus() {
        Focus::Detail(detail) if detail.view.is_comments() => {
            app.set_detail_view(DetailView::reading());
        }
        Focus::Detail(_) => leave_detail(app),
        Focus::View(_) => {
            app.close_view_surface();
        }
        Focus::MyWork | Focus::Recent | Focus::SavedViews | Focus::Teams => app.focus_my_work(),
    }

    Effects::default()
}

pub(super) fn leave_detail(app: &mut App) {
    match app.take_origin() {
        Origin::Panel(panel) => app.focus_panel(panel),
        Origin::View(surface) => app.open_view_surface(*surface),
        Origin::Search(search) => {
            app.focus_panel(LeftPanel::MyWork);
            app.set_overlay(Overlay::Search(*search));
        }
    }
}

pub(super) fn descend(app: &mut App) -> Effects {
    match app.focus().clone() {
        Focus::SavedViews => open_view(app),
        Focus::MyWork | Focus::Recent | Focus::View(_) => {
            let Some((target, summary)) = selected_open(app) else {
                return Effects::default();
            };
            let origin = app.take_origin();
            open_issue(app, target, summary, origin)
        }
        Focus::Teams | Focus::Detail(..) => Effects::default(),
    }
}

pub(super) fn open_view(app: &mut App) -> Effects {
    let Some(view) = app.workspace.saved_views.selected_view().cloned() else {
        return Effects::default();
    };

    let command = access_feed(app, FeedKey::View(view.id.clone()));
    app.open_view_surface(ViewSurface::new(view));

    command
}

pub(super) fn cycle_view_group(app: &mut App) {
    let keep = app.view_selected_issue().map(|issue| issue.id.clone());
    if let Some(view) = app.view_mut() {
        view.display.cycle_group();
    }
    reselect_view(app, keep);
}

pub(super) fn cycle_view_sort(app: &mut App) {
    let keep = app.view_selected_issue().map(|issue| issue.id.clone());
    if let Some(view) = app.view_mut() {
        view.display.cycle_sort();
    }
    reselect_view(app, keep);
}

pub(super) fn reselect_view(app: &mut App, keep: Option<IssueId>) {
    let pos = keep
        .and_then(|id| {
            let issues = app.view_issues()?;
            app.view_ordered()
                .iter()
                .position(|&index| issues[index].id == id)
        })
        .unwrap_or(0);

    if let Some(view) = app.view_mut() {
        view.layout = ListState::default();
        view.state.select(Some(pos));
    }
}

pub(super) fn history_step(app: &mut App, direction: Direction) -> Effects {
    if app.workspace.recently_viewed.is_empty() {
        return Effects::default();
    }

    let target = match (app.open_recent_pos(), direction) {
        (Some(pos), Direction::Next) => pos.checked_sub(1),
        (Some(pos), Direction::Prev) => Some(pos + 1),
        (None, _) => Some(0),
    };

    let Some(issue) = target
        .and_then(|index| app.workspace.recently_viewed.get(index))
        .cloned()
    else {
        return Effects::default();
    };

    let origin = app.take_origin();

    open_issue(app, issue.id.clone().into(), Some(issue), origin)
}

pub(super) fn move_selection(app: &mut App, direction: Direction) -> Effects {
    app.step_selection(direction);
    prefetch_selected_view(app).or_else(|| load_more_for_focus(app))
}

pub(super) fn scroll_half(app: &mut App, direction: Direction) -> Effects {
    app.scroll_half_page(direction);
    load_more_for_focus(app)
}

pub(super) fn jump_edge(app: &mut App, edge: Edge) -> Effects {
    let mut overlay = app.take_overlay();

    let commands = match &mut overlay {
        Overlay::Menu(menu) => {
            menu.jump_edge(edge);
            Effects::default()
        }
        Overlay::Picker(picker) => {
            select_edge(&mut picker.state, picker.items.len(), edge);
            Effects::default()
        }
        Overlay::Search(search) => {
            let key = FeedKey::Search(search.query.clone());
            let len = app.search_results(&search.query).len();
            select_edge(&mut search.state, len, edge);
            let selected = search.state.selected();

            load_more(app, &key, selected, len)
        }
        Overlay::None
        | Overlay::Confirm(_)
        | Overlay::Prefix(_)
        | Overlay::Input(_)
        | Overlay::Editor(_)
        | Overlay::Find(_)
        | Overlay::Reactions(_)
        | Overlay::Workspaces(_)
        | Overlay::Labels(_) => {
            app.jump_to_edge(edge);
            load_more_for_focus(app)
        }
    };

    app.set_overlay(overlay);

    commands
}

pub(super) fn cycle_view(app: &mut App, direction: Direction) -> Effects {
    match app.focus().clone() {
        Focus::MyWork => {
            let next = direction.wrap(app.active_view_index(), app.ui.views.len());
            select_view(app, next)
        }
        Focus::Recent | Focus::SavedViews | Focus::View(_) | Focus::Teams | Focus::Detail(..) => {
            Effects::default()
        }
    }
}

pub(super) fn select_view(app: &mut App, index: usize) -> Effects {
    app.focus_my_work();
    app.ui.view_state.select(Some(index));
    app.ui.list_state.select(Some(0));
    app.workspace.bust_detail();
    app.ui.find_query = None;

    access_active(app)
}
