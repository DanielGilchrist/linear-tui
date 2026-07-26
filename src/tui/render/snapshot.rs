use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};

use super::render;
use crate::tui::app::App;

pub fn render_to_string(app: &mut App, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test backend");
    terminal
        .draw(|frame| render(app, frame))
        .expect("draw to test backend");
    buffer_to_string(terminal.backend().buffer())
}

fn buffer_to_string(buffer: &Buffer) -> String {
    let area = buffer.area;
    let mut out = String::new();
    for y in 0..area.height {
        let mut line = String::new();
        for x in 0..area.width {
            line.push_str(buffer[(x, y)].symbol());
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

pub fn render_styled_to_string(app: &mut App, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test backend");

    terminal
        .draw(|frame| render(app, frame))
        .expect("draw to test backend");

    buffer_to_styled_string(terminal.backend().buffer())
}

fn buffer_to_styled_string(buffer: &Buffer) -> String {
    let area = buffer.area;
    let default_key = cell_style_key(&ratatui::buffer::Cell::default());
    let mut out = String::new();

    for y in 0..area.height {
        let mut symbols = String::new();
        let mut runs: Vec<String> = Vec::new();
        let mut start = 0usize;
        let mut key = cell_style_key(&buffer[(0, y)]);
        let mut text = String::new();

        let flush = |start: usize, key: &str, text: &str, runs: &mut Vec<String>| {
            let blank = text.trim().is_empty() && key == default_key;
            if !blank {
                runs.push(format!("[{start}] {key} {text:?}"));
            }
        };

        for x in 0..area.width {
            let cell = &buffer[(x, y)];
            symbols.push_str(cell.symbol());

            let cell_key = cell_style_key(cell);
            if cell_key == key {
                text.push_str(cell.symbol());
            } else {
                flush(start, &key, &text, &mut runs);
                key = cell_key;
                start = x as usize;
                text = cell.symbol().to_string();
            }
        }

        flush(start, &key, &text, &mut runs);

        out.push_str(symbols.trim_end());
        out.push('\n');

        for run in runs {
            out.push_str("    ");
            out.push_str(&run);
            out.push('\n');
        }
    }

    out
}

fn cell_style_key(cell: &ratatui::buffer::Cell) -> String {
    let style = cell.style();
    format!(
        "fg={:?} bg={:?} mod={:?}",
        style.fg, style.bg, style.add_modifier
    )
}
