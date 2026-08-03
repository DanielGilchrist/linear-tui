use ratatui::{
    text::{Line, Span},
    widgets::ListItem,
};

use super::super::theme;
use super::issue_row::issue_row;
use crate::api::{IssueSummary, NotificationItem, Timestamp};
use crate::tui::display::{self, GroupBy};
use crate::tui::spinner::Spinner;

pub fn view_items(
    issues: &[IssueSummary],
    groups: &[display::Group],
    group_by: GroupBy,
    selected: Option<usize>,
    id_width: usize,
    width: usize,
    now: Timestamp,
) -> (Vec<ListItem<'static>>, Option<usize>) {
    let mut items: Vec<ListItem> = Vec::new();
    let mut flat = 0usize;
    let mut selected_row = None;

    for group in groups {
        if let Some(label) = &group.label {
            items.push(ListItem::new(Line::from(vec![
                Span::styled(label.clone(), theme::group_header()),
                Span::styled(format!("  {}", group.indices.len()), theme::dim()),
            ])));
        }

        for &index in &group.indices {
            if Some(flat) == selected {
                selected_row = Some(items.len());
            }

            items.push(ListItem::new(issue_row(
                &issues[index],
                group_by,
                id_width,
                width,
                now,
            )));

            flat += 1;
        }
    }

    (items, selected_row)
}

pub fn breakdown_line(groups: &[display::Group]) -> Line<'static> {
    let mut spans: Vec<Span> = Vec::new();

    for group in groups {
        if let Some(label) = &group.label {
            if !spans.is_empty() {
                spans.push(Span::styled("  ·  ", theme::dim()));
            }

            spans.push(Span::styled(
                format!("{} ", group.indices.len()),
                theme::accent(),
            ));

            spans.push(Span::styled(label.clone(), theme::dim()));
        }
    }

    Line::from(spans)
}

pub fn loading_more_row(spinner: Spinner) -> ListItem<'static> {
    ListItem::new(Line::from(Span::styled(
        format!("{spinner}  loading more…"),
        theme::dim(),
    )))
}

pub fn notification_items(notifications: &[NotificationItem]) -> Vec<ListItem<'static>> {
    notifications
        .iter()
        .map(|notification| {
            let indicator = if notification.is_read {
                Span::raw("  ")
            } else {
                Span::styled("● ", theme::person())
            };
            let title_style = if notification.is_read {
                theme::dim()
            } else {
                theme::TITLE
            };
            ListItem::new(Line::from(vec![
                indicator,
                Span::styled(notification.title.clone(), title_style),
            ]))
        })
        .collect()
}
