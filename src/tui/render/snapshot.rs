use std::num::NonZeroUsize;

use ratatui::{
    backend::TestBackend,
    buffer::{Buffer, Cell},
    Terminal,
};

use super::render;
use crate::tui::app::App;

pub fn render_to_string(app: &mut App, width: u16, height: u16) -> String {
    let Ok(mut terminal) = Terminal::new(TestBackend::new(width, height));
    let Ok(_) = terminal.draw(|frame| render(app, frame));

    buffer_to_string(terminal.backend().buffer())
}

pub fn render_styled_to_string(app: &mut App, width: u16, height: u16) -> String {
    let Ok(mut terminal) = Terminal::new(TestBackend::new(width, height));
    let Ok(_) = terminal.draw(|frame| render(app, frame));

    buffer_to_styled_string(terminal.backend().buffer())
}

fn rows(buffer: &Buffer) -> impl Iterator<Item = &[Cell]> {
    let width = NonZeroUsize::new(usize::from(buffer.area.width));

    width
        .into_iter()
        .flat_map(|width| buffer.content.chunks(width.get()))
}

fn buffer_to_string(buffer: &Buffer) -> String {
    let mut out = String::new();

    for row in rows(buffer) {
        let line: String = row.iter().map(Cell::symbol).collect();

        out.push_str(line.trim_end());
        out.push('\n');
    }

    out
}

struct Run {
    start: usize,
    key: String,
    text: String,
}

fn buffer_to_styled_string(buffer: &Buffer) -> String {
    let default_key = cell_style_key(&Cell::default());
    let mut out = String::new();

    for row in rows(buffer) {
        let symbols: String = row.iter().map(Cell::symbol).collect();
        let mut runs: Vec<Run> = Vec::new();

        for (x, cell) in row.iter().enumerate() {
            let key = cell_style_key(cell);

            match runs.last_mut() {
                Some(run) if run.key == key => run.text.push_str(cell.symbol()),
                _ => runs.push(Run {
                    start: x,
                    key,
                    text: cell.symbol().to_string(),
                }),
            }
        }

        out.push_str(symbols.trim_end());
        out.push('\n');

        for Run { start, key, text } in runs {
            if text.trim().is_empty() && key == default_key {
                continue;
            }

            out.push_str(&format!("    [{start}] {key} {text:?}\n"));
        }
    }

    out
}

fn cell_style_key(cell: &Cell) -> String {
    let style = cell.style();

    format!(
        "fg={:?} bg={:?} mod={:?}",
        style.fg, style.bg, style.add_modifier
    )
}
