use crossterm::event::KeyEvent;

use super::action::ConfirmInput;
use super::app::App;
use super::event::Redraw;
use super::focus::Reveal;
use super::message::{ApiCommand, Commands, Effect, Effects};
use super::overlay::{Overlay, Workspaces};
use crate::api::Timestamp;

mod feed;
mod input;
mod issue;
mod message;
mod nav;

pub use feed::{initial_commands, restore_feeds};
pub use message::apply;

use input::{
    apply_action, apply_confirm, apply_editor, apply_find, apply_input, apply_labels, apply_menu,
    apply_outcome, apply_picker, apply_prefix, apply_reactions, apply_search, apply_workspaces,
    resolve_browse,
};

pub fn open_workspaces(app: &mut App) {
    app.set_overlay(Overlay::Workspaces(Workspaces::new(
        app.session.accounts(),
        app.session.active_workspace(),
    )));
}

pub fn reconnect(app: &mut App) -> Effects {
    app.workspace.cancel_in_flight();
    app.cancel_overlay_in_flight();

    let mut effects = initial_commands(app);
    effects.extend(feed::revalidate_focus(app));

    if let Some(target) = app.focus().detail().map(|detail| detail.issue.clone()) {
        app.workspace.begin_detail();

        effects.push(Effect::Api(ApiCommand::LoadDetail {
            target,
            reveal: Reveal::Keep,
        }));
    }

    effects
}

pub fn handle_key(app: &mut App, key: KeyEvent) -> Commands {
    if super::action::is_quit(&key) {
        app.should_quit = true;
        return Commands::default();
    }

    let outcome = match app.take_overlay() {
        Overlay::Confirm(confirm) => apply_confirm(confirm, ConfirmInput::from_key(key)),
        Overlay::Picker(picker) => apply_picker(picker, key),
        Overlay::Menu(menu) => apply_menu(menu, key),
        Overlay::Prefix(prefix) => apply_prefix(prefix, key),
        Overlay::Input(input) => apply_input(app, input, key),
        Overlay::Editor(editor) => apply_editor(editor, key),
        Overlay::Search(search) => apply_search(app, search, key),
        Overlay::Find(find) => apply_find(app, find, key),
        Overlay::Reactions(reactions) => apply_reactions(app, reactions, key),
        Overlay::Labels(labels) => apply_labels(labels, key),
        Overlay::Workspaces(workspaces) => apply_workspaces(app, workspaces, key),
        Overlay::None => {
            return resolve_browse(app, key)
                .map(|action| apply_action(app, action))
                .unwrap_or_default()
                .into();
        }
    };

    apply_outcome(app, outcome)
}

pub fn tick(app: &mut App, now: Timestamp) -> Redraw {
    let timestamp_due = earliest_time_refresh(app).is_some_and(|due| now >= due);

    app.now = now;

    let auth_changed = app.expire_stuck_refresh();

    let spinner_advanced = app.is_loading();
    if spinner_advanced {
        app.ui.spinner.tick();
    }

    if spinner_advanced || timestamp_due || auth_changed {
        Redraw::Needed
    } else {
        Redraw::Skipped
    }
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
