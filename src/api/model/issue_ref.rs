use super::id::IssueId;
use super::issue::{IssueDetail, IssueSummary};

/// How an issue was designated before it is fetched.
/// A pasted URL or typed `DAN-14` yields an identifier, which is not an id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueRef {
    Id(IssueId),
    Identifier(String),
}

impl IssueRef {
    pub fn parse(input: &str) -> Self {
        let trimmed = input.trim();
        let candidate = identifier_from_url(trimmed).unwrap_or(trimmed);

        if looks_like_identifier(candidate) {
            IssueRef::Identifier(candidate.to_uppercase())
        } else {
            IssueRef::Id(IssueId::from_raw(candidate))
        }
    }

    pub fn matches_detail(&self, detail: &IssueDetail) -> bool {
        match self {
            IssueRef::Id(id) => detail.id == *id,
            IssueRef::Identifier(identifier) => detail.identifier == *identifier,
        }
    }

    pub fn matches_summary(&self, issue: &IssueSummary) -> bool {
        match self {
            IssueRef::Id(id) => issue.id == *id,
            IssueRef::Identifier(identifier) => issue.identifier == *identifier,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            IssueRef::Id(id) => id.as_str(),
            IssueRef::Identifier(identifier) => identifier,
        }
    }
}

impl From<IssueId> for IssueRef {
    fn from(id: IssueId) -> Self {
        IssueRef::Id(id)
    }
}

impl std::fmt::Display for IssueRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

fn identifier_from_url(input: &str) -> Option<&str> {
    let id = input.split("/issue/").nth(1)?.split('/').next()?;
    (!id.is_empty()).then_some(id)
}

fn looks_like_identifier(candidate: &str) -> bool {
    match candidate.split_once('-') {
        Some((team, number)) => {
            !team.is_empty()
                && team.chars().all(|c| c.is_ascii_alphanumeric())
                && !number.is_empty()
                && number.chars().all(|c| c.is_ascii_digit())
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identifier(raw: &str) -> IssueRef {
        IssueRef::Identifier(raw.to_string())
    }

    #[test]
    fn parses_a_bare_identifier() {
        assert_eq!(IssueRef::parse("DAN2-7"), identifier("DAN2-7"));
    }

    #[test]
    fn uppercases_a_lowercase_identifier() {
        assert_eq!(IssueRef::parse("dan-14"), identifier("DAN-14"));
        assert_eq!(IssueRef::parse("  dan2-7  "), identifier("DAN2-7"));
    }

    #[test]
    fn extracts_the_identifier_from_a_url() {
        assert_eq!(
            IssueRef::parse("https://linear.app/dans-donuts/issue/DAN2-7/wood-fired-oven"),
            identifier("DAN2-7")
        );
    }

    #[test]
    fn anything_that_is_not_an_identifier_is_taken_as_an_id() {
        assert_eq!(
            IssueRef::parse("dan-abc"),
            IssueRef::Id(IssueId::from_raw("dan-abc"))
        );
    }
}
