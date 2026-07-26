use crossterm::event::KeyEvent;

use super::action::ConfirmInput;
use super::app::App;
use super::event::Redraw;
use super::message::Command;
use super::overlay::Overlay;
use crate::api::Timestamp;

mod feed;
mod input;
mod issue;
mod message;
mod nav;

pub use feed::{initial_commands, restore_feeds};
pub use message::apply;

use input::{
    apply_action, apply_confirm, apply_editor, apply_find, apply_input, apply_menu, apply_picker,
    apply_prefix, apply_search, resolve_browse,
};

pub fn handle_key(app: &mut App, key: KeyEvent) -> Option<Command> {
    if super::action::is_quit(&key) {
        app.should_quit = true;
        return None;
    }

    app.status = None;

    match std::mem::take(&mut app.overlay) {
        Overlay::Confirm(confirm) => apply_confirm(app, confirm, ConfirmInput::from_key(key)),
        Overlay::Picker(picker) => apply_picker(app, picker, key),
        Overlay::Menu(menu) => apply_menu(app, menu, key),
        Overlay::Prefix(prefix) => apply_prefix(app, prefix, key),
        Overlay::Input(input) => apply_input(app, input, key),
        Overlay::Editor(editor) => apply_editor(app, editor, key),
        Overlay::Search(search) => apply_search(app, search, key),
        Overlay::Find(find) => apply_find(app, find, key),
        Overlay::None => resolve_browse(app, key).and_then(|action| apply_action(app, action)),
    }
}

pub fn tick(app: &mut App, now: Timestamp) -> Redraw {
    app.now = now;

    let spinner_advanced = app.is_loading();
    if spinner_advanced {
        app.spinner.tick();
    }

    let timestamp_due = app.time_refresh_due.is_some_and(|due| now >= due);

    if spinner_advanced || timestamp_due {
        Redraw::Needed
    } else {
        Redraw::Skipped
    }
}
