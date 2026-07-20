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
    pub url: String,
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
#[cynic(schema_path = "schema.graphql", graphql_type = "IssueUpdateInput")]
pub struct StatusInput {
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub state_id: Option<String>,
}

// No `skip_serializing_if`: `None` serialises as explicit `null`, which unassigns.
// TODO: fold back into one input via MaybeUndefined once https://codeberg.org/obmarg/cynic/issues/125 lands.
#[derive(Debug, Clone, InputObject)]
#[cynic(schema_path = "schema.graphql", graphql_type = "IssueUpdateInput")]
pub struct AssigneeInput {
    pub assignee_id: Option<String>,
}

#[derive(Debug, QueryVariables)]
pub struct StatusVariables {
    pub id: String,
    pub input: StatusInput,
}

#[derive(Debug, QueryVariables)]
pub struct AssigneeVariables {
    pub id: String,
    pub input: AssigneeInput,
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
    variables = "StatusVariables"
)]
pub struct StatusMutation {
    #[arguments(id: $id, input: $input)]
    pub issue_update: IssuePayload,
}

#[derive(Debug, QueryFragment)]
#[cynic(
    schema_path = "schema.graphql",
    graphql_type = "Mutation",
    variables = "AssigneeVariables"
)]
pub struct AssigneeMutation {
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

#[derive(Debug, Clone, InputObject)]
#[cynic(schema_path = "schema.graphql")]
pub struct CommentUpdateInput {
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

#[derive(Debug, QueryVariables)]
pub struct CommentUpdateVariables {
    pub id: String,
    pub input: CommentUpdateInput,
}

#[derive(Debug, QueryFragment)]
#[cynic(
    schema_path = "schema.graphql",
    graphql_type = "Mutation",
    variables = "CommentUpdateVariables"
)]
pub struct CommentUpdateMutation {
    #[arguments(id: $id, input: $input)]
    pub comment_update: CommentPayload,
}
