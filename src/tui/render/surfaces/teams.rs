use ratatui::{
    layout::Rect,
    text::{Line, Span, Text},
    widgets::ListItem,
    Frame,
};

use super::super::theme::{self, Emphasis};
use super::super::widgets::{placeholder, text_panel, PlaceholderText, StyledList};
use crate::tui::cache::CacheStatus;
use crate::tui::feed::{FeedKey, FeedStore};
use crate::tui::spinner::Spinner;
use crate::tui::team::TeamModes;
use crate::tui::workspace::TeamsPanel;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    panel: &mut TeamsPanel,
    emphasis: Emphasis,
    spinner: Spinner,
) {
    let selected = panel.state.selected();
    let total = panel.list().len();
    let status = panel.teams.status();

    let items: Vec<ListItem> = panel
        .list()
        .iter()
        .map(|team| ListItem::new(Line::from(team.name.clone())))
        .collect();

    let list = StyledList::new("Teams")
        .items(items)
        .emphasis(emphasis)
        .state(&mut panel.state)
        .position(selected, total);

    let list = match total {
        0 => list.placeholder(empty_line(status, spinner)),
        _ => list,
    };

    frame.render_widget(list, area);
}

fn empty_line(status: CacheStatus, spinner: Spinner) -> Line<'static> {
    placeholder(
        Some(status),
        PlaceholderText {
            empty: "No teams",
            loading: super::LOADING_TEXT,
            failed: super::LOAD_FAILED_TEXT,
        },
        spinner,
    )
}

pub fn render_preview(
    frame: &mut Frame,
    area: Rect,
    panel: &TeamsPanel,
    feeds: &FeedStore,
    spinner: Spinner,
) {
    let Some(team) = panel.selected() else {
        let empty = empty_line(panel.teams.status(), spinner);

        text_panel(frame, area, "Teams", Text::from(empty), Emphasis::Blurred);

        return;
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled(team.key.clone(), theme::ACCENT),
            Span::raw("  "),
            Span::styled(team.name.clone(), theme::TEXT),
        ]),
        Line::from(""),
        Line::from(Span::styled("modes", theme::MUTED)),
    ];

    for mode in TeamModes::for_team(team).as_slice() {
        let cached = feeds
            .get(&FeedKey::Issues(mode.filter(&team.id)))
            .filter(|feed| !feed.items().is_empty())
            .map(|feed| feed.items().len());

        lines.push(Line::from(vec![
            Span::styled(format!("  {:<9}", mode.label()), theme::TEXT),
            match cached {
                Some(count) => Span::styled(count.to_string(), theme::DIM),
                None => Span::styled("–", theme::DIM),
            },
        ]));
    }

    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled(
        "enter to browse   ] [ to switch mode",
        theme::DIM,
    )));

    text_panel(
        frame,
        area,
        &team.name,
        Text::from(lines),
        Emphasis::Blurred,
    );
}
