use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Clear, ListItem, ListState, Paragraph},
    Frame,
};

use super::super::format;
use super::super::theme::{self, Emphasis};
use super::super::widgets::StyledList;
use crate::tui::layout;
use crate::tui::overlay::{Cell, Editor, MentionMenu};

pub fn area(frame_area: Rect) -> Rect {
    layout::centred_rect(frame_area, 70, 50)
}

pub fn render(editor: &Editor, frame: &mut Frame, area: Rect) {
    let block = Block::bordered()
        .title(editor.title)
        .border_style(theme::ACCENT);

    let inner_height = (area.height.saturating_sub(2) as usize).max(1);
    let offset = editor.row.saturating_sub(inner_height - 1);

    let lines: Vec<Line> = editor
        .lines
        .iter()
        .enumerate()
        .skip(offset)
        .take(inner_height)
        .map(|(row, cells)| {
            let cursor = (row == editor.row).then_some(editor.col);
            editor_line(cells, cursor)
        })
        .collect();

    let inner_width = area.width.saturating_sub(2);
    let cursor_column = editor
        .lines
        .get(editor.row)
        .map_or(1, |cells| cursor_column(cells, editor.col));

    let scroll_x = cursor_column.saturating_sub(inner_width.saturating_sub(1));
    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .scroll((0, scroll_x));

    frame.render_widget(paragraph, area);

    if let Some(mention) = &editor.mention {
        render_mention_popup(editor, mention, frame, area);
    }
}

fn cell_width(cell: &Cell) -> u16 {
    let rendered = match cell {
        Cell::Char(c) => c.to_string(),
        Cell::Mention(mention) => format!("@{}", mention.display),
    };

    format::width(&rendered) as u16
}

fn cursor_column(cells: &[Cell], col: usize) -> u16 {
    1 + cells.iter().take(col).map(cell_width).sum::<u16>()
}

fn editor_line(cells: &[Cell], cursor: Option<usize>) -> Line<'static> {
    let mut spans = vec![Span::raw(" ".to_string())];

    for (index, cell) in cells.iter().enumerate() {
        let mut span = match cell {
            Cell::Char(c) => Span::raw(c.to_string()),
            Cell::Mention(mention) => Span::styled(format!("@{}", mention.display), theme::PERSON),
        };

        if cursor == Some(index) {
            span.style = span.style.add_modifier(Modifier::REVERSED);
        }

        spans.push(span);
    }

    if cursor == Some(cells.len()) {
        spans.push(Span::styled(
            " ".to_string(),
            Style::default().add_modifier(Modifier::REVERSED),
        ));
    }

    Line::from(spans)
}

fn render_mention_popup(
    editor: &Editor,
    mention: &MentionMenu,
    frame: &mut Frame,
    editor_area: Rect,
) {
    let candidates = editor.candidates(&mention.query);

    if candidates.is_empty() {
        return;
    }

    let selected = mention.state.selected();

    let visible = candidates.len().min(6) as u16;
    let width = editor_area.width.saturating_sub(4).min(40);
    let height = visible + 2;
    let area = Rect {
        x: editor_area.x + 2,
        y: (editor_area.y + editor_area.height).saturating_sub(height + 1),
        width,
        height,
    };

    let items: Vec<ListItem> = candidates
        .iter()
        .map(|user| {
            ListItem::new(Line::from(Span::styled(
                format!("@{}", user.display_name),
                theme::PERSON,
            )))
        })
        .collect();

    frame.render_widget(Clear, area);

    let mut state = ListState::default().with_selected(selected);

    let mentions = StyledList::new("Mention")
        .items(items)
        .emphasis(Emphasis::Focused)
        .state(&mut state);

    frame.render_widget(mentions, area);
}
