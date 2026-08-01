use ratatui::widgets::ListState;

use super::input::Report;
use super::nav::clamp_selection;
use crate::api::{
    IssueId, IssueRef, IssueSummary, Label, Priority, Reaction, ReactionTarget, StateOption,
    TeamId, User,
};
use crate::tui::app::{App, FocusedIssue};
use crate::tui::cache::{RefreshPolicy, Remote};
use crate::tui::focus::{DetailFocus, DetailView, Focus, Origin, Reveal};
use crate::tui::message::{ApiCommand, Effect, Effects, PlatformCommand, StoreCommand};
use crate::tui::overlay::{
    AssignOptions, Compose, Confirm, Editor, Labels, Overlay, Picker, PickerItem, PickerKind,
    Reactions,
};
use crate::tui::status::Status;

const STATES_REFRESH: RefreshPolicy = RefreshPolicy::new(60 * 60, 24 * 60 * 60);
const MEMBERS_REFRESH: RefreshPolicy = RefreshPolicy::new(60 * 60, 24 * 60 * 60);

pub(super) fn enter_comments(app: &mut App) -> Report {
    if !app.has_comments() {
        return Report::status(Status::NoComments);
    }

    let len = app.open_detail().map_or(0, |detail| detail.thread_len());
    if let Some(view) = DetailView::comments(len) {
        app.set_detail_view(view);
    }

    Effects::default().into()
}

pub(super) fn open_reply_editor(app: &mut App) -> Effects {
    let Some(detail) = app.open_detail() else {
        return Effects::default();
    };
    let issue_id = detail.id.clone();
    let team_id = detail.team_id.clone();
    let Some(selected) = app.comment_cursor() else {
        return Effects::default();
    };
    let Some(threaded) = detail
        .threaded_comments()
        .get(selected)
        .map(|t| t.comment.reply_parent())
    else {
        return Effects::default();
    };
    let parent_id = threaded;

    open_editor(app, issue_id, Compose::Reply { parent_id }, team_id, None)
}

pub(super) fn open_edit_editor(app: &mut App) -> Report {
    let Some(selected) = app.comment_cursor() else {
        return Effects::default().into();
    };

    let picked = app.open_detail().and_then(|detail| {
        detail.threaded_comments().get(selected).map(|threaded| {
            (
                detail.id.clone(),
                detail.team_id.clone(),
                threaded.comment.id.clone(),
                threaded.comment.body.clone(),
                threaded.comment.is_mine,
            )
        })
    });

    let Some((issue_id, team_id, comment_id, body, is_mine)) = picked else {
        return Effects::default().into();
    };

    if !is_mine {
        return Report::status(Status::NotYourComment);
    }

    open_editor(
        app,
        issue_id,
        Compose::Edit { comment_id },
        team_id,
        Some(&body),
    )
    .into()
}

pub(super) fn open_reactions(app: &mut App) -> Effects {
    let Some(detail) = app.open_detail() else {
        return Effects::default();
    };
    let detail_id = detail.id.clone();
    let Some(view) = app.focus().detail().map(|focus| focus.view) else {
        return Effects::default();
    };

    let selection = match view {
        DetailView::Comments { .. } => {
            let Some(selected) = app.comment_cursor() else {
                return Effects::default();
            };
            detail.threaded_comments().get(selected).map(|threaded| {
                (
                    ReactionTarget::Comment(threaded.comment.id.clone()),
                    threaded.comment.reactions.clone(),
                )
            })
        }
        DetailView::Reading { .. } => Some((
            ReactionTarget::Issue(detail.id.clone()),
            detail.reactions.clone(),
        )),
    };

    let Some((target, reactions)) = selection else {
        return Effects::default();
    };

    app.set_overlay(Overlay::Reactions(Reactions::new(
        detail_id, target, &reactions,
    )));

    Effects::default()
}

pub(super) fn toggle_reaction(
    app: &App,
    issue_id: &IssueId,
    target: ReactionTarget,
    emoji: &str,
) -> Report {
    let Some(detail) = app.open_detail().filter(|detail| &detail.id == issue_id) else {
        return Report::status(Status::NeedHighlightedIssue);
    };
    let issue_id = issue_id.clone();

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
        Some(reaction) => Effect::Api(ApiCommand::DeleteReaction {
            issue_id,
            reaction_id: reaction.id.clone(),
        }),
        None => Effect::Api(ApiCommand::CreateReaction {
            issue_id,
            target,
            emoji: emoji.to_string(),
        }),
    };

    Effects::one(command).into()
}

pub(super) fn open_delete_comment(app: &mut App) -> Report {
    let Some(selected) = app.comment_cursor() else {
        return Effects::default().into();
    };

    let picked = app.open_detail().and_then(|detail| {
        detail.threaded_comments().get(selected).map(|threaded| {
            (
                detail.id.clone(),
                threaded.comment.id.clone(),
                threaded.comment.is_mine,
            )
        })
    });

    let Some((issue_id, comment_id, is_mine)) = picked else {
        return Effects::default().into();
    };

    if !is_mine {
        return Report::status(Status::NotYourComment);
    }

    app.set_overlay(Overlay::Confirm(Confirm {
        message: "Delete this comment?".into(),
        command: Effect::Api(ApiCommand::DeleteComment {
            issue_id,
            comment_id,
        }),
    }));

    Effects::default().into()
}

pub(super) fn open_issue(
    app: &mut App,
    target: IssueRef,
    summary: Option<IssueSummary>,
    origin: Origin,
) -> Effects {
    app.open_detail_focus(DetailFocus {
        issue: target.clone(),
        origin,
        view: DetailView::reading(),
        summary: summary.map(Box::new),
    });

    let already_loaded = app
        .workspace
        .detail()
        .value()
        .is_some_and(|detail| target.matches_detail(detail));

    if already_loaded {
        return Effects::default();
    }

    app.workspace.bust_detail();
    app.workspace.begin_detail();

    Effects::one(Effect::Api(ApiCommand::LoadDetail {
        target,
        reveal: Reveal::Top,
    }))
}

pub(super) fn clear_recent(app: &mut App) {
    match app.focus() {
        Focus::Recent if !app.workspace.recently_viewed.is_empty() => {
            app.set_overlay(Overlay::Confirm(Confirm {
                message: "Clear recently viewed?".into(),
                command: Effect::Store(StoreCommand::ClearRecent),
            }));
        }
        Focus::MyWork
        | Focus::Recent
        | Focus::SavedViews
        | Focus::View(_)
        | Focus::Teams
        | Focus::Detail(..) => {}
    }
}

pub(super) fn open_status_picker(app: &mut App) -> Report {
    let target = match require(app.action_target(), Status::NeedOpenIssue) {
        Ok(target) => target,
        Err(status) => return Report::status(status),
    };

    open_picker(app, PickerKind::Status, target).into()
}

pub(super) fn open_assign_picker(app: &mut App) -> Report {
    let target = match require(app.action_target(), Status::NeedOpenIssue) {
        Ok(target) => target,
        Err(status) => return Report::status(status),
    };

    open_picker(app, PickerKind::Assign(AssignOptions::Suggested), target).into()
}

pub(super) fn open_priority_picker(app: &mut App) -> Report {
    let target = match require(app.action_target(), Status::NeedOpenIssue) {
        Ok(target) => target,
        Err(status) => return Report::status(status),
    };

    open_picker(app, PickerKind::Priority, target).into()
}

pub(super) fn open_labels(app: &mut App) -> Report {
    let target = match require(app.action_target(), Status::NeedOpenIssue) {
        Ok(target) => target,
        Err(status) => return Report::status(status),
    };
    let current = current_labels(app);

    app.set_overlay(Overlay::Labels(Labels::new(
        target.id,
        target.identifier,
        current,
    )));

    Effects::one(Effect::Api(ApiCommand::SearchLabels {
        query: String::new(),
    }))
    .into()
}

fn current_labels(app: &App) -> Vec<Label> {
    if let Some(detail) = app.open_detail() {
        return detail.labels.clone();
    }

    app.view_selected_issue()
        .map(|issue| issue.labels.clone())
        .unwrap_or_default()
}

pub(super) fn priority_items() -> Vec<PickerItem> {
    [
        Priority::Urgent,
        Priority::High,
        Priority::Medium,
        Priority::Low,
        Priority::None,
    ]
    .into_iter()
    .map(PickerItem::from)
    .collect()
}

pub(super) fn open_comment_input(app: &mut App) -> Report {
    let target = match require(app.action_target(), Status::NeedOpenIssue) {
        Ok(target) => target,
        Err(status) => return Report::status(status),
    };

    open_editor(app, target.id, Compose::Comment, target.team_id, None).into()
}

pub(super) fn status_items(states: &[StateOption]) -> Vec<PickerItem> {
    states.iter().cloned().map(PickerItem::from).collect()
}

pub(super) fn assign_suggestions(app: &App) -> Vec<PickerItem> {
    let mut items = vec![PickerItem::unassign()];

    if let Some(session) = app.workspace.session.value() {
        items.push(PickerItem::from(session.user.clone()));
    }

    items
}

pub(super) fn found_users(users: Vec<User>) -> Vec<PickerItem> {
    users.into_iter().map(PickerItem::from).collect()
}

pub(super) fn fill_picker(picker: &mut Picker, items: Vec<PickerItem>) {
    let was_empty = picker.items.is_empty();
    picker.items = items;

    if was_empty {
        picker.state.select(Some(0));
    } else {
        clamp_selection(&mut picker.state, picker.items.len());
    }
}

pub(super) fn stop_assign_picker(picker: Option<&mut Picker>) {
    if let Some(picker) = picker {
        picker.settle_search();
    }
}

pub(super) fn access_states(app: &mut App, team_id: &TeamId) -> Effects {
    let began = app
        .workspace
        .states
        .get_or_default(team_id)
        .begin_access(app.now, &STATES_REFRESH);

    Effects::when(
        began,
        Effect::Api(ApiCommand::LoadStates {
            team_id: team_id.clone(),
        }),
    )
}

pub(super) fn access_members(app: &mut App, team_id: &TeamId) -> Effects {
    let began = app
        .workspace
        .members
        .get_or_default(team_id)
        .begin_access(app.now, &MEMBERS_REFRESH);

    Effects::when(
        began,
        Effect::Api(ApiCommand::LoadMembers {
            team_id: team_id.clone(),
        }),
    )
}

pub(super) fn open_picker(app: &mut App, kind: PickerKind, target: FocusedIssue) -> Effects {
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
        PickerKind::Assign(_) => (assign_suggestions(app), Effects::default()),
        PickerKind::Priority => (priority_items(), Effects::default()),
    };

    app.set_overlay(Overlay::Picker(Picker {
        kind,
        target_issue: target.id,
        target_label: target.identifier,
        target_team: team_id,
        items,
        state: ListState::default().with_selected(Some(0)),
    }));

    command
}

pub(super) fn place_editor(
    app: &mut App,
    issue_id: IssueId,
    compose: Compose,
    team_id: &TeamId,
    seed: Option<&str>,
) {
    let mut editor = match seed {
        Some(body) => Editor::seeded(issue_id, team_id.clone(), compose, body),
        None => Editor::new(issue_id, team_id.clone(), compose),
    };
    editor.set_members(
        app.workspace
            .members
            .get(team_id)
            .and_then(Remote::value)
            .cloned()
            .unwrap_or_default(),
    );
    app.set_overlay(Overlay::Editor(editor));
}

pub(super) fn open_editor(
    app: &mut App,
    issue_id: IssueId,
    compose: Compose,
    team_id: TeamId,
    seed: Option<&str>,
) -> Effects {
    place_editor(app, issue_id, compose, &team_id, seed);

    access_members(app, &team_id)
}

pub(super) fn open_in_browser(app: &mut App) -> Report {
    let target = match require(app.open_target(), Status::NeedHighlightedIssue) {
        Ok(target) => target,
        Err(status) => return Report::status(status),
    };

    Effects::one(Effect::Platform(PlatformCommand::OpenUrl(target.url))).into()
}

pub(super) fn yank_url(app: &mut App) -> Report {
    let target = match require(app.open_target(), Status::NeedHighlightedIssue) {
        Ok(target) => target,
        Err(status) => return Report::status(status),
    };

    Report::with_status(
        Effects::one(Effect::Platform(PlatformCommand::CopyToClipboard(
            target.url,
        ))),
        Status::CopiedUrl,
    )
}

pub(super) fn require<T>(target: Option<T>, status: Status) -> Result<T, Status> {
    target.ok_or(status)
}

pub(super) fn newest_comment_index(detail: &crate::api::IssueDetail) -> Option<usize> {
    detail
        .threaded_comments()
        .iter()
        .enumerate()
        .max_by_key(|(_, threaded)| threaded.comment.created_at)
        .map(|(index, _)| index)
}
