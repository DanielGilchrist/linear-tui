use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use ratatui::{
    layout::Direction,
    widgets::{ListState, ScrollbarState},
    Frame,
};

use super::components::{Inbox, IssueDetail, IssuePreview, IssuesList, TeamsList};
use super::layout;
use crate::api::{issue, notifications::Notification, team_issues, Client, Team};

pub const SCROLL_STEP: usize = 2;
const SIDEBAR_PCT: u16 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Inbox,
    Teams,
    Issues,
    IssueDetail,
}

impl Focus {
    pub const SIDEBAR: &[Focus] = &[Focus::Inbox, Focus::Teams];
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

    pub fn render(&mut self, frame: &mut Frame) {
        let [sidebar_area, content_area] = layout::split_horizontal(frame.area(), SIDEBAR_PCT);
        let sidebar_chunks = layout::split_even(
            sidebar_area,
            Direction::Vertical,
            Focus::SIDEBAR.len() as u32,
        );

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
                let [list_area, preview_area] =
                    layout::split_half(content_area, Direction::Vertical);

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

    pub(super) fn get_selected_notification(&self) -> Option<&Notification> {
        self.inbox_list_state
            .selected()
            .and_then(|i| self.notifications.get(i))
    }

    pub(super) fn get_selected_team(&self) -> Option<&Team> {
        self.team_list_state
            .selected()
            .and_then(|i| self.teams.get(i))
    }

    pub(super) fn get_selected_team_issue(&self) -> Option<&team_issues::Issue> {
        self.issue_list_state
            .selected()
            .and_then(|i| self.issues.get(i))
    }

    fn get_selected_team_issue_cloned(&self) -> Option<team_issues::Issue> {
        self.get_selected_team_issue().cloned()
    }

    fn deduplicate_notifications(notifications: Vec<Notification>) -> Vec<Notification> {
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
}
