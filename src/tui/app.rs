use anyhow::Result;
use ratatui::{
    widgets::{ListState, ScrollbarState},
    Frame,
};
use std::sync::Arc;

use super::components::{IssueDetail, IssuesList, Renderable, TeamsList};
use super::layout::TwoColumnLayout;
use crate::api::{issue, team_issues, Client, Team};

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    TeamsList,
    IssuesList,
    IssueDetail,
}

enum ListDirection {
    Next,
    Previous,
}

pub struct App {
    pub client: Arc<Client>,
    pub state: AppState,
    pub teams: Vec<Team>,
    pub team_list_state: ListState,
    pub issues: Vec<team_issues::Issue>,
    pub issue_list_state: ListState,
    pub selected_team: Option<Team>,
    pub selected_issue: Option<issue::Issue>,
    pub scroll_position: usize,
    pub scroll_state: ScrollbarState,
}

impl App {
    pub fn new(client: Client) -> Self {
        let mut team_list_state = ListState::default();
        team_list_state.select(Some(0));

        let mut issue_list_state = ListState::default();
        issue_list_state.select(Some(0));

        Self {
            client: Arc::new(client),
            state: AppState::TeamsList,
            teams: Vec::new(),
            team_list_state,
            issues: Vec::new(),
            issue_list_state,
            selected_team: None,
            selected_issue: None,
            scroll_position: 0,
            scroll_state: ScrollbarState::default(),
        }
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

        self.reset_scroll_position();

        Ok(())
    }

    fn reset_scroll_position(&mut self) {
        self.scroll_position = 0;
        self.scroll_state = ScrollbarState::default();
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

    pub fn next_item(&mut self) {
        match self.state {
            AppState::TeamsList => Self::navigate_list(
                &mut self.team_list_state,
                self.teams.len(),
                ListDirection::Next,
            ),
            AppState::IssuesList => Self::navigate_list(
                &mut self.issue_list_state,
                self.issues.len(),
                ListDirection::Next,
            ),
            AppState::IssueDetail => {
                self.scroll_position = self.scroll_position.saturating_add(1);
            }
        }
    }

    pub fn previous_item(&mut self) {
        match self.state {
            AppState::TeamsList => Self::navigate_list(
                &mut self.team_list_state,
                self.teams.len(),
                ListDirection::Previous,
            ),
            AppState::IssuesList => Self::navigate_list(
                &mut self.issue_list_state,
                self.issues.len(),
                ListDirection::Previous,
            ),
            AppState::IssueDetail => {
                self.scroll_position = self.scroll_position.saturating_sub(1);
            }
        }
    }

    pub fn get_selected_team(&self) -> Option<&Team> {
        self.team_list_state
            .selected()
            .and_then(|i| self.teams.get(i))
    }

    pub fn get_selected_team_issue(&self) -> Option<&team_issues::Issue> {
        self.issue_list_state
            .selected()
            .and_then(|i| self.issues.get(i))
    }

    pub fn render(&mut self, frame: &mut Frame) {
        match self.state {
            AppState::TeamsList | AppState::IssuesList => {
                let teams_component = TeamsList::new(&self.teams, &mut self.team_list_state)
                    .focused(self.state == AppState::TeamsList);

                let issues_component = IssuesList::new(&self.issues, &mut self.issue_list_state)
                    .focused(self.state == AppState::IssuesList)
                    .show_placeholder(self.state != AppState::IssuesList || self.issues.is_empty());

                let mut layout = TwoColumnLayout::new(teams_component, issues_component);
                layout.render(frame, frame.area());
            }
            AppState::IssueDetail => {
                if let Some(issue) = &self.selected_issue {
                    let mut issue_detail =
                        IssueDetail::new(issue, self.scroll_position, &mut self.scroll_state);
                    issue_detail.render(frame, frame.area());

                    self.scroll_position = issue_detail.get_clamped_scroll_position();
                }
            }
        }
    }
}
