use ratatui::text::{Line, Span};

use super::super::theme;
use crate::tui::cache::CacheStatus;
use crate::tui::spinner::Spinner;

pub struct PlaceholderText {
    pub empty: &'static str,
    pub loading: &'static str,
    pub failed: &'static str,
}

pub fn placeholder(
    status: Option<&CacheStatus>,
    text: PlaceholderText,
    spinner: Spinner,
) -> Line<'static> {
    match status {
        Some(CacheStatus::Failed(_)) => {
            Line::from(Span::styled(text.failed.to_string(), theme::ERROR))
        }
        Some(CacheStatus::Ready) => Line::from(Span::styled(text.empty.to_string(), theme::DIM)),
        None | Some(CacheStatus::Idle | CacheStatus::Loading | CacheStatus::Revalidating) => {
            Line::from(Span::styled(
                format!("{spinner}  {}", text.loading),
                theme::DIM,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts() -> PlaceholderText {
        PlaceholderText {
            empty: "nothing here",
            loading: "Loading…",
            failed: "failed  ·  r to retry",
        }
    }

    fn content(line: &Line) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    #[test]
    fn failed_shows_the_failed_text_in_error_style() {
        let line = placeholder(
            Some(&CacheStatus::Failed("boom".into())),
            texts(),
            Spinner::default(),
        );
        assert_eq!(content(&line), "failed  ·  r to retry");
        assert_eq!(line.spans[0].style, theme::ERROR);
    }

    #[test]
    fn ready_but_empty_shows_the_empty_text() {
        let line = placeholder(Some(&CacheStatus::Ready), texts(), Spinner::default());
        assert_eq!(content(&line), "nothing here");
        assert_eq!(line.spans[0].style, theme::DIM);
    }

    #[test]
    fn loading_and_absent_show_the_spinner_and_loading_text() {
        for status in [
            None,
            Some(CacheStatus::Loading),
            Some(CacheStatus::Revalidating),
        ] {
            let line = placeholder(status.as_ref(), texts(), Spinner::default());
            assert!(
                content(&line).ends_with("Loading…"),
                "expected loading text, got {}",
                content(&line)
            );
        }
    }
}
