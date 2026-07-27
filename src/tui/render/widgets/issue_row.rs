use ratatui::{
    text::{Line, Span},
    widgets::ListItem,
};

use super::super::format;
use super::super::theme;
use crate::api::{IssueSummary, Label, Timestamp, User};
use crate::tui::display::{Column, GroupBy};

pub fn issue_items(issues: &[IssueSummary]) -> Vec<ListItem<'static>> {
    issues
        .iter()
        .map(|issue| ListItem::new(issue_item(issue)))
        .collect()
}

fn title(issue: &IssueSummary) -> String {
    issue.title.clone().unwrap_or_else(|| "Untitled".into())
}

fn state_span(issue: &IssueSummary) -> Span<'static> {
    Span::styled(
        issue.state.name.clone(),
        theme::state(issue.state.state_type),
    )
}

fn assignee_span(assignee: &User) -> Span<'static> {
    Span::styled(assignee.display_name.clone(), theme::PERSON)
}

fn label_chip(label: &Label) -> Span<'static> {
    Span::styled(format!(" {} ", label.name), theme::label_chip(label.colour))
}

fn issue_item(issue: &IssueSummary) -> Line<'static> {
    let mut spans = vec![
        theme::priority_glyph(issue.priority),
        Span::raw(" "),
        Span::styled(issue.identifier.clone(), theme::DIM),
        Span::raw(" "),
        state_span(issue),
        Span::raw(" "),
        Span::styled(title(issue), theme::TEXT),
    ];

    if let Some(assignee) = &issue.assignee {
        spans.push(Span::raw(" "));
        spans.push(assignee_span(assignee));
    }

    for label in &issue.labels {
        spans.push(Span::raw(" "));
        spans.push(label_chip(label));
    }

    Line::from(spans)
}

pub(super) fn issue_row(
    issue: &IssueSummary,
    group: GroupBy,
    id_width: usize,
    width: usize,
    now: Timestamp,
) -> Line<'static> {
    let omit = group.omitted_column();

    let mut left: Vec<Span> = Vec::new();

    if omit == Some(Column::Priority) {
        left.push(Span::raw("    "));
    } else {
        left.push(theme::priority_glyph(issue.priority));
        left.push(Span::raw(" "));
    }

    left.push(Span::styled(
        format!("{:<id_width$}", issue.identifier),
        theme::DIM,
    ));

    left.push(Span::raw(" "));

    let left_w = 4 + id_width + 1;

    let mut right: Vec<Span> = Vec::new();

    if omit != Some(Column::State) {
        right.push(state_span(issue));
        right.push(Span::raw("  "));
    }

    for label in &issue.labels {
        right.push(label_chip(label));
        right.push(Span::raw(" "));
    }

    if omit != Some(Column::Assignee) {
        if let Some(assignee) = &issue.assignee {
            right.push(assignee_span(assignee));
            right.push(Span::raw("  "));
        }
    }

    right.push(Span::styled(issue.updated_at.age_short(now), theme::DIM));

    let right_w: usize = right.iter().map(|span| span.width()).sum();
    let gap = 2;
    let title_area = width.saturating_sub(left_w + right_w + gap);
    let title = format::fit(&title(issue), title_area);
    let pad = title_area.saturating_sub(format::width(&title)) + gap;
    let mut spans = left;

    spans.push(Span::styled(title, theme::TEXT));
    spans.push(Span::raw(" ".repeat(pad)));
    spans.extend(right);

    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{
        IssueId, Priority, StateType, TeamId, Timestamp, User, UserId, WorkflowState,
    };

    fn issue(identifier: &str, title: &str) -> IssueSummary {
        IssueSummary {
            id: IssueId::from_raw(identifier),
            identifier: identifier.into(),
            title: Some(title.into()),
            state: WorkflowState {
                name: "In Progress".into(),
                state_type: StateType::Started,
            },
            priority: Priority::Urgent,
            assignee: Some(User {
                id: UserId::from_raw("u"),
                name: "dan".into(),
                display_name: "dan".into(),
                url: String::new(),
                is_me: false,
            }),
            labels: Vec::new(),
            url: String::new(),
            branch_name: String::new(),
            team_id: TeamId::default(),
            updated_at: Timestamp::default(),
        }
    }

    fn text(line: &Line) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    #[test]
    fn identifier_is_padded_to_the_id_width() {
        let row = issue_row(
            &issue("DAN2-7", "short"),
            GroupBy::None,
            8,
            80,
            Timestamp::default(),
        );
        assert_eq!(&text(&row)[..12], "!!! DAN2-7  ");
    }

    #[test]
    fn a_row_fills_exactly_the_given_width_when_the_title_fits() {
        let issue = issue("DAN2-7", "a short title");
        for width in [70, 90, 120] {
            let row = issue_row(&issue, GroupBy::None, 6, width, Timestamp::default());
            assert_eq!(row.width(), width, "row should fill width {width}");
        }
    }

    #[test]
    fn a_wide_glyph_title_still_fills_exactly_the_width() {
        let issue = issue("DAN2-7", "日本語のタイトル");
        for width in [70, 90, 120] {
            let row = issue_row(&issue, GroupBy::None, 6, width, Timestamp::default());
            assert_eq!(
                row.width(),
                width,
                "wide-glyph row should fill width {width}"
            );
        }
    }

    #[test]
    fn a_long_title_is_truncated_with_an_ellipsis() {
        let issue = issue(
            "DAN2-7",
            "a very long title that will not fit into the narrow column at all",
        );
        let row = issue_row(&issue, GroupBy::None, 6, 40, Timestamp::default());
        assert!(
            text(&row).contains('…'),
            "narrow row should truncate: {}",
            text(&row)
        );
        assert!(row.width() <= 40, "truncated row must not exceed width");
    }

    #[test]
    fn grouping_omits_the_redundant_column() {
        let issue = issue("DAN2-7", "title");

        let by_priority = issue_row(&issue, GroupBy::Priority, 6, 80, Timestamp::default());
        assert!(text(&by_priority).starts_with("    "));
        assert!(!text(&by_priority).contains("!!!"));

        let by_status = issue_row(&issue, GroupBy::Status, 6, 80, Timestamp::default());
        assert!(!text(&by_status).contains("In Progress"));

        let by_assignee = issue_row(&issue, GroupBy::Assignee, 6, 80, Timestamp::default());
        assert!(!text(&by_assignee).contains("dan"));

        let none = issue_row(&issue, GroupBy::None, 6, 80, Timestamp::default());
        assert!(text(&none).contains("!!!"));
        assert!(text(&none).contains("In Progress"));
        assert!(text(&none).contains("dan"));
    }
}
