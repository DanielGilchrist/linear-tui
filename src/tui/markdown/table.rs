use pulldown_cmark::Alignment;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use super::style::dim_style;

const SEPARATOR: &str = " │ ";

type Cell = Vec<Span<'static>>;

pub(super) struct Table {
    aligns: Vec<Alignment>,
    header: Vec<Cell>,
    body: Vec<Vec<Cell>>,
    row: Vec<Cell>,
}

impl Table {
    pub(super) fn new(aligns: Vec<Alignment>) -> Self {
        Self {
            aligns,
            header: Vec::new(),
            body: Vec::new(),
            row: Vec::new(),
        }
    }

    pub(super) fn push_cell(&mut self, cell: Vec<Span<'static>>) {
        self.row.push(cell);
    }

    pub(super) fn finish_header(&mut self) {
        self.header = std::mem::take(&mut self.row);
    }

    pub(super) fn finish_row(&mut self) {
        self.body.push(std::mem::take(&mut self.row));
    }

    pub(super) fn render(self, base: Style) -> Vec<Line<'static>> {
        let cols = self
            .header
            .len()
            .max(self.body.iter().map(Vec::len).max().unwrap_or(0));

        if cols == 0 {
            return Vec::new();
        }

        let mut widths = vec![0usize; cols];
        for row in std::iter::once(&self.header).chain(self.body.iter()) {
            for (index, cell) in row.iter().enumerate() {
                widths[index] = widths[index].max(cell_width(cell));
            }
        }

        let mut lines = Vec::new();

        if !self.header.is_empty() {
            lines.push(self.row_line(&self.header, &widths, base, true));

            let rule =
                widths.iter().sum::<usize>() + SEPARATOR.chars().count() * cols.saturating_sub(1);
            lines.push(Line::from(Span::styled("─".repeat(rule), dim_style(base))));
        }

        for row in &self.body {
            lines.push(self.row_line(row, &widths, base, false));
        }

        lines
    }

    fn row_line(
        &self,
        cells: &[Cell],
        widths: &[usize],
        base: Style,
        header: bool,
    ) -> Line<'static> {
        let mut spans: Vec<Span<'static>> = Vec::new();

        for (index, width) in widths.iter().enumerate() {
            if index > 0 {
                spans.push(Span::styled(SEPARATOR.to_string(), dim_style(base)));
            }

            let cell = cells.get(index);
            let pad = width.saturating_sub(cell.map_or(0, |cell| cell_width(cell)));
            let align = self.aligns.get(index).copied().unwrap_or(Alignment::None);
            let last = index + 1 == widths.len();

            let (left, right) = match align {
                Alignment::Right => (pad, 0),
                Alignment::Center => (pad / 2, pad - pad / 2),
                Alignment::Left | Alignment::None => (0, pad),
            };

            if left > 0 {
                spans.push(Span::raw(" ".repeat(left)));
            }
            if let Some(cell) = cell {
                for span in cell {
                    let mut span = span.clone();
                    if header {
                        span.style = span.style.add_modifier(Modifier::BOLD);
                    }
                    spans.push(span);
                }
            }
            if right > 0 && !last {
                spans.push(Span::raw(" ".repeat(right)));
            }
        }

        Line::from(spans)
    }
}

fn cell_width(cell: &[Span<'static>]) -> usize {
    cell.iter().map(|span| span.content.chars().count()).sum()
}
