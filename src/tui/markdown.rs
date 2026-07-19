use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

const RULE_WIDTH: usize = 40;

pub fn render(input: &str, base: Style) -> Vec<Line<'static>> {
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
    table_header: bool,
    row_started: bool,
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
            table_header: false,
            row_started: false,
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

    fn open_line(&mut self) {
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
                self.spans.push(Span::styled(part.to_string(), style));
            }

            if parts.peek().is_some() {
                self.flush_line();
            }
        }
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
                self.spans
                    .push(Span::styled(text.to_string(), code_style(self.base)));
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
            Tag::Image { .. } => {
                self.open_line();
                self.spans
                    .push(Span::styled("🖼 ".to_string(), dim_style(self.base)));
                self.push_style(dim_style(self.base).add_modifier(Modifier::ITALIC));
            }
            Tag::Table(_) => {
                self.flush_line();
                self.gap();
            }
            Tag::TableHead => {
                self.table_header = true;
                self.row_started = false;
                self.open_line();
            }
            Tag::TableRow => {
                self.row_started = false;
                self.open_line();
            }
            Tag::TableCell => {
                if self.row_started {
                    self.spans
                        .push(Span::styled(" │ ".to_string(), dim_style(self.base)));
                }
                self.row_started = true;
                if self.table_header {
                    self.push_style(self.current_style().add_modifier(Modifier::BOLD));
                }
            }
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
            TagEnd::Image => self.pop_style(),
            TagEnd::TableCell => {
                if self.table_header {
                    self.pop_style();
                }
            }
            TagEnd::TableHead => {
                self.flush_line();
                self.table_header = false;
                self.lines.push(Line::from(Span::styled(
                    "─".repeat(RULE_WIDTH),
                    dim_style(self.base),
                )));
            }
            TagEnd::TableRow => self.flush_line(),
            TagEnd::Table => {}
            TagEnd::HtmlBlock | TagEnd::FootnoteDefinition | TagEnd::MetadataBlock(_) => {}
            TagEnd::DefinitionList
            | TagEnd::DefinitionListTitle
            | TagEnd::DefinitionListDefinition
            | TagEnd::Superscript
            | TagEnd::Subscript => {}
        }
    }

    fn finish(mut self) -> Vec<Line<'static>> {
        self.flush_line();
        while self.lines.last().is_some_and(is_blank) {
            self.lines.pop();
        }
        self.lines
    }
}

fn is_blank(line: &Line) -> bool {
    line.spans.iter().all(|span| span.content.is_empty())
}

fn heading_style(base: Style, level: HeadingLevel) -> Style {
    let style = base.add_modifier(Modifier::BOLD);
    match level {
        HeadingLevel::H1 => style.fg(Color::White),
        HeadingLevel::H2 => style.fg(Color::Cyan),
        _ => style.fg(Color::Rgb(0x81, 0xa1, 0xc1)),
    }
}

fn code_style(base: Style) -> Style {
    base.fg(Color::Rgb(0xa3, 0xbe, 0x8c))
}

fn link_style(base: Style) -> Style {
    base.fg(Color::Blue).add_modifier(Modifier::UNDERLINED)
}

fn quote_style(base: Style) -> Style {
    base.fg(Color::DarkGray)
}

fn dim_style(base: Style) -> Style {
    base.fg(Color::DarkGray)
}

fn marker_style(base: Style) -> Style {
    base.fg(Color::Rgb(0x81, 0xa1, 0xc1))
}

fn task_marker(base: Style, checked: bool) -> Span<'static> {
    if checked {
        Span::styled("[x] ".to_string(), base.fg(Color::Green))
    } else {
        Span::styled("[ ] ".to_string(), base.fg(Color::DarkGray))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(input: &str) -> Vec<String> {
        render(input, Style::default())
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect()
    }

    fn span_style(input: &str, needle: &str) -> Style {
        render(input, Style::default())
            .into_iter()
            .flat_map(|line| line.spans)
            .find(|span| span.content.contains(needle))
            .unwrap_or_else(|| panic!("no span containing {needle:?}"))
            .style
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
    fn strong_text_is_bold() {
        assert!(span_style("**loud**", "loud")
            .add_modifier
            .contains(Modifier::BOLD));
    }

    #[test]
    fn emphasis_text_is_italic() {
        assert!(span_style("*soft*", "soft")
            .add_modifier
            .contains(Modifier::ITALIC));
    }

    #[test]
    fn headings_are_bold() {
        assert!(span_style("# Title", "Title")
            .add_modifier
            .contains(Modifier::BOLD));
    }

    #[test]
    fn trailing_blank_lines_are_trimmed() {
        assert_eq!(lines("text\n\n\n"), vec!["text"]);
    }

    #[test]
    fn empty_input_produces_no_lines() {
        assert!(lines("").is_empty());
    }

    #[test]
    fn inline_code_keeps_its_text_and_is_styled() {
        assert_eq!(lines("run `cargo test` now"), vec!["run cargo test now"]);
        assert_eq!(
            span_style("run `cargo test` now", "cargo test").fg,
            Some(Color::Rgb(0xa3, 0xbe, 0x8c))
        );
    }

    #[test]
    fn strikethrough_is_crossed_out() {
        assert!(span_style("~~gone~~", "gone")
            .add_modifier
            .contains(Modifier::CROSSED_OUT));
    }

    #[test]
    fn nested_emphasis_applies_both_modifiers() {
        let style = span_style("***loud***", "loud");
        assert!(style.add_modifier.contains(Modifier::BOLD));
        assert!(style.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn heading_levels_get_distinct_colours() {
        assert_eq!(span_style("# One", "One").fg, Some(Color::White));
        assert_eq!(span_style("## Two", "Two").fg, Some(Color::Cyan));
        assert_eq!(
            span_style("### Three", "Three").fg,
            Some(Color::Rgb(0x81, 0xa1, 0xc1))
        );
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
    fn images_render_alt_text_with_a_glyph() {
        assert_eq!(lines("![a diagram](chart.png)"), vec!["🖼 a diagram"]);
    }

    #[test]
    fn tables_render_headers_a_separator_and_rows() {
        let out = lines("| A | B |\n| - | - |\n| 1 | 2 |");
        assert_eq!(out[0], "A │ B");
        assert_eq!(out[1], "─".repeat(RULE_WIDTH));
        assert_eq!(out[2], "1 │ 2");
        assert!(span_style("| A | B |\n| - | - |\n| 1 | 2 |", "A")
            .add_modifier
            .contains(Modifier::BOLD));
    }

    #[test]
    fn a_heading_is_separated_from_a_following_list() {
        assert_eq!(lines("## Steps\n- first"), vec!["Steps", "", "• first"]);
    }
}
