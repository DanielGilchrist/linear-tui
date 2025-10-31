use anyhow::Result;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{ListState, ScrollbarState},
    Frame,
};
use std::sync::Arc;

use super::components::{Inbox, IssueDetail, IssuePreview, IssuesList, TeamsList};
use crate::api::{issue, notifications::Notification, team_issues, Client, Team};

const SCROLL_STEP: usize = 2;
const SIDEBAR_PCT: u16 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Inbox,
    Teams,
    Issues,
    IssueDetail,
}

impl Focus {
    const SIDEBAR: &[Focus] = &[Focus::Inbox, Focus::Teams];
}

enum ListDirection {
    Next,
    Previous,
}

pub struct App {
    pub client: Arc<Client>,
    pub focus: Focus,
    pub notifications: Vec<Notification>,
    pub inbox_list_state: ListState,
    pub teams: Vec<Team>,
    pub team_list_state: ListState,
    pub issues: Vec<team_issues::Issue>,
    pub issue_list_state: ListState,
    pub selected_team: Option<Team>,
    pub selected_issue: Option<issue::Issue>,
    pub previewing: bool,
    pub scroll_position: usize,
    pub scroll_state: ScrollbarState,
}

impl App {
    pub fn new(client: Client) -> Self {
        Self {
            client: Arc::new(client),
            focus: Focus::Inbox,
            notifications: Vec::new(),
            inbox_list_state: ListState::default().with_selected(Some(0)),
            teams: Vec::new(),
            team_list_state: ListState::default().with_selected(Some(0)),
            issues: Vec::new(),
            issue_list_state: ListState::default().with_selected(Some(0)),
            selected_team: None,
            selected_issue: None,
            previewing: false,
            scroll_position: 0,
            scroll_state: ScrollbarState::default(),
        }
    }

    pub async fn load_notifications(&mut self) -> Result<()> {
        let all_notifications = self.client.get_notifications().await?;
        self.notifications = Self::deduplicate_notifications(all_notifications);

        if !self.notifications.is_empty() {
            self.inbox_list_state.select(Some(0));
        }

        Ok(())
    }

    pub async fn load_teams(&mut self) -> Result<()> {
        self.teams = self.client.get_teams().await?;

        if !self.teams.is_empty() {
            self.team_list_state.select(Some(0));
        }

        Ok(())
    }

    pub async fn load_team_issues(&mut self, team_id: &str) -> Result<()> {
        self.issues = self.client.get_team_issues(team_id).await?;

        if !self.issues.is_empty() {
            self.issue_list_state.select(Some(0));
        }

        Ok(())
    }

    pub async fn load_issue_detail(&mut self, issue_id: &str) -> Result<()> {
        if let Some(issue) = self.client.get_issue(issue_id).await? {
            self.selected_issue = Some(issue);
        }

        self.previewing = false;
        self.scroll_position = 0;
        self.scroll_state = ScrollbarState::default();

        Ok(())
    }

    pub fn toggle_preview(&mut self) {
        if self.focus != Focus::Issues {
            return;
        }

        self.previewing = !self.previewing;
    }

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
                self.scroll_position = self.scroll_position.saturating_add(SCROLL_STEP);
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
                self.scroll_position = self.scroll_position.saturating_sub(SCROLL_STEP);
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

    pub fn render(&mut self, frame: &mut Frame) {
        let [sidebar_area, content_area] = split_horizontal(frame.area(), SIDEBAR_PCT);
        let sidebar_chunks = split_sidebar(sidebar_area, Focus::SIDEBAR.len() as u32);

        Inbox::new(&self.notifications, &mut self.inbox_list_state)
            .focused(self.focus == Focus::Inbox)
            .panel_number(1)
            .render(frame, sidebar_chunks[0]);

        TeamsList::new(&self.teams, &mut self.team_list_state)
            .focused(self.focus == Focus::Teams)
            .panel_number(2)
            .render(frame, sidebar_chunks[1]);

        if let Some(issue) = &self.selected_issue {
            if self.focus == Focus::IssueDetail {
                let mut detail =
                    IssueDetail::new(issue, self.scroll_position, &mut self.scroll_state);
                detail.render(frame, content_area);
                self.scroll_position = detail.clamped_scroll_position();
                return;
            }
        }

        if self.previewing && self.focus == Focus::Issues {
            if let Some(issue) = self.get_selected_team_issue_cloned() {
                let [list_area, preview_area] = split_content_with_preview(content_area);

                let has_issues = self.selected_team.is_some() && !self.issues.is_empty();
                IssuesList::new(&self.issues, &mut self.issue_list_state)
                    .focused(true)
                    .show_placeholder(!has_issues)
                    .render(frame, list_area);

                IssuePreview::new(&issue).render(frame, preview_area);
                return;
            }
        }

        let has_issues = self.selected_team.is_some() && !self.issues.is_empty();
        IssuesList::new(&self.issues, &mut self.issue_list_state)
            .focused(self.focus == Focus::Issues)
            .show_placeholder(!has_issues)
            .render(frame, content_area);
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

    fn get_selected_notification(&self) -> Option<&Notification> {
        self.inbox_list_state
            .selected()
            .and_then(|i| self.notifications.get(i))
    }

    fn get_selected_team(&self) -> Option<&Team> {
        self.team_list_state
            .selected()
            .and_then(|i| self.teams.get(i))
    }

    fn get_selected_team_issue(&self) -> Option<&team_issues::Issue> {
        self.issue_list_state
            .selected()
            .and_then(|i| self.issues.get(i))
    }

    fn get_selected_team_issue_cloned(&self) -> Option<team_issues::Issue> {
        self.get_selected_team_issue().cloned()
    }

    fn deduplicate_notifications(notifications: Vec<Notification>) -> Vec<Notification> {
        use std::collections::hash_map::Entry;
        use std::collections::HashMap;

        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut result = Vec::new();

        for notification in notifications {
            let key = notification.grouping_key().to_string();

            if let Entry::Vacant(e) = seen.entry(key) {
                e.insert(result.len());
                result.push(notification);
            }
        }

        result
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

pub enum NavigateAction {
    LoadTeamIssues(String),
    LoadIssue(String),
}

fn split_horizontal(area: Rect, left_pct: u16) -> [Rect; 2] {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(left_pct),
            Constraint::Percentage(100 - left_pct),
        ])
        .split(area);

    [chunks[0], chunks[1]]
}

fn split_sidebar(area: Rect, panel_count: u32) -> Vec<Rect> {
    let constraints: Vec<Constraint> = (0..panel_count)
        .map(|_| Constraint::Ratio(1, panel_count))
        .collect();

    Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area)
        .to_vec()
}

fn split_content_with_preview(area: Rect) -> [Rect; 2] {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    [chunks[0], chunks[1]]
}
