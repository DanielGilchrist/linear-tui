use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem},
    Frame,
};

use super::super::theme;
use crate::tui::layout;
use crate::tui::overlay::{WorkspaceRow, Workspaces};

pub fn area(frame_area: Rect) -> Rect {
    layout::centred_rect_fixed(frame_area, 52, 12)
}

pub fn render(workspaces: &mut Workspaces, frame: &mut Frame, area: Rect) {
    let items: Vec<ListItem> = workspaces.rows.iter().map(row_item).collect();

    let list = List::new(items)
        .block(
            Block::bordered()
                .title("Workspaces")
                .border_style(theme::ACCENT),
        )
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED));

    frame.render_stateful_widget(list, area, &mut workspaces.state);
}

fn row_item(row: &WorkspaceRow) -> ListItem<'static> {
    let line = match row {
        WorkspaceRow::Account {
            name,
            detail,
            active,
            ..
        } => Line::from(vec![
            Span::styled(if *active { "● " } else { "  " }, theme::ACCENT),
            Span::styled(name.clone(), theme::TEXT),
            Span::raw("  "),
            Span::styled(detail.clone(), theme::DIM),
        ]),
        WorkspaceRow::AddBrowser => {
            Line::from(Span::styled("+ Sign in with browser", theme::PERSON))
        }
        WorkspaceRow::AddKey => Line::from(Span::styled("+ Add with an API key", theme::PERSON)),
        WorkspaceRow::AddEnvVar => Line::from(Span::styled(
            "+ Add from an environment variable",
            theme::PERSON,
        )),
    };

    ListItem::new(line)
}
