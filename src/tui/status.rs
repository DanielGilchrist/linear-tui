use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    NoActiveSearch,
    FindInList,
    NothingToSearch,
    NoComments,
    Cancelled,
    PostingComment,
    RecentCleared,
    IssueUpdated,
    CommentPosted,
    CopiedUrl,
    Applying,
    NeedHighlightedIssue,
    NeedOpenIssue,
    Error(String),
}

impl Status {
    pub fn is_error(&self) -> bool {
        matches!(self, Status::Error(_))
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Status::NoActiveSearch => "No active search (press /)",
            Status::FindInList => "Find works in a list",
            Status::NothingToSearch => "Nothing to search",
            Status::NoComments => "No comments to reply to",
            Status::Cancelled => "Cancelled",
            Status::PostingComment => "Posting comment…",
            Status::RecentCleared => "Recently viewed cleared",
            Status::IssueUpdated => "Issue updated",
            Status::CommentPosted => "Comment posted",
            Status::CopiedUrl => "Copied issue URL to clipboard",
            Status::Applying => "Applying…",
            Status::NeedHighlightedIssue => "Highlight an issue first",
            Status::NeedOpenIssue => "Open the issue first (enter)",
            Status::Error(message) => return f.write_str(message),
        };
        f.write_str(text)
    }
}
