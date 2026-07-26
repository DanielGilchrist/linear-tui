use ratatui::widgets::ScrollbarState;

use super::feed::{
    access_feed, feed_keep_id, reconcile_feed, resolve, revalidate_focus, save_feeds_command,
    selected_view_key,
};
use super::issue::{assign_items, fill_picker, newest_comment_index, status_items, stop_picker};
use super::nav::clamp_selection;
use crate::api::IssueSummary;
use crate::tui::app::{App, RECENT_CAP};
use crate::tui::cache::Stale;
use crate::tui::feed::FeedRequest;
use crate::tui::focus::{DetailView, Focus, LeftPanel, Reveal};
use crate::tui::message::{Command, FailureTarget, Message};
use crate::tui::overlay::{Overlay, PickerKind};
use crate::tui::status::Status;
use crate::tui::view::ViewKind;

pub fn apply(app: &mut App, msg: Message) -> Option<Command> {
    match msg {
        Message::SessionLoaded(session) => {
            app.workspace.session = Some(session);
            None
        }
        Message::FeedLoaded { key, request, page } => {
            let keep = feed_keep_id(app, &key);

            let applied = app
                .workspace
                .feeds
                .get_or_default(&key)
                .apply(&request, page, app.now);
            if !applied {
                return None;
            }

            app.status = None;
            reconcile_feed(app, &key, keep);

            match request {
                FeedRequest::Refresh => Some(save_feeds_command(app)),
                FeedRequest::LoadMore { .. } => None,
            }
        }
        Message::InboxLoaded { request, page } => {
            let inbox_active = match app.active_view().kind {
                ViewKind::Inbox => true,
                ViewKind::Issues(_) => false,
            };
            let keep = inbox_active
                .then(|| app.selected_notification().map(|n| n.grouping_key.clone()))
                .flatten();

            let applied = app.workspace.inbox.apply(&request, page, app.now);
            if !applied {
                return None;
            }

            app.status = None;

            if inbox_active {
                let idx = resolve(
                    app.workspace.inbox.items(),
                    keep.as_deref(),
                    app.list_state.selected(),
                );

                app.list_state.select(idx);
            }

            match request {
                FeedRequest::Refresh => Some(save_feeds_command(app)),
                FeedRequest::LoadMore { .. } => None,
            }
        }
        Message::CustomViewsLoaded(views) => {
            app.workspace.saved_views.views.set(views, app.now);

            let len = app.workspace.saved_views.list().len();
            clamp_selection(&mut app.workspace.saved_views.state, len);

            selected_view_key(app).and_then(|key| access_feed(app, key))
        }
        Message::DetailLoaded { detail, reveal } => {
            app.workspace.set_detail(*detail, app.now);
            app.status = None;

            app.scroll_position = match reveal {
                Reveal::Top | Reveal::NewestComment => 0,
                Reveal::Bottom => usize::MAX,
            };

            app.scroll_state = ScrollbarState::default();

            let detail = app.workspace.detail().value()?;
            let summary = IssueSummary::from_detail(detail);
            let comment_count = detail.comments.len();
            let newest_comment = match reveal {
                Reveal::NewestComment => newest_comment_index(detail),
                _ => None,
            };

            if let Focus::Detail(panel, DetailView::Comments) = app.focus {
                if comment_count == 0 {
                    app.focus = Focus::Detail(panel, DetailView::Reading);
                } else if let Some(index) = newest_comment {
                    app.comment_state.select(Some(index));
                } else {
                    clamp_selection(&mut app.comment_state, comment_count);
                }
            }

            app.record_recent(summary);

            Some(Command::SaveRecent(app.workspace.recently_viewed.clone()))
        }
        Message::RecentLoaded(mut issues) => {
            if app.workspace.recently_viewed.is_empty() {
                issues.truncate(RECENT_CAP);
                app.workspace.recently_viewed = issues;
                app.workspace.recent_state.select(Some(0));
            }

            None
        }
        Message::RecentCleared => {
            app.workspace.recently_viewed.clear();
            app.workspace.recent_state.select(Some(0));
            app.status = Some(Status::RecentCleared);

            if app.focus.left() == LeftPanel::Recent {
                app.focus = Focus::MyWork;
            }

            None
        }
        Message::StatesLoaded { team_id, states } => {
            let items = status_items(&states);

            app.workspace
                .states
                .get_or_default(&team_id)
                .set(states, app.now);

            if let Overlay::Picker(picker) = &mut app.overlay {
                if picker.kind == PickerKind::Status {
                    fill_picker(picker, items);
                }
            }
            None
        }
        Message::MembersLoaded { team_id, members } => {
            let items = assign_items(&members);

            app.workspace
                .members
                .get_or_default(&team_id)
                .set(members.clone(), app.now);

            match &mut app.overlay {
                Overlay::Picker(picker) if picker.kind == PickerKind::Assign => {
                    fill_picker(picker, items)
                }
                Overlay::Editor(editor) => editor.members = members,
                _ => {}
            }

            None
        }
        Message::IssueUpdated { id } => {
            app.status = Some(Status::IssueUpdated);
            app.workspace.feeds.invalidate_all();
            app.workspace.inbox.mark_stale();
            let refresh = revalidate_focus(app);

            match app.focus {
                Focus::Detail(..) => {
                    app.workspace.begin_detail();

                    let detail = Command::LoadDetail {
                        id,
                        reveal: Reveal::Top,
                    };

                    Some(match refresh {
                        Some(refresh) => Command::Batch(vec![refresh, detail]),
                        None => detail,
                    })
                }
                _ => refresh,
            }
        }
        Message::CommentPosted { id } => {
            app.status = Some(Status::CommentPosted);
            app.workspace.begin_detail();

            let reveal = match app.focus {
                Focus::Detail(_, DetailView::Comments) => Reveal::NewestComment,
                _ => Reveal::Bottom,
            };

            Some(Command::LoadDetail { id, reveal })
        }
        Message::CommentEdited { id } => {
            app.status = Some(Status::CommentEdited);
            app.workspace.begin_detail();

            Some(Command::LoadDetail {
                id,
                reveal: Reveal::Top,
            })
        }
        Message::CommentDeleted { id } => {
            app.status = Some(Status::CommentDeleted);
            app.workspace.begin_detail();

            Some(Command::LoadDetail {
                id,
                reveal: Reveal::Top,
            })
        }
        Message::Failed { target, error } => {
            match target {
                FailureTarget::Feed(key) => {
                    app.workspace.feeds.get_or_default(&key).fail(error.clone())
                }
                FailureTarget::Inbox => app.workspace.inbox.fail(error.clone()),
                FailureTarget::CustomViews => app.workspace.saved_views.views.fail(error.clone()),
                FailureTarget::Detail => app.workspace.fail_detail(error.clone()),
                FailureTarget::States { team_id } => {
                    app.workspace
                        .states
                        .get_or_default(&team_id)
                        .fail(error.clone());
                    stop_picker(&mut app.overlay, PickerKind::Status);
                }
                FailureTarget::Members { team_id } => {
                    app.workspace
                        .members
                        .get_or_default(&team_id)
                        .fail(error.clone());
                    stop_picker(&mut app.overlay, PickerKind::Assign);
                }
                FailureTarget::Ephemeral => {}
            }

            app.status = Some(Status::Error(error));
            None
        }
    }
}
