mod style;
mod table;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use crate::api::{ImageOrigin, ImageUrl};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use style::{
    code_style, dim_style, heading_style, link_style, marker_style, mention_style, quote_style,
    task_marker,
};
use table::Table;

const RULE_WIDTH: usize = 40;
const PROFILE_PREFIX: &str = "https://linear.app/";
const PROFILE_SEGMENT: &str = "/profiles/";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImagePlacement {
    pub url: ImageUrl,
    pub alt: String,
    pub top: usize,
}

#[derive(Debug, Clone, Default)]
pub struct Rendered {
    pub lines: Vec<Line<'static>>,
    pub images: Vec<ImagePlacement>,
}

pub fn render(input: &str, base: Style) -> Rendered {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);

    let mut writer = Writer::new(base);

    for event in Parser::new_ext(input, options) {
        writer.event(event);
    }

    writer.finish()
}

pub fn render_lines(input: &str, base: Style) -> Vec<Line<'static>> {
    render(input, base).lines
}

struct ListCtx {
    next: Option<u64>,
}

struct Writer {
    base: Style,
    lines: Vec<Line<'static>>,
    spans: Vec<Span<'static>>,
    line_open: bool,
    styles: Vec<Style>,
    lists: Vec<ListCtx>,
    quote_depth: usize,
    pending_marker: Option<Vec<Span<'static>>>,
    code_buf: Option<String>,
    table: Option<Table>,
    cell: Option<Vec<Span<'static>>>,
    images: Vec<ImagePlacement>,
    open_image: Option<(String, Vec<Span<'static>>)>,
}

impl Writer {
    fn new(base: Style) -> Self {
        Self {
            base,
            lines: Vec::new(),
            spans: Vec::new(),
            line_open: false,
            styles: vec![base],
            lists: Vec::new(),
            quote_depth: 0,
            pending_marker: None,
            code_buf: None,
            table: None,
            cell: None,
            images: Vec::new(),
            open_image: None,
        }
    }

    fn current_style(&self) -> Style {
        *self.styles.last().unwrap_or(&self.base)
    }

    fn push_style(&mut self, style: Style) {
        self.styles.push(style);
    }

    fn pop_style(&mut self) {
        self.styles.pop();
        if self.styles.is_empty() {
            self.styles.push(self.base);
        }
    }

    fn top_level(&self) -> bool {
        self.lists.is_empty() && self.quote_depth == 0
    }

    fn gap(&mut self) {
        if self.lines.last().is_some_and(|line| !is_blank(line)) {
            self.lines.push(Line::default());
        }
    }

    fn target(&mut self) -> &mut Vec<Span<'static>> {
        if let Some((_, alt)) = &mut self.open_image {
            return alt;
        }

        match &mut self.cell {
            Some(cell) => cell,
            None => &mut self.spans,
        }
    }

    fn open_line(&mut self) {
        if self.cell.is_some() {
            return;
        }

        if self.line_open {
            return;
        }

        self.line_open = true;

        let mut prefix: Vec<Span<'static>> = Vec::new();

        for _ in 0..self.quote_depth {
            prefix.push(Span::styled("▌ ".to_string(), quote_style(self.base)));
        }

        if !self.lists.is_empty() {
            let depth = self.lists.len();

            if depth > 1 {
                prefix.push(Span::raw("  ".repeat(depth - 1)));
            }

            match self.pending_marker.take() {
                Some(marker) => prefix.extend(marker),
                None => prefix.push(Span::raw("  ".to_string())),
            }
        }

        self.spans = prefix;
    }

    fn flush_line(&mut self) {
        if self.line_open {
            let spans = std::mem::take(&mut self.spans);

            self.lines.push(Line::from(spans));
            self.line_open = false;
        }
    }

    fn push_text(&mut self, text: &str, style: Style) {
        let mut parts = text.split('\n').peekable();

        while let Some(part) = parts.next() {
            if !part.is_empty() {
                self.open_line();
                self.push_run(part, style);
            }

            if parts.peek().is_some() {
                self.flush_line();
            }
        }
    }

    fn push_run(&mut self, text: &str, style: Style) {
        let mention = mention_style(self.base);
        let mut out: Vec<Span<'static>> = Vec::new();
        let mut rest = text;

        while let Some(start) = rest.find(PROFILE_PREFIX) {
            let token_end = rest[start..]
                .find(char::is_whitespace)
                .map(|offset| start + offset)
                .unwrap_or(rest.len());

            match profile_handle(&rest[start..token_end]) {
                Some(handle) => {
                    if start > 0 {
                        out.push(Span::styled(rest[..start].to_string(), style));
                    }
                    out.push(Span::styled(format!("@{handle}"), mention));
                }
                None => {
                    out.push(Span::styled(rest[..token_end].to_string(), style));
                }
            }

            rest = &rest[token_end..];
        }

        if !rest.is_empty() {
            out.push(Span::styled(rest.to_string(), style));
        }

        self.target().extend(out);
    }

    fn event(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) => {
                if let Some(buf) = self.code_buf.as_mut() {
                    buf.push_str(&text);
                } else {
                    let style = self.current_style();
                    self.push_text(&text, style);
                }
            }
            Event::Code(text) => {
                self.open_line();
                let style = code_style(self.base);
                self.target().push(Span::styled(text.to_string(), style));
            }
            Event::Html(text) | Event::InlineHtml(text) => {
                self.push_text(text.trim_end_matches('\n'), dim_style(self.base));
            }
            Event::SoftBreak | Event::HardBreak => self.flush_line(),
            Event::Rule => {
                self.gap();
                self.lines.push(Line::from(Span::styled(
                    "─".repeat(RULE_WIDTH),
                    dim_style(self.base),
                )));
            }
            Event::TaskListMarker(checked) => {
                self.pending_marker = Some(vec![task_marker(self.base, checked)]);
            }
            Event::FootnoteReference(_) | Event::InlineMath(_) | Event::DisplayMath(_) => {}
        }
    }

    fn start(&mut self, tag: Tag) {
        match tag {
            Tag::Paragraph => {
                self.flush_line();
                if self.top_level() {
                    self.gap();
                }
            }
            Tag::Heading { level, .. } => {
                self.flush_line();
                self.gap();
                self.push_style(heading_style(self.base, level));
            }
            Tag::BlockQuote(_) => {
                self.flush_line();
                if self.top_level() {
                    self.gap();
                }
                self.quote_depth += 1;
            }
            Tag::CodeBlock(_) => {
                self.flush_line();
                self.gap();
                self.code_buf = Some(String::new());
            }
            Tag::List(start) => {
                self.flush_line();
                if self.top_level() {
                    self.gap();
                }
                self.lists.push(ListCtx { next: start });
            }
            Tag::Item => {
                let marker = match self.lists.last_mut() {
                    Some(ctx) => match ctx.next {
                        Some(n) => {
                            ctx.next = Some(n + 1);
                            Span::styled(format!("{n}. "), marker_style(self.base))
                        }
                        None => Span::styled("• ".to_string(), marker_style(self.base)),
                    },
                    None => Span::styled("• ".to_string(), marker_style(self.base)),
                };
                self.pending_marker = Some(vec![marker]);
            }
            Tag::Emphasis => self.push_style(self.current_style().add_modifier(Modifier::ITALIC)),
            Tag::Strong => self.push_style(self.current_style().add_modifier(Modifier::BOLD)),
            Tag::Strikethrough => {
                self.push_style(self.current_style().add_modifier(Modifier::CROSSED_OUT))
            }
            Tag::Link { .. } => self.push_style(link_style(self.base)),
            Tag::Image { dest_url, .. } => {
                self.open_image = Some((dest_url.to_string(), Vec::new()));
                self.push_style(dim_style(self.base).add_modifier(Modifier::ITALIC));
            }
            Tag::Table(aligns) => {
                self.flush_line();
                self.gap();
                self.table = Some(Table::new(aligns));
            }
            Tag::TableHead | Tag::TableRow => {}
            Tag::TableCell => self.cell = Some(Vec::new()),
            Tag::HtmlBlock | Tag::FootnoteDefinition(_) | Tag::MetadataBlock(_) => {}
            Tag::DefinitionList
            | Tag::DefinitionListTitle
            | Tag::DefinitionListDefinition
            | Tag::Superscript
            | Tag::Subscript => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => self.flush_line(),
            TagEnd::Heading(_) => {
                self.flush_line();
                self.pop_style();
            }
            TagEnd::BlockQuote(_) => {
                self.flush_line();
                self.quote_depth = self.quote_depth.saturating_sub(1);
            }
            TagEnd::CodeBlock => {
                let buf = self.code_buf.take().unwrap_or_default();
                let content = buf.strip_suffix('\n').unwrap_or(&buf);
                for line in content.split('\n') {
                    self.open_line();
                    self.spans
                        .push(Span::styled("▏ ".to_string(), dim_style(self.base)));
                    self.spans
                        .push(Span::styled(line.to_string(), code_style(self.base)));
                    self.flush_line();
                }
            }
            TagEnd::List(_) => {
                self.lists.pop();
            }
            TagEnd::Item => {
                self.flush_line();
                self.pending_marker = None;
            }
            TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough | TagEnd::Link => {
                self.pop_style()
            }
            TagEnd::Image => {
                self.pop_style();

                if let Some((url, alt)) = self.open_image.take() {
                    let alt = alt
                        .iter()
                        .map(|span| span.content.as_ref())
                        .collect::<String>();

                    self.reserve_image(url, alt);
                }
            }
            TagEnd::TableCell => {
                if let Some(cell) = self.cell.take() {
                    if let Some(table) = &mut self.table {
                        table.push_cell(cell);
                    }
                }
            }
            TagEnd::TableHead => {
                if let Some(table) = &mut self.table {
                    table.finish_header();
                }
            }
            TagEnd::TableRow => {
                if let Some(table) = &mut self.table {
                    table.finish_row();
                }
            }
            TagEnd::Table => {
                if let Some(table) = self.table.take() {
                    let base = self.base;
                    self.lines.extend(table.render(base));
                }
            }
            TagEnd::HtmlBlock | TagEnd::FootnoteDefinition | TagEnd::MetadataBlock(_) => {}
            TagEnd::DefinitionList
            | TagEnd::DefinitionListTitle
            | TagEnd::DefinitionListDefinition
            | TagEnd::Superscript
            | TagEnd::Subscript => {}
        }
    }

    fn finish(mut self) -> Rendered {
        self.flush_line();

        let reserved = self
            .images
            .iter()
            .map(|image| image.top + 1)
            .max()
            .unwrap_or(0);

        while self.lines.len() > reserved && self.lines.last().is_some_and(is_blank) {
            self.lines.pop();
        }

        Rendered {
            lines: self.lines,
            images: self.images,
        }
    }

    fn reserve_image(&mut self, url: String, alt: String) {
        if self.line_open && self.spans.iter().all(|span| span.content.is_empty()) {
            self.spans.clear();
            self.line_open = false;
        } else {
            self.flush_line();
        }

        let top = self.lines.len();

        let label = if alt.is_empty() {
            "image".to_string()
        } else {
            alt.clone()
        };
        let parsed = ImageUrl::parse(&url);
        let hint = match &parsed {
            Some(url) => image_source(url),
            None => "  unsupported link".to_string(),
        };

        self.lines.push(Line::from(vec![
            Span::styled(
                format!("🖼 {label}"),
                dim_style(self.base).add_modifier(Modifier::ITALIC),
            ),
            Span::styled(hint, dim_style(self.base)),
        ]));

        if let Some(url) = parsed {
            self.images.push(ImagePlacement { url, alt, top });
        }
    }
}

fn image_source(url: &ImageUrl) -> String {
    match (url.origin(), url.url().host_str()) {
        (ImageOrigin::External, Some(host)) => format!("  from {host}"),
        (ImageOrigin::External, None) | (ImageOrigin::LinearUpload, _) => String::new(),
    }
}

fn is_blank(line: &Line) -> bool {
    line.spans.iter().all(|span| span.content.is_empty())
}

fn profile_handle(url: &str) -> Option<&str> {
    let index = url.find(PROFILE_SEGMENT)?;
    let handle = &url[index + PROFILE_SEGMENT.len()..];

    (!handle.is_empty()).then_some(handle)
}

#[cfg(test)]
#[allow(clippy::disallowed_types)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn lines(input: &str) -> Vec<String> {
        render_lines(input, Style::default())
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect()
    }

    fn span_style(input: &str, needle: &str) -> Result<Style, String> {
        render_lines(input, Style::default())
            .into_iter()
            .flat_map(|line| line.spans)
            .find(|span| span.content.contains(needle))
            .map(|span| span.style)
            .ok_or_else(|| format!("no span containing {needle:?}"))
    }

    #[test]
    fn paragraphs_are_separated_by_a_blank_line() {
        assert_eq!(lines("first\n\nsecond"), vec!["first", "", "second"]);
    }

    #[test]
    fn soft_breaks_split_lines_without_a_gap() {
        assert_eq!(lines("first\nsecond"), vec!["first", "second"]);
    }

    #[test]
    fn bullet_lists_get_markers_and_nesting_indents() {
        assert_eq!(
            lines("- one\n- two\n  - nested"),
            vec!["• one", "• two", "  • nested"]
        );
    }

    #[test]
    fn ordered_lists_number_their_items() {
        assert_eq!(lines("1. one\n2. two"), vec!["1. one", "2. two"]);
    }

    #[test]
    fn task_items_render_checkboxes_instead_of_bullets() {
        assert_eq!(
            lines("- [x] done\n- [ ] todo"),
            vec!["[x] done", "[ ] todo"]
        );
    }

    #[test]
    fn blockquotes_are_prefixed() {
        assert_eq!(lines("> quoted"), vec!["▌ quoted"]);
    }

    #[test]
    fn code_blocks_keep_a_gutter_and_indentation() {
        assert_eq!(
            lines("```\nfn main() {\n    body\n}\n```"),
            vec!["▏ fn main() {", "▏     body", "▏ }"]
        );
    }

    #[test]
    fn links_render_their_text_without_the_url() {
        assert_eq!(
            lines("see [the docs](https://x.test)"),
            vec!["see the docs"]
        );
    }

    #[test]
    fn strong_text_is_bold() -> TestResult {
        assert!(span_style("**loud**", "loud")?
            .add_modifier
            .contains(Modifier::BOLD));

        Ok(())
    }

    #[test]
    fn emphasis_text_is_italic() -> TestResult {
        assert!(span_style("*soft*", "soft")?
            .add_modifier
            .contains(Modifier::ITALIC));

        Ok(())
    }

    #[test]
    fn headings_are_bold() -> TestResult {
        assert!(span_style("# Title", "Title")?
            .add_modifier
            .contains(Modifier::BOLD));

        Ok(())
    }

    #[test]
    fn trailing_blank_lines_are_trimmed() {
        assert_eq!(lines("text\n\n\n"), vec!["text"]);
    }

    #[test]
    fn profile_urls_render_as_mentions() {
        assert_eq!(
            lines("ping https://linear.app/dans-donuts/profiles/danniieelg please"),
            vec!["ping @danniieelg please"]
        );
    }

    #[test]
    fn non_profile_linear_urls_are_left_alone() {
        assert_eq!(
            lines("see https://linear.app/dans-donuts/issue/DAN2-8"),
            vec!["see https://linear.app/dans-donuts/issue/DAN2-8"]
        );
    }

    #[test]
    fn empty_input_produces_no_lines() {
        assert!(lines("").is_empty());
    }

    #[test]
    fn inline_code_keeps_its_text_and_is_styled() -> TestResult {
        assert_eq!(lines("run `cargo test` now"), vec!["run cargo test now"]);
        assert_eq!(
            span_style("run `cargo test` now", "cargo test")?.fg,
            Some(Color::Green)
        );

        Ok(())
    }

    #[test]
    fn strikethrough_is_crossed_out() -> TestResult {
        assert!(span_style("~~gone~~", "gone")?
            .add_modifier
            .contains(Modifier::CROSSED_OUT));

        Ok(())
    }

    #[test]
    fn nested_emphasis_applies_both_modifiers() -> TestResult {
        let style = span_style("***loud***", "loud")?;
        assert!(style.add_modifier.contains(Modifier::BOLD));
        assert!(style.add_modifier.contains(Modifier::ITALIC));

        Ok(())
    }

    #[test]
    fn heading_levels_get_distinct_colours() -> TestResult {
        assert_eq!(span_style("# One", "One")?.fg, Some(Color::Reset));
        assert_eq!(span_style("## Two", "Two")?.fg, Some(Color::Cyan));
        assert_eq!(span_style("### Three", "Three")?.fg, Some(Color::Blue));

        Ok(())
    }

    #[test]
    fn ordered_lists_respect_the_start_number() {
        assert_eq!(lines("3. three\n4. four"), vec!["3. three", "4. four"]);
    }

    #[test]
    fn horizontal_rules_span_the_rule_width() {
        assert_eq!(lines("above\n\n---\n\nbelow"), {
            let rule = "─".repeat(RULE_WIDTH);
            vec![
                "above".to_string(),
                String::new(),
                rule,
                String::new(),
                "below".to_string(),
            ]
        });
    }

    #[test]
    fn nested_blockquotes_stack_their_prefixes() {
        assert_eq!(lines("> > deep"), vec!["▌ ▌ deep"]);
    }

    #[test]
    fn images_reserve_rows_and_record_where_they_went() -> TestResult {
        let rendered = render(
            "![a diagram](https://uploads.linear.app/chart.png)",
            Style::default(),
        );

        assert_eq!(rendered.lines.len(), 1);
        assert_eq!(
            rendered
                .lines
                .first()
                .ok_or("no lines")?
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>(),
            "🖼 a diagram"
        );
        assert_eq!(
            rendered.images,
            vec![ImagePlacement {
                url: ImageUrl::parse("https://uploads.linear.app/chart.png")
                    .ok_or("a valid upload url")?,
                alt: "a diagram".into(),
                top: 0,
            }]
        );

        Ok(())
    }

    #[test]
    fn an_image_after_text_records_its_offset() -> TestResult {
        let rendered = render(
            "intro\n\n![shot](https://uploads.linear.app/a.png)",
            Style::default(),
        );

        let placement = rendered.images.first().ok_or("no image placements")?;

        assert_eq!(placement.top, 2);
        assert_eq!(
            rendered
                .lines
                .get(placement.top)
                .ok_or("no line at the image's offset")?
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>(),
            "🖼 shot"
        );

        Ok(())
    }

    #[test]
    fn an_external_image_names_its_host() -> TestResult {
        let text = |rendered: &Rendered| -> Result<String, &'static str> {
            Ok(rendered
                .lines
                .first()
                .ok_or("no lines")?
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect())
        };

        let external = render("![pixel](https://tracker.example/p.gif)", Style::default());
        let upload = render(
            "![shot](https://uploads.linear.app/a.png)",
            Style::default(),
        );

        assert_eq!(text(&external)?, "🖼 pixel  from tracker.example");
        assert_eq!(text(&upload)?, "🖼 shot");

        Ok(())
    }

    #[test]
    fn an_unsupported_image_link_is_labelled_and_never_placed() -> TestResult {
        let rendered = render("![shot](file:///etc/passwd)", Style::default());

        assert!(rendered.images.is_empty(), "nothing can fetch it");
        assert_eq!(
            rendered
                .lines
                .first()
                .ok_or("no lines")?
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>(),
            "🖼 shot  unsupported link"
        );

        Ok(())
    }

    #[test]
    fn tables_render_headers_a_separator_and_rows() -> TestResult {
        let out = lines("| A | B |\n| - | - |\n| 1 | 2 |");
        let [header, separator, row, ..] = out.as_slice() else {
            return Err(format!("expected at least three lines, got {out:?}").into());
        };
        assert_eq!(header, "A │ B");
        assert_eq!(*separator, "─".repeat(5));
        assert_eq!(row, "1 │ 2");
        assert!(span_style("| A | B |\n| - | - |\n| 1 | 2 |", "A")?
            .add_modifier
            .contains(Modifier::BOLD));

        Ok(())
    }

    #[test]
    fn tables_pad_cells_so_columns_line_up() -> TestResult {
        let out = lines("| Time | Target |\n| - | - |\n| 6pm | 430C |\n| 7pm | 12345C |");
        let [header, separator, first, second, ..] = out.as_slice() else {
            return Err(format!("expected at least four lines, got {out:?}").into());
        };
        assert_eq!(header, "Time │ Target");
        assert_eq!(*separator, "─".repeat(4 + 6 + 3));
        assert_eq!(first, "6pm  │ 430C");
        assert_eq!(second, "7pm  │ 12345C");

        Ok(())
    }

    #[test]
    fn table_columns_right_align_when_marked() -> TestResult {
        let out = lines("| N |\n| --: |\n| 5 |\n| 4321 |");
        let [header, _, first, second, ..] = out.as_slice() else {
            return Err(format!("expected at least four lines, got {out:?}").into());
        };
        assert_eq!(header, "   N");
        assert_eq!(first, "   5");
        assert_eq!(second, "4321");

        Ok(())
    }

    #[test]
    fn a_heading_is_separated_from_a_following_list() {
        assert_eq!(lines("## Steps\n- first"), vec!["Steps", "", "• first"]);
    }
}
