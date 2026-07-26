use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

use super::super::theme;
use crate::tui::layout;
use crate::tui::overlay::{ReactionChoice, Reactions, Section};

const LABEL_WIDTH: u16 = 9;
const CELL_WIDTH: u16 = 5;
const HINT: &str = "h/l move · j/k row · enter toggle · c custom · esc";

pub fn area(reactions: &Reactions, frame_area: Rect) -> Rect {
    let current = reactions
        .choices
        .iter()
        .filter(|choice| choice.in_section(Section::Current))
        .count();
    let add = reactions.choices.len() - current;

    let cols = current.max(add).max(1) as u16;
    let content = LABEL_WIDTH + cols * CELL_WIDTH;
    let width = content.max(HINT.chars().count() as u16) + 2;
    let section_rows = if current > 0 { 2 } else { 1 };
    let height = section_rows + 3;

    layout::centred_box(frame_area, width, height)
}

pub fn render(reactions: &Reactions, frame: &mut Frame, area: Rect) {
    let block = Block::bordered().title("React").border_style(theme::ACCENT);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let has_current = reactions
        .choices
        .iter()
        .any(|choice| choice.in_section(Section::Current));

    let mut constraints = Vec::new();
    if has_current {
        constraints.push(Constraint::Length(1));
    }
    constraints.push(Constraint::Length(1));
    constraints.push(Constraint::Length(1));
    constraints.push(Constraint::Length(1));

    let rows = Layout::vertical(constraints).split(inner);

    let mut next = 0;
    if has_current {
        render_section(frame, rows[next], "Current:", reactions, Section::Current);
        next += 1;
    }
    render_section(frame, rows[next], "Add:", reactions, Section::Add);

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(HINT, theme::DIM))),
        rows[rows.len() - 1],
    );
}

fn render_section(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    reactions: &Reactions,
    section: Section,
) {
    let items: Vec<(usize, &ReactionChoice)> = reactions
        .choices
        .iter()
        .enumerate()
        .filter(|(_, choice)| choice.in_section(section))
        .collect();

    let selected = reactions.state.selected();

    let constraints = std::iter::once(Constraint::Length(LABEL_WIDTH)).chain(std::iter::repeat_n(
        Constraint::Length(CELL_WIDTH),
        items.len(),
    ));
    let cells = Layout::horizontal(constraints).split(area);

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(label.to_string(), theme::DIM))),
        cells[0],
    );

    for (slot, (flat, choice)) in items.iter().enumerate() {
        let text = match section {
            Section::Current => format!("{} {}", choice.glyph, choice.count),
            Section::Add => choice.glyph.clone(),
        };

        let mut style = if choice.mine {
            theme::REACTION_MINE
        } else {
            theme::TEXT
        };
        if Some(*flat) == selected {
            style = style.add_modifier(Modifier::REVERSED);
        }

        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(text, style))),
            cells[slot + 1],
        );
    }
}
