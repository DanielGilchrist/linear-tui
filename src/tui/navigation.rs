use ratatui::widgets::ListState;

use super::app::{App, Focus};

pub enum NavigateAction {
    LoadTeamIssues(String),
    LoadIssue(String),
}

enum ListDirection {
    Next,
    Previous,
}

impl App {
    pub fn navigate_to(&mut self) -> Option<NavigateAction> {
        match self.focus {
            Focus::Inbox => {
                let issue_id = self
                    .get_selected_notification()
                    .and_then(|n| n.issue_id())
                    .map(|s| s.to_string());

                if let Some(id) = issue_id {
                    self.focus = Focus::IssueDetail;
                    return Some(NavigateAction::LoadIssue(id));
                }

                None
            }
            Focus::Teams => {
                if let Some(team) = self.get_selected_team() {
                    let team_id = team.id.inner().to_string();
                    self.selected_team = Some(team.clone());
                    self.focus = Focus::Issues;
                    return Some(NavigateAction::LoadTeamIssues(team_id));
                }

                None
            }
            Focus::Issues => {
                if let Some(issue) = self.get_selected_team_issue() {
                    let issue_id = issue.id.inner().to_string();
                    self.focus = Focus::IssueDetail;
                    return Some(NavigateAction::LoadIssue(issue_id));
                }

                None
            }
            Focus::IssueDetail => None,
        }
    }

    pub fn go_back(&mut self) {
        if self.previewing {
            self.previewing = false;
            return;
        }

        match self.focus {
            Focus::IssueDetail => {
                if self.selected_team.is_some() {
                    self.focus = Focus::Issues;
                } else {
                    self.focus = Focus::Inbox;
                }
                self.selected_issue = None;
            }
            Focus::Issues => {
                self.focus = Focus::Teams;
                self.issues.clear();
                self.selected_team = None;
            }
            _ => {}
        }
    }

    pub fn toggle_preview(&mut self) {
        if self.focus != Focus::Issues {
            return;
        }

        self.previewing = !self.previewing;
    }

    pub fn next_item(&mut self) {
        match self.focus {
            Focus::Inbox => Self::navigate_list(
                &mut self.inbox_list_state,
                self.notifications.len(),
                ListDirection::Next,
            ),
            Focus::Teams => Self::navigate_list(
                &mut self.team_list_state,
                self.teams.len(),
                ListDirection::Next,
            ),
            Focus::Issues => Self::navigate_list(
                &mut self.issue_list_state,
                self.issues.len(),
                ListDirection::Next,
            ),
            Focus::IssueDetail => {
                self.scroll_position = self.scroll_position.saturating_add(super::app::SCROLL_STEP);
            }
        }
    }

    pub fn previous_item(&mut self) {
        match self.focus {
            Focus::Inbox => Self::navigate_list(
                &mut self.inbox_list_state,
                self.notifications.len(),
                ListDirection::Previous,
            ),
            Focus::Teams => Self::navigate_list(
                &mut self.team_list_state,
                self.teams.len(),
                ListDirection::Previous,
            ),
            Focus::Issues => Self::navigate_list(
                &mut self.issue_list_state,
                self.issues.len(),
                ListDirection::Previous,
            ),
            Focus::IssueDetail => {
                self.scroll_position = self.scroll_position.saturating_sub(super::app::SCROLL_STEP);
            }
        }
    }

    pub fn next_panel(&mut self) {
        self.previewing = false;
        let panes = self.visible_panes();
        let current = panes.iter().position(|&p| p == self.focus).unwrap_or(0);
        self.focus = panes[(current + 1) % panes.len()];
    }

    pub fn previous_panel(&mut self) {
        self.previewing = false;
        let panes = self.visible_panes();
        let current = panes.iter().position(|&p| p == self.focus).unwrap_or(0);
        let prev = if current == 0 {
            panes.len() - 1
        } else {
            current - 1
        };
        self.focus = panes[prev];
    }

    pub fn jump_to_panel(&mut self, index: usize) {
        if let Some(&pane) = Focus::SIDEBAR.get(index) {
            self.previewing = false;
            self.focus = pane;
        }
    }

    fn visible_panes(&self) -> Vec<Focus> {
        let mut panes: Vec<Focus> = Focus::SIDEBAR.to_vec();

        if self.selected_issue.is_some() {
            panes.push(Focus::IssueDetail);
        } else if self.selected_team.is_some() {
            panes.push(Focus::Issues);
        }

        panes
    }

    fn navigate_list(state: &mut ListState, list_size: usize, direction: ListDirection) {
        if list_size == 0 {
            return;
        }

        let index = match state.selected() {
            Some(index) => match direction {
                ListDirection::Next => (index + 1) % list_size,
                ListDirection::Previous => {
                    if index == 0 {
                        list_size - 1
                    } else {
                        index - 1
                    }
                }
            },
            None => 0,
        };

        state.select(Some(index));
    }
}
