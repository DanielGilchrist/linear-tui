use ratatui::{
    layout::Rect,
    style::Modifier,
    text::{Line, Span, Text},
    widgets::ScrollbarState,
    Frame,
};

use super::super::theme::{self, Emphasis};
use super::super::widgets::{
    notification_preview_text, preview_text, reaction_chips, text_panel, ScrollableText,
};
use crate::api::{IssueDetail, IssueSummary, NotificationItem, ThreadedComment, Timestamp};
use crate::tui::cache::{Phase, Remote};
use crate::tui::spinner::Spinner;
use crate::tui::workspace::RenderedDetail;

pub enum Preview<'a> {
    Issue(Option<&'a IssueSummary>),
    Notification(Option<&'a NotificationItem>),
}

pub struct ReadingProps<'a> {
    pub now: Timestamp,
    pub selected: Option<usize>,
    pub scroll_position: &'a mut usize,
    pub scroll_state: &'a mut ScrollbarState,
    pub emphasis: Emphasis,
}

pub fn render_reading(
    frame: &mut Frame,
    area: Rect,
    detail: &IssueDetail,
    rendered: &RenderedDetail,
    props: ReadingProps,
) {
    let ReadingProps {
        now,
        selected,
        scroll_position,
        scroll_state,
        emphasis,
    } = props;

    let body = detail_text(detail, rendered, now, selected);
    let title = detail.identifier.clone();

    if let Some(start) = selected.and_then(|index| body.comment_top(index)) {
        *scroll_position = start;
    }

    frame.render_widget(
        ScrollableText::new(body.text, scroll_position, scroll_state)
            .title(&title)
            .border_style(emphasis.border()),
        area,
    );
}

pub fn render_pane(
    frame: &mut Frame,
    area: Rect,
    detail: &Remote<IssueDetail>,
    rendered: &RenderedDetail,
    spinner: Spinner,
    preview: Preview,
    props: ReadingProps,
) {
    match detail.phase() {
        Phase::Ready => {
            if let Some(detail) = detail.value() {
                render_reading(frame, area, detail, rendered, props);
            }
        }
        Phase::Loading => text_panel(
            frame,
            area,
            "Issue",
            Text::from(format!("{spinner}  Loading issue…")),
            props.emphasis,
        ),
        Phase::Missing | Phase::Failed => render_work_preview(frame, area, preview, props.emphasis),
    }
}

pub fn render_work_preview(frame: &mut Frame, area: Rect, preview: Preview, emphasis: Emphasis) {
    let (title, text) = match preview {
        Preview::Issue(Some(issue)) => (issue.identifier.clone(), preview_text(issue)),
        Preview::Issue(None) => ("Preview".to_string(), Text::from("No issue selected")),
        Preview::Notification(Some(notification)) => (
            "Notification".to_string(),
            notification_preview_text(notification),
        ),
        Preview::Notification(None) => ("Notification".to_string(), Text::from("Nothing selected")),
    };

    text_panel(frame, area, &title, text, emphasis);
}

struct DetailBody {
    text: Text<'static>,
    comment_offsets: Vec<usize>,
}

impl DetailBody {
    fn comment_top(&self, index: usize) -> Option<usize> {
        self.comment_offsets.get(index).copied()
    }
}

fn detail_text(
    detail: &IssueDetail,
    rendered: &RenderedDetail,
    now: Timestamp,
    selected: Option<usize>,
) -> DetailBody {
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![
        Span::styled(detail.identifier.clone(), theme::DIM),
        Span::raw("  "),
        Span::styled(
            detail.state.name.clone(),
            theme::state(detail.state.state_type),
        ),
    ]));
    lines.push(Line::from(Span::styled(
        detail.title.clone().unwrap_or_else(|| "Untitled".into()),
        theme::TITLE,
    )));

    let mut meta: Vec<Span> = Vec::new();

    if let Some(assignee) = &detail.assignee {
        meta.push(Span::styled(
            format!("@{}", assignee.display_name),
            theme::PERSON,
        ));
    }

    for label in &detail.labels {
        meta.push(Span::raw(" "));
        meta.push(Span::styled(
            format!(" {} ", label.name),
            theme::label_chip(label.colour),
        ));
    }

    if !meta.is_empty() {
        lines.push(Line::from(meta));
    }

    lines.push(Line::from(Span::styled(detail.url.clone(), theme::DIM)));
    lines.push(Line::from(""));

    if !rendered.description.is_empty() {
        lines.extend(rendered.description.iter().cloned());
        lines.push(Line::from(""));
    }

    if let Some(chips) = reaction_chips(&detail.reactions) {
        lines.push(chips);
        lines.push(Line::from(""));
    }

    let mut comment_offsets = Vec::new();

    if !detail.comments.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("Comments ({})", detail.comments.len()),
            theme::ACCENT,
        )));

        lines.push(Line::from(""));

        let threaded = detail.threaded_comments();

        for (index, (threaded, body)) in threaded
            .into_iter()
            .zip(&rendered.comment_bodies)
            .enumerate()
        {
            comment_offsets.push(lines.len());
            append_comment(&mut lines, threaded, body, selected == Some(index), now);
        }
    }

    DetailBody {
        text: Text::from(lines),
        comment_offsets,
    }
}

fn append_comment(
    lines: &mut Vec<Line<'static>>,
    threaded: ThreadedComment,
    body: &[Line<'static>],
    highlighted: bool,
    now: Timestamp,
) {
    let ThreadedComment { comment, depth } = threaded;
    let indent = "  ".repeat(depth);
    let body_indent = "  ".repeat(depth + 1);

    let mut header: Vec<Span<'static>> = Vec::new();

    if depth > 0 {
        header.push(Span::styled(format!("{indent}└ "), theme::DIM));
    }

    header.push(Span::styled(
        comment.author.clone().unwrap_or_else(|| "unknown".into()),
        theme::COMMENT_AUTHOR,
    ));

    header.push(Span::styled(
        format!(" · {}", comment.created_at.humanise(now)),
        theme::DIM,
    ));

    if highlighted {
        for span in &mut header {
            span.style = span.style.add_modifier(Modifier::REVERSED);
        }
    }

    lines.push(Line::from(header));

    for line in body {
        let mut spans = vec![Span::raw(body_indent.clone())];
        spans.extend(line.spans.iter().cloned());

        lines.push(Line::from(spans));
    }

    if let Some(chips) = reaction_chips(&comment.reactions) {
        let mut spans = vec![Span::raw(body_indent.clone())];

        spans.extend(chips.spans);
        lines.push(Line::from(spans));
    }

    lines.push(Line::from(""));
}
