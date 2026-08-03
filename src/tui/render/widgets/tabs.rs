use ratatui::text::{Line, Span};

use super::super::format;
use super::super::theme;
use crate::tui::spinner::Spinner;
use crate::tui::view::View;

const SEPARATOR: &str = " · ";

pub fn view_tabs(
    views: &[View],
    active: usize,
    loading: bool,
    spinner: Spinner,
    max_width: usize,
) -> Line<'static> {
    let spinner_width = if loading {
        2 + format::width(spinner.glyph())
    } else {
        0
    };

    let mut spans = if full_strip_width(views) + spinner_width <= max_width {
        full_strip(views, active)
    } else {
        compact_strip(views, active, max_width.saturating_sub(spinner_width))
    };

    if loading {
        spans.push(Span::styled(format!("  {spinner}"), theme::accent()));
    }

    Line::from(spans)
}

fn full_strip_width(views: &[View]) -> usize {
    views.iter().map(|v| format::width(&v.name)).sum::<usize>()
        + format::width(SEPARATOR) * views.len().saturating_sub(1)
}

fn full_strip(views: &[View], active: usize) -> Vec<Span<'static>> {
    let mut spans: Vec<Span> = Vec::new();

    for (index, view) in views.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(SEPARATOR, theme::DIM));
        }

        let style = if index == active {
            theme::TITLE
        } else {
            theme::DIM
        };
        spans.push(Span::styled(view.name.clone(), style));
    }

    spans
}

fn compact_strip(views: &[View], active: usize, budget: usize) -> Vec<Span<'static>> {
    let indicator = format!(" {}/{}", active + 1, views.len());
    let name_budget = budget.saturating_sub(format::width(&indicator));

    vec![
        Span::styled(format::fit(&views[active].name, name_budget), theme::TITLE),
        Span::styled(indicator, theme::DIM),
    ]
}
