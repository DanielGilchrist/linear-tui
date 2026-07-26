use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

pub fn cursor_line(text: &str, col: usize) -> Line<'static> {
    let chars: Vec<char> = text.chars().collect();
    let cursor = col.min(chars.len());
    let before: String = chars[..cursor].iter().collect();
    let under = chars.get(cursor).copied().unwrap_or(' ').to_string();
    let after: String = chars
        .get(cursor + 1..)
        .map(|rest| rest.iter().collect())
        .unwrap_or_default();

    Line::from(vec![
        Span::raw(format!(" {before}")),
        Span::styled(under, Style::default().add_modifier(Modifier::REVERSED)),
        Span::raw(after),
    ])
}
