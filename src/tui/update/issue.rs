use ratatui::widgets::{ListState, ScrollbarState};

use super::nav::clamp_selection;
use crate::api::{Reaction, ReactionTarget, StateOption, User};
use crate::tui::app::{App, FocusedIssue};
use crate::tui::cache::{RefreshPolicy, Remote};
use crate::tui::focus::{DetailView, Focus, Reveal};
use crate::tui::message::Command;
use crate::tui::overlay::{
    Compose, Confirm, Editor, Overlay, Picker, PickerItem, PickerKind, Reactions,
};
use crate::tui::status::Status;

const STATES_REFRESH: RefreshPolicy = RefreshPolicy::new(60 * 60, 24 * 60 * 60);
const MEMBERS_REFRESH: RefreshPolicy = RefreshPolicy::new(60 * 60, 24 * 60 * 60);

pub(super) fn enter_comments(app: &mut App) -> Option<Command> {
    if !app.has_comments() {
        app.status = Some(Status::NoComments);
        return None;
    }

    app.focus = Focus::Detail(app.focus.left(), DetailView::Comments);
    app.comment_state.select(Some(0));

    None
}

pub(super) fn open_reply_editor(app: &mut App) -> Option<Command> {
    let detail = app.workspace.detail().value()?;
    let team_id = detail.team_id.clone();
    let selected = app.comment_state.selected()?;
    let parent_id = detail
        .threaded_comments()
        .get(selected)?
        .comment
        .reply_parent();

    open_editor(app, Compose::Reply { parent_id }, team_id, None)
}

pub(super) fn open_edit_editor(app: &mut App) -> Option<Command> {
    let selected = app.comment_state.selected()?;

    let picked = app.workspace.detail().value().and_then(|detail| {
        detail.threaded_comments().get(selected).map(|threaded| {
            (
                detail.team_id.clone(),
                threaded.comment.id.clone(),
                threaded.comment.body.clone(),
                threaded.comment.is_mine,
            )
        })
    });

    let (team_id, comment_id, body, is_mine) = picked?;

    if !is_mine {
        app.status = Some(Status::NotYourComment);
        return None;
    }

    open_editor(app, Compose::Edit { comment_id }, team_id, Some(&body))
}

pub(super) fn open_reactions(app: &mut App) -> Option<Command> {
    let detail = app.workspace.detail().value()?;

    let (target, reactions) = match app.focus {
        Focus::Detail(_, DetailView::Comments) => {
            let selected = app.comment_state.selected()?;
            let comment = detail.threaded_comments().get(selected)?.comment;
            (
                ReactionTarget::Comment(comment.id.clone()),
                comment.reactions.clone(),
            )
        }
        Focus::Detail(_, DetailView::Reading) => (
            ReactionTarget::Issue(detail.id.clone()),
            detail.reactions.clone(),
        ),
        Focus::MyWork | Focus::Recent | Focus::SavedViews | Focus::View | Focus::Stub(_) => {
            return None
        }
    };

    app.overlay = Overlay::Reactions(Reactions::new(target, &reactions));
    None
}

pub(super) fn toggle_reaction(
    app: &mut App,
    target: ReactionTarget,
    emoji: &str,
) -> Option<Command> {
    let detail = app.workspace.detail().value()?;
    let issue_id = detail.id.clone();

    let reactions: &[Reaction] = match &target {
        ReactionTarget::Issue(_) => &detail.reactions,
        ReactionTarget::Comment(comment_id) => detail
            .comments
            .iter()
            .find(|comment| &comment.id == comment_id)
            .map_or(&[], |comment| comment.reactions.as_slice()),
    };

    let mine = reactions
        .iter()
        .find(|reaction| reaction.mine && reaction.emoji == emoji);

    let command = match mine {
        Some(reaction) => Command::DeleteReaction {
            issue_id,
            reaction_id: reaction.id.clone(),
        },
        None => Command::CreateReaction {
            issue_id,
            target,
            emoji: emoji.to_string(),
        },
    };

    Some(command)
}

pub(super) fn open_delete_comment(app: &mut App) -> Option<Command> {
    let selected = app.comment_state.selected()?;

    let picked = app.workspace.detail().value().and_then(|detail| {
        detail.threaded_comments().get(selected).map(|threaded| {
            (
                detail.id.clone(),
                threaded.comment.id.clone(),
                threaded.comment.is_mine,
            )
        })
    });

    let (issue_id, comment_id, is_mine) = picked?;

    if !is_mine {
        app.status = Some(Status::NotYourComment);
        return None;
    }

    app.overlay = Overlay::Confirm(Confirm {
        message: "Delete this comment?".into(),
        command: Command::DeleteComment {
            issue_id,
            comment_id,
        },
    });

    None
}

pub(super) fn open_issue(app: &mut App, id: String) -> Option<Command> {
    app.search_return = None;
    app.focus = Focus::Detail(app.focus.left(), DetailView::Reading);
    app.scroll_position = 0;
    app.scroll_state = ScrollbarState::default();

    let already_open = app
        .workspace
        .detail()
        .value()
        .is_some_and(|detail| detail.id == id);

    if already_open {
        return None;
    }

    app.workspace.bust_detail();
    app.workspace.begin_detail();

    Some(Command::LoadDetail {
        id,
        reveal: Reveal::Top,
    })
}

pub(super) fn clear_recent(app: &mut App) {
    match app.focus {
        Focus::Recent if !app.workspace.recently_viewed.is_empty() => {
            app.overlay = Overlay::Confirm(Confirm {
                message: "Clear recently viewed?".into(),
                command: Command::ClearRecent,
            });
        }
        Focus::MyWork
        | Focus::Recent
        | Focus::SavedViews
        | Focus::View
        | Focus::Stub(_)
        | Focus::Detail(..) => {}
    }
}

pub(super) fn open_status_picker(app: &mut App) -> Option<Command> {
    let target = require(app, app.action_target(), Status::NeedOpenIssue)?;
    open_picker(app, PickerKind::Status, target)
}

pub(super) fn open_assign_picker(app: &mut App) -> Option<Command> {
    let target = require(app, app.action_target(), Status::NeedOpenIssue)?;
    open_picker(app, PickerKind::Assign, target)
}

pub(super) fn open_comment_input(app: &mut App) -> Option<Command> {
    let target = require(app, app.action_target(), Status::NeedOpenIssue)?;
    open_editor(app, Compose::Comment, target.team_id, None)
}

pub(super) fn status_items(states: &[StateOption]) -> Vec<PickerItem> {
    states.iter().cloned().map(PickerItem::from).collect()
}

pub(super) fn assign_items(members: &[User]) -> Vec<PickerItem> {
    let mut items = vec![PickerItem::unassign()];
    items.extend(members.iter().cloned().map(PickerItem::from));
    items
}

pub(super) fn fill_picker(picker: &mut Picker, items: Vec<PickerItem>) {
    let was_loading = picker.loading;
    picker.items = items;
    picker.loading = false;
    if was_loading {
        picker.state.select(Some(0));
    } else {
        clamp_selection(&mut picker.state, picker.items.len());
    }
}

pub(super) fn stop_picker(overlay: &mut Overlay, kind: PickerKind) {
    if let Overlay::Picker(picker) = overlay {
        if picker.kind == kind {
            picker.loading = false;
        }
    }
}

pub(super) fn access_states(app: &mut App, team_id: &str) -> Option<Command> {
    app.workspace
        .states
        .get_or_default(&team_id.to_string())
        .begin_access(app.now, &STATES_REFRESH)
        .then(|| Command::LoadStates {
            team_id: team_id.to_string(),
        })
}

pub(super) fn access_members(app: &mut App, team_id: &str) -> Option<Command> {
    app.workspace
        .members
        .get_or_default(&team_id.to_string())
        .begin_access(app.now, &MEMBERS_REFRESH)
        .then(|| Command::LoadMembers {
            team_id: team_id.to_string(),
        })
}

pub(super) fn open_picker(
    app: &mut App,
    kind: PickerKind,
    target: FocusedIssue,
) -> Option<Command> {
    let team_id = target.team_id;
    let (items, command) = match kind {
        PickerKind::Status => {
            let command = access_states(app, &team_id);
            let items = app
                .workspace
                .states
                .get(&team_id)
                .and_then(Remote::value)
                .map_or_else(Vec::new, |states| status_items(states));
            (items, command)
        }
        PickerKind::Assign => {
            let command = access_members(app, &team_id);
            let items = app
                .workspace
                .members
                .get(&team_id)
                .and_then(Remote::value)
                .map_or_else(Vec::new, |members| assign_items(members));
            (items, command)
        }
    };

    let loading = items.is_empty();
    app.overlay = Overlay::Picker(Picker {
        kind,
        target_issue: target.id,
        target_label: target.identifier,
        items,
        state: ListState::default().with_selected(Some(0)),
        loading,
    });
    command
}

pub(super) fn open_editor(
    app: &mut App,
    compose: Compose,
    team_id: String,
    seed: Option<&str>,
) -> Option<Command> {
    let mut editor = match seed {
        Some(body) => Editor::seeded(compose, body),
        None => Editor::new(compose),
    };
    editor.members = app
        .workspace
        .members
        .get(&team_id)
        .and_then(Remote::value)
        .cloned()
        .unwrap_or_default();
    app.overlay = Overlay::Editor(editor);
    access_members(app, &team_id)
}

pub(super) fn open_in_browser(app: &mut App) -> Option<Command> {
    let target = require(app, app.open_target(), Status::NeedHighlightedIssue)?;
    Some(Command::OpenUrl(target.url))
}

pub(super) fn yank_url(app: &mut App) -> Option<Command> {
    let target = require(app, app.open_target(), Status::NeedHighlightedIssue)?;
    app.status = Some(Status::CopiedUrl);
    Some(Command::CopyToClipboard(target.url))
}

pub(super) fn require<T>(app: &mut App, target: Option<T>, status: Status) -> Option<T> {
    match target {
        some @ Some(_) => some,
        None => {
            app.status = Some(status);
            None
        }
    }
}

pub(super) fn newest_comment_index(detail: &crate::api::IssueDetail) -> Option<usize> {
    detail
        .threaded_comments()
        .iter()
        .enumerate()
        .max_by_key(|(_, threaded)| threaded.comment.created_at)
        .map(|(index, _)| index)
}
