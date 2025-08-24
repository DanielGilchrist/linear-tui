use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Comment {
    pub body: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Issue {
    pub title: String,
    pub description: String,
    pub comments: CommentsWrapper,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommentsWrapper {
    pub nodes: Vec<Comment>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Team {
    pub id: String,
    pub name: String,
    #[serde(rename = "issueCount")]
    pub issue_count: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TeamIssue {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TeamsWrapper {
    pub nodes: Vec<Team>,
}

#[derive(Debug, Deserialize)]
pub struct IssuesWrapper {
    pub nodes: Vec<TeamIssue>,
}

#[derive(Debug, Deserialize)]
pub struct TeamsResponse {
    pub data: TeamsData,
}

#[derive(Debug, Deserialize)]
pub struct TeamsData {
    pub teams: TeamsWrapper,
}

#[derive(Debug, Deserialize)]
pub struct IssueResponse {
    pub data: IssueData,
}

#[derive(Debug, Deserialize)]
pub struct IssueData {
    pub issue: Issue,
}

#[derive(Debug, Deserialize)]
pub struct TeamIssuesResponse {
    pub data: TeamIssuesData,
}

#[derive(Debug, Deserialize)]
pub struct TeamIssuesData {
    pub team: TeamWithIssues,
}

#[derive(Debug, Deserialize)]
pub struct TeamWithIssues {
    pub issues: IssuesWrapper,
}
