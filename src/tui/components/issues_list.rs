use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{ListItem, ListState},
    Frame,
};

use super::styled_list::StyledList;
use crate::api::team_issues;

pub struct IssuesList<'a> {
    issues: &'a [team_issues::Issue],
    state: &'a mut ListState,
    focused: bool,
    show_placeholder: bool,
}

impl<'a> IssuesList<'a> {
    pub fn new(issues: &'a [team_issues::Issue], state: &'a mut ListState) -> Self {
        Self {
            issues,
            state,
            focused: false,
            show_placeholder: issues.is_empty(),
        }
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn show_placeholder(mut self, show: bool) -> Self {
        self.show_placeholder = show;
        self
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .issues
            .iter()
            .map(|issue| {
                let (priority_icon, priority_color) = priority_indicator(issue.priority as u8);
                let state_color = state_type_color(&issue.state.state_type);

                let mut spans = vec![
                    Span::styled(priority_icon, Style::default().fg(priority_color)),
                    Span::raw(" "),
                    Span::styled(&issue.identifier, Style::default().fg(Color::DarkGray)),
                    Span::raw(" "),
                    Span::styled(&issue.state.name, Style::default().fg(state_color)),
                    Span::raw(" "),
                    Span::styled(
                        issue.title.as_deref().unwrap_or("Untitled"),
                        Style::default().fg(Color::White),
                    ),
                ];

                if let Some(assignee) = &issue.assignee {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(
                        &assignee.display_name,
                        Style::default().fg(Color::Blue),
                    ));
                }

                let labels = &issue.labels.nodes;
                if !labels.is_empty() {
                    spans.push(Span::raw(" "));
                    for (i, label) in labels.iter().enumerate() {
                        if i > 0 {
                            spans.push(Span::raw(" "));
                        }
                        let label_color = parse_hex_color(&label.color);
                        spans.push(Span::styled(
                            format!(" {} ", &label.name),
                            Style::default().fg(Color::Black).bg(label_color),
                        ));
                    }
                }

                ListItem::new(Line::from(spans))
            })
            .collect();

        let selected = self.state.selected();
        let total = self.issues.len();

        let mut list = StyledList::new("Issues")
            .items(items)
            .focused(self.focused)
            .state(self.state)
            .position(selected, total);

        if self.show_placeholder {
            list = list.placeholder("Select a team to view issues");
        }

        list.render(frame, area);
    }
}

fn priority_indicator(priority: u8) -> (&'static str, Color) {
    match priority {
        1 => ("!!!", Color::Red),
        2 => ("!! ", Color::LightRed),
        3 => ("!  ", Color::Yellow),
        4 => ("-  ", Color::Blue),
        _ => ("   ", Color::DarkGray),
    }
}

fn state_type_color(state_type: &str) -> Color {
    match state_type {
        "started" => Color::Yellow,
        "completed" => Color::Green,
        "canceled" => Color::Red,
        "triage" => Color::Magenta,
        "backlog" => Color::DarkGray,
        "unstarted" => Color::Gray,
        _ => Color::Gray,
    }
}

fn parse_hex_color(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');

    if hex.len() != 6 {
        return Color::Gray;
    }

    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(128);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(128);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(128);

    Color::Rgb(r, g, b)
}
