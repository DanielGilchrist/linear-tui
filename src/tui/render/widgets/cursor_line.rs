use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

pub fn cursor_line(text: &str, col: usize) -> Line<'static> {
    let before: String = text.chars().take(col).collect();
    let mut rest = text.chars().skip(col);
    let under = rest.next().unwrap_or(' ').to_string();
    let after: String = rest.collect();

    Line::from(vec![
        Span::raw(format!(" {before}")),
        Span::styled(under, Style::default().add_modifier(Modifier::REVERSED)),
        Span::raw(after),
    ])
}
