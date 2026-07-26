use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::super::format;
use super::super::theme;
use crate::tui::layout;

pub enum Footer {
    Find(FindBar),
    Normal { left: FooterLeft, workspace: String },
}

pub enum FindBar {
    Typing {
        query: String,
        total: usize,
    },
    NoMatches {
        query: String,
    },
    Matches {
        query: String,
        position: usize,
        total: usize,
    },
}

pub enum FooterLeft {
    Status { text: String, is_error: bool },
    Hint { text: String },
}

pub fn render(frame: &mut Frame, area: Rect, footer: Footer) {
    match footer {
        Footer::Find(find) => {
            frame.render_widget(Paragraph::new(find_bar(find)), area);
        }
        Footer::Normal { left, workspace } => {
            let [left_area, right_area] =
                layout::split_footer(area, format::width(&workspace) as u16 + 1);

            frame.render_widget(
                Paragraph::new(left_line(left, left_area.width as usize)),
                left_area,
            );

            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(workspace, theme::WORKSPACE)))
                    .alignment(Alignment::Right),
                right_area,
            );
        }
    }
}

fn left_line(left: FooterLeft, width: usize) -> Line<'static> {
    match left {
        FooterLeft::Status { text, is_error } => {
            let style = if is_error {
                theme::ERROR
            } else {
                theme::ACCENT
            };
            Line::from(Span::styled(format::fit(&format!(" {text}"), width), style))
        }
        FooterLeft::Hint { text } => Line::from(Span::styled(
            format::fit(&format!(" {text}"), width),
            theme::DIM,
        )),
    }
}

fn find_bar(find: FindBar) -> Line<'static> {
    let label = Span::styled(" Search ", theme::FIND_LABEL);

    match find {
        FindBar::Typing { query, total } => Line::from(vec![
            label,
            Span::styled(format!(" {query}"), theme::TEXT),
            Span::styled(" ", Style::default().add_modifier(Modifier::REVERSED)),
            Span::styled(
                format!("   {total} matches   enter select   esc cancel"),
                theme::DIM,
            ),
        ]),
        FindBar::NoMatches { query } => Line::from(vec![
            label,
            Span::styled(format!(" no matches for '{query}'"), theme::ERROR),
            Span::styled("   esc exit", theme::DIM),
        ]),
        FindBar::Matches {
            query,
            position,
            total,
        } => Line::from(vec![
            label,
            Span::styled(format!(" '{query}'  {position} of {total}"), theme::TEXT),
            Span::styled("   n next   N prev   esc exit", theme::DIM),
        ]),
    }
}
