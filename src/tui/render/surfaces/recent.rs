use ratatui::{
    layout::Rect,
    text::{Line, Text},
    widgets::ListState,
    Frame,
};

use super::super::theme::Emphasis;
use super::super::widgets::{issue_items, preview_text, text_panel, StyledList};
use crate::api::IssueSummary;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    issues: &[IssueSummary],
    state: &mut ListState,
    emphasis: Emphasis,
) {
    let selected = state.selected();
    let total = issues.len();
    let items = issue_items(issues);

    let list = StyledList::new("Recently viewed")
        .items(items)
        .emphasis(emphasis)
        .state(state)
        .position(selected, total);

    let list = match total {
        0 => list.placeholder(Line::from("Issues you open land here")),
        _ => list,
    };

    frame.render_widget(list, area);
}

pub fn render_preview(frame: &mut Frame, area: Rect, issue: Option<&IssueSummary>) {
    let (title, text) = match issue {
        Some(issue) => (issue.identifier.clone(), preview_text(issue)),
        None => (
            "Recently viewed".to_string(),
            Text::from("Open an issue and it shows up here"),
        ),
    };

    text_panel(frame, area, &title, text, Emphasis::Blurred);
}
