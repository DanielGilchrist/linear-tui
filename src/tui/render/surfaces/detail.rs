use ratatui::{
    layout::{Rect, Size},
    style::Modifier,
    text::{Line, Span, Text},
    widgets::{Block, Clear, Paragraph},
    Frame,
};

use super::super::theme::{self, Emphasis};
use super::super::widgets::{
    max_scroll, notification_preview_text, preview_text, reaction_chips, text_area, text_panel,
    wrapped_rows, ScrollableText,
};
use crate::api::{IssueDetail, IssueSummary, NotificationItem, ThreadedComment, Timestamp};
use crate::tui::cache::{CacheStatus, Phase, Remote};
use crate::tui::focus::Scroll;
use crate::tui::markdown::{ImagePlacement, Rendered};
use crate::tui::spinner::Spinner;
use crate::tui::workspace::{ImageStore, RenderedDetail, WorkspaceData};
use ratatui_image::sliced::{SignedPosition, SlicedImage};
use std::collections::HashSet;

pub enum Preview<'a> {
    Issue(Option<&'a IssueSummary>),
    Notification(Option<&'a NotificationItem>),
}

pub struct ReadingProps<'a> {
    pub now: Timestamp,
    pub selected: Option<usize>,
    pub scroll: Scroll,
    pub emphasis: Emphasis,
    pub expanded: &'a HashSet<String>,
    pub comment_scroll: usize,
    pub spinner: Spinner,
}

#[derive(Default)]
pub struct Measured {
    pub scroll_max: usize,
    pub comment_scroll_max: usize,
}

pub fn render_reading(
    frame: &mut Frame,
    area: Rect,
    detail: &IssueDetail,
    rendered: &RenderedDetail,
    images: &mut ImageStore,
    props: ReadingProps<'_>,
) -> Measured {
    let ReadingProps {
        now,
        selected,
        scroll,
        emphasis,
        expanded,
        comment_scroll,
        spinner,
    } = props;

    let (sizing_width, sizing_height) = text_area(area);
    let sizing = Sizing {
        width: sizing_width,
        max_rows: sizing_height.saturating_sub(1).max(1) as u16,
        images,
        expanded,
    };
    let body = detail_text(detail, rendered, now, selected, Some(&sizing));
    let title = detail.identifier.clone();

    let comment_scroll_max = selected
        .and_then(|index| body.comment_rows(index))
        .map(|rows| rows.saturating_sub(sizing_height))
        .unwrap_or(0);
    let within = comment_scroll.min(comment_scroll_max);

    let scroll = match selected.and_then(|index| body.comment_top(index)) {
        Some(start) => Scroll::At(start + within),
        None => scroll,
    };

    let overlays = image_overlays(&body, area, scroll);

    let scroll_max = ScrollableText::new(body.text, scroll)
        .title(&title)
        .border_style(emphasis.border())
        .render(frame, area);

    for (placed, spot) in overlays {
        render_image(frame, spot, &placed, images, spinner);
    }

    Measured {
        scroll_max,
        comment_scroll_max,
    }
}

fn render_image(
    frame: &mut Frame,
    spot: ImageSpot,
    placed: &Placed,
    images: &mut ImageStore,
    spinner: Spinner,
) {
    frame.render_widget(Clear, spot.visible);

    let natural = Size::new(spot.visible.width, placed.rows as u16);
    let cell = images.get_or_default(&placed.image.url);
    let status = match cell.value() {
        Some(loaded) if loaded.failed_at(natural) => {
            CacheStatus::Failed("Could not render this image".to_string())
        }
        _ => cell.status(),
    };

    match cell.value_mut().and_then(|loaded| loaded.sliced(natural)) {
        Some(sliced) => {
            let position = SignedPosition::from((0, spot.offset));

            frame.render_widget(SlicedImage::new(sliced, position), spot.visible);
        }
        None => render_image_pending(frame, spot.visible, &placed.image, status, spinner),
    }
}

fn render_image_pending(
    frame: &mut Frame,
    rect: Rect,
    placement: &ImagePlacement,
    status: CacheStatus,
    spinner: Spinner,
) {
    let label = if placement.alt.is_empty() {
        "image"
    } else {
        placement.alt.as_str()
    };

    let block = Block::bordered()
        .title(Span::styled(label.to_string(), theme::dim()))
        .border_style(theme::dim());
    let inner = block.inner(rect);

    frame.render_widget(block, rect);

    let (text, style) = match status {
        CacheStatus::Failed(error) => (error, theme::error()),
        _ => (format!("{spinner}  Loading image…"), theme::dim()),
    };

    frame.render_widget(Paragraph::new(Span::styled(text, style)), inner);
}

#[derive(Clone, Copy)]
struct ImageSpot {
    visible: Rect,
    offset: i16,
}

fn image_overlays(body: &DetailBody, area: Rect, scroll: Scroll) -> Vec<(Placed, ImageSpot)> {
    let (width, height) = text_area(area);

    if width == 0 || height == 0 {
        return Vec::new();
    }

    let row = scroll.resolve(max_scroll(&body.text, area));

    body.images()
        .iter()
        .filter(|placed| placed.expanded)
        .filter_map(|placed| {
            let lines = &body.text.lines;
            let before = lines.get(..placed.top)?;
            let top = wrapped_rows(before, width);
            let rows = wrapped_rows(lines.get(placed.top..placed.top + placed.rows)?, width);

            let hidden_above = row.saturating_sub(top);

            if hidden_above >= rows {
                return None;
            }

            let visible_top = top.saturating_sub(row);
            let visible_rows = (rows - hidden_above).min(height.saturating_sub(visible_top));

            if visible_rows == 0 {
                return None;
            }

            let indent = (placed.indent as u16).saturating_mul(2);

            Some((
                placed.clone(),
                ImageSpot {
                    visible: Rect {
                        x: area.x + 1 + indent,
                        y: area.y + 1 + visible_top as u16,
                        width: width.saturating_sub(indent),
                        height: visible_rows as u16,
                    },
                    offset: -(hidden_above as i16),
                },
            ))
        })
        .collect()
}

pub fn render_pane(
    frame: &mut Frame,
    area: Rect,
    workspace: &mut WorkspaceData,
    spinner: Spinner,
    preview_of: impl FnOnce(&WorkspaceData) -> Preview<'_>,
    props: ReadingProps<'_>,
) -> Measured {
    match workspace.detail().phase() {
        Phase::Ready => {
            let (detail, rendered, images) = workspace.detail_render_parts();

            match detail.value() {
                Some(detail) => render_reading(frame, area, detail, rendered, images, props),
                None => Measured::default(),
            }
        }
        Phase::Loading => {
            text_panel(
                frame,
                area,
                "Issue",
                Text::from(format!("{spinner}  Loading issue…")),
                props.emphasis,
            );

            Measured::default()
        }
        Phase::Missing | Phase::Failed => {
            render_work_preview(frame, area, preview_of(workspace), props.emphasis);

            Measured::default()
        }
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

pub const PLACEHOLDER_ROWS: usize = 6;

#[derive(Clone)]
pub struct Placed {
    pub image: ImagePlacement,
    pub top: usize,
    pub rows: usize,
    pub indent: usize,
    pub expanded: bool,
}

pub struct DetailBody {
    text: Text<'static>,
    comment_offsets: Vec<usize>,
    images: Vec<Placed>,
}

impl DetailBody {
    fn comment_top(&self, index: usize) -> Option<usize> {
        self.comment_offsets.get(index).copied()
    }

    pub fn images(&self) -> &[Placed] {
        &self.images
    }

    fn comment_rows(&self, index: usize) -> Option<usize> {
        let start = self.comment_offsets.get(index).copied()?;
        let end = self
            .comment_offsets
            .get(index + 1)
            .copied()
            .unwrap_or(self.text.lines.len());

        Some(end.saturating_sub(start))
    }

    pub fn line_texts(&self) -> Vec<String> {
        self.text
            .lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect()
    }
}

pub struct Sizing<'a> {
    pub width: u16,
    pub max_rows: u16,
    pub images: &'a ImageStore,
    pub expanded: &'a HashSet<String>,
}

pub fn detail_text(
    detail: &IssueDetail,
    rendered: &RenderedDetail,
    now: Timestamp,
    selected: Option<usize>,
    sizing: Option<&Sizing<'_>>,
) -> DetailBody {
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![
        Span::styled(detail.identifier.clone(), theme::dim()),
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
            theme::person(),
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

    lines.push(Line::from(Span::styled(detail.url.clone(), theme::dim())));
    lines.push(Line::from(""));

    let mut images = Vec::new();

    if !rendered.description.lines.is_empty() {
        append_rendered(&mut lines, &mut images, &rendered.description, 0, sizing);
        lines.push(Line::from(""));
    }

    if let Some(chips) = reaction_chips(&detail.reactions) {
        lines.push(chips);
        lines.push(Line::from(""));
    }

    let mut comment_offsets = Vec::new();

    if !detail.comments.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("Comments ({})", detail.thread_len()),
            theme::accent(),
        )));

        lines.push(Line::from(""));

        let threaded = detail.threaded_comments();

        for (index, (threaded, body)) in threaded
            .into_iter()
            .zip(&rendered.comment_bodies)
            .enumerate()
        {
            comment_offsets.push(lines.len());
            append_comment(
                &mut lines,
                &mut images,
                threaded,
                body,
                selected == Some(index),
                now,
                sizing,
            );
        }
    }

    DetailBody {
        text: Text::from(lines),
        comment_offsets,
        images,
    }
}

fn append_rendered(
    lines: &mut Vec<Line<'static>>,
    placed: &mut Vec<Placed>,
    rendered: &Rendered,
    indent: usize,
    sizing: Option<&Sizing<'_>>,
) {
    let prefix = "  ".repeat(indent);

    for (index, line) in rendered.lines.iter().enumerate() {
        let top = lines.len();

        if prefix.is_empty() {
            lines.push(line.clone());
        } else {
            let mut spans = vec![Span::raw(prefix.clone())];
            spans.extend(line.spans.iter().cloned());
            lines.push(Line::from(spans));
        }

        let Some(image) = rendered.images.iter().find(|image| image.top == index) else {
            continue;
        };

        let expanded = sizing.is_some_and(|sizing| sizing.expanded.contains(&image.url));
        let rows = image_rows(&image.url, indent, sizing);

        for _ in 1..rows {
            lines.push(Line::from(""));
        }

        placed.push(Placed {
            image: image.clone(),
            top,
            rows,
            indent,
            expanded,
        });
    }
}

fn image_rows(url: &str, indent: usize, sizing: Option<&Sizing<'_>>) -> usize {
    let Some(sizing) = sizing.filter(|sizing| sizing.expanded.contains(url)) else {
        return 1;
    };

    let width = sizing
        .width
        .saturating_sub((indent as u16).saturating_mul(2));

    sizing
        .images
        .get(&url.to_string())
        .and_then(Remote::value)
        .map(|loaded| loaded.rows_for(width, sizing.max_rows) as usize)
        .unwrap_or(PLACEHOLDER_ROWS)
}

fn append_comment(
    lines: &mut Vec<Line<'static>>,
    images: &mut Vec<Placed>,
    threaded: ThreadedComment,
    body: &Rendered,
    highlighted: bool,
    now: Timestamp,
    sizing: Option<&Sizing<'_>>,
) {
    let ThreadedComment { comment, depth } = threaded;
    let indent = "  ".repeat(depth);
    let body_indent = "  ".repeat(depth + 1);

    let mut header: Vec<Span<'static>> = Vec::new();

    if depth > 0 {
        header.push(Span::styled(format!("{indent}└ "), theme::dim()));
    }

    header.push(Span::styled(
        comment.author.clone().unwrap_or_else(|| "unknown".into()),
        theme::comment_author(),
    ));

    header.push(Span::styled(
        format!(" · {}", comment.created_at.humanise(now)),
        theme::dim(),
    ));

    if highlighted {
        for span in &mut header {
            span.style = span.style.add_modifier(Modifier::REVERSED);
        }
    }

    lines.push(Line::from(header));

    append_rendered(lines, images, body, depth + 1, sizing);

    if let Some(chips) = reaction_chips(&comment.reactions) {
        let mut spans = vec![Span::raw(body_indent.clone())];

        spans.extend(chips.spans);
        lines.push(Line::from(spans));
    }

    lines.push(Line::from(""));
}
