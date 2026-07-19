use cynic::{InputObject, QueryFragment, QueryVariables};

mod schema {
    cynic::use_schema!("schema.graphql");
}

#[derive(Debug, Clone, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct WorkflowState {
    pub id: cynic::Id,
    pub name: String,
    #[cynic(rename = "type")]
    pub state_type: String,
    pub position: f64,
}

#[derive(Debug, Clone, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct WorkflowStateConnection {
    pub nodes: Vec<WorkflowState>,
}

#[derive(Debug, Clone, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct User {
    pub id: cynic::Id,
    pub name: String,
    pub display_name: String,
    pub is_me: bool,
}

#[derive(Debug, Clone, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct UserConnection {
    pub nodes: Vec<User>,
}

#[derive(Debug, QueryFragment)]
#[cynic(schema_path = "schema.graphql", graphql_type = "Team")]
pub struct TeamStates {
    pub states: WorkflowStateConnection,
}

#[derive(Debug, QueryFragment)]
#[cynic(schema_path = "schema.graphql", graphql_type = "Team")]
pub struct TeamMembers {
    pub members: UserConnection,
}

#[derive(Debug, QueryVariables)]
pub struct TeamVariables {
    pub id: String,
}

#[derive(Debug, QueryFragment)]
#[cynic(
    schema_path = "schema.graphql",
    graphql_type = "Query",
    variables = "TeamVariables"
)]
pub struct TeamStatesQuery {
    #[arguments(id: $id)]
    pub team: Option<TeamStates>,
}

#[derive(Debug, QueryFragment)]
#[cynic(
    schema_path = "schema.graphql",
    graphql_type = "Query",
    variables = "TeamVariables"
)]
pub struct TeamMembersQuery {
    #[arguments(id: $id)]
    pub team: Option<TeamMembers>,
}

#[derive(Debug, Clone, InputObject)]
#[cynic(schema_path = "schema.graphql")]
pub struct IssueUpdateInput {
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub state_id: Option<String>,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub assignee_id: Option<String>,
}

#[derive(Debug, QueryVariables)]
pub struct IssueUpdateVariables {
    pub id: String,
    pub input: IssueUpdateInput,
}

#[derive(Debug, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct IssuePayload {
    pub success: bool,
}

#[derive(Debug, QueryFragment)]
#[cynic(
    schema_path = "schema.graphql",
    graphql_type = "Mutation",
    variables = "IssueUpdateVariables"
)]
pub struct IssueUpdateMutation {
    #[arguments(id: $id, input: $input)]
    pub issue_update: IssuePayload,
}

#[derive(Debug, Clone, InputObject)]
#[cynic(schema_path = "schema.graphql")]
pub struct CommentCreateInput {
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub issue_id: Option<String>,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
}

#[derive(Debug, QueryVariables)]
pub struct CommentCreateVariables {
    pub input: CommentCreateInput,
}

#[derive(Debug, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct CommentPayload {
    pub success: bool,
}

#[derive(Debug, QueryFragment)]
#[cynic(
    schema_path = "schema.graphql",
    graphql_type = "Mutation",
    variables = "CommentCreateVariables"
)]
pub struct CommentCreateMutation {
    #[arguments(input: $input)]
    pub comment_create: CommentPayload,
}
