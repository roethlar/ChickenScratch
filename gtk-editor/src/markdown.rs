use anyhow::Result;
use gtk::prelude::*;
use gtk::{TextBuffer, TextIter, TextTag};

use crate::ui::{Tags, BULLET_PREFIX};

#[derive(Clone, Copy)]
enum SpanKind {
    Bold,
    Italic,
    Strike,
    Code,
    Heading1,
    Heading2,
    ListItem,
    BlockQuote,
}

struct StyleSpan {
    start: i32,
    end: i32,
    kind: SpanKind,
}

#[derive(Clone)]
struct InlineSegment {
    text: String,
    bold: bool,
    italic: bool,
    strike: bool,
    code: bool,
}

impl InlineSegment {
    fn empty() -> Self {
        Self {
            text: String::new(),
            bold: false,
            italic: false,
            strike: false,
            code: false,
        }
    }
}

pub fn apply_to_buffer(buffer: &TextBuffer, markdown: &str) -> Result<()> {
    let mut plain_text = String::new();
    let mut spans: Vec<StyleSpan> = Vec::new();
    let mut offset: i32 = 0;

    let lines = split_lines_preserve_blank(markdown);

    for (idx, raw) in lines.iter().enumerate() {
        let mut line = raw.trim_end_matches('\r').to_string();
        let mut line = raw_line.trim_end_matches('\r').to_string();

        let mut blockquote = false;
        if let Some(rest) = line.strip_prefix("> ") {
            blockquote = true;
            line = rest.to_string();
        } else if line == ">" {
            blockquote = true;
            line.clear();
        }

        let mut heading = None;
        if let Some(rest) = line.strip_prefix("## ") {
            heading = Some(2);
            line = rest.to_string();
        } else if let Some(rest) = line.strip_prefix("# ") {
            heading = Some(1);
            line = rest.to_string();
        }

        let mut list_item = false;
        if let Some(rest) = line.strip_prefix("- ") {
            list_item = true;
            line = rest.to_string();
        } else if let Some(rest) = line.strip_prefix("* ") {
            list_item = true;
            line = rest.to_string();
        }

        let mut segments = parse_inline(&line);
        let line_start = offset;

        if list_item {
            plain_text.push_str(BULLET_PREFIX);
            offset += char_count(BULLET_PREFIX);
        }

        for segment in segments.drain(..) {
            if segment.text.is_empty() {
                continue;
            }
            let start = offset;
            plain_text.push_str(&segment.text);
            offset += char_count(&segment.text);
            if segment.bold {
                spans.push(StyleSpan {
                    start,
                    end: offset,
                    kind: SpanKind::Bold,
                });
            }
            if segment.italic {
                spans.push(StyleSpan {
                    start,
                    end: offset,
                    kind: SpanKind::Italic,
                });
            }
            if segment.strike {
                spans.push(StyleSpan {
                    start,
                    end: offset,
                    kind: SpanKind::Strike,
                });
            }
            if segment.code {
                spans.push(StyleSpan {
                    start,
                    end: offset,
                    kind: SpanKind::Code,
                });
            }
        }

        let line_end = offset;

        if let Some(level) = heading {
            spans.push(StyleSpan {
                start: line_start,
                end: line_end,
                kind: if level == 1 {
                    SpanKind::Heading1
                } else {
                    SpanKind::Heading2
                },
            });
        }

        if list_item {
            spans.push(StyleSpan {
                start: line_start,
                end: line_end,
                kind: SpanKind::ListItem,
            });
        }

        if blockquote {
            spans.push(StyleSpan {
                start: line_start,
                end: line_end,
                kind: SpanKind::BlockQuote,
            });
        }

        if idx + 1 < lines.len() {
            plain_text.push('\n');
            offset += 1;
        }
    }

    buffer.set_text(&plain_text);

    let tag_table = buffer.tag_table();
    for span in spans {
        if span.start == span.end {
            continue;
        }

        let tag_name = match span.kind {
            SpanKind::Bold => Tags::BOLD,
            SpanKind::Italic => Tags::ITALIC,
            SpanKind::Strike => Tags::STRIKE,
            SpanKind::Code => Tags::CODE,
            SpanKind::Heading1 => Tags::HEADING1,
            SpanKind::Heading2 => Tags::HEADING2,
            SpanKind::ListItem => Tags::LIST_ITEM,
            SpanKind::BlockQuote => Tags::BLOCKQUOTE,
        };

        if let Some(tag) = tag_table.lookup(tag_name) {
            let mut start_iter = buffer.iter_at_offset(span.start);
            let mut end_iter = buffer.iter_at_offset(span.end);
            buffer.apply_tag(&tag, &mut start_iter, &mut end_iter);
        }
    }

    Ok(())
}

pub fn buffer_to_markdown(buffer: &TextBuffer) -> String {
    let full_text = buffer
        .text(&buffer.start_iter(), &buffer.end_iter(), false)
        .to_string();
    let ends_with_newline = full_text.ends_with('\n');

    let mut lines: Vec<String> = Vec::new();
    let tag_lookup = TagLookup::new(buffer);

    let mut iter = buffer.start_iter();
    while iter != buffer.end_iter() {
        let line_start = iter.clone();
        let mut line_end = line_start.clone();
        let had_line = line_end.forward_to_line_end();

        let mut segments = collect_segments(buffer, &line_start, &line_end, &tag_lookup);
        if tag_lookup.list_item.is_some()
            && line_has_tag(&line_start, &line_end, tag_lookup.list_item.as_ref())
        {
            if let Some(first) = segments.first_mut() {
                if first.text.starts_with(BULLET_PREFIX) {
                    first.text = first.text[BULLET_PREFIX.len()..].to_string();
                }
            }
        }

        let mut line_md = build_markdown_line(
            &segments,
            line_has_tag(&line_start, &line_end, tag_lookup.heading1.as_ref()),
            line_has_tag(&line_start, &line_end, tag_lookup.heading2.as_ref()),
            line_has_tag(&line_start, &line_end, tag_lookup.list_item.as_ref()),
            line_has_tag(&line_start, &line_end, tag_lookup.blockquote.as_ref()),
        );

        if !had_line && line_start == buffer.end_iter() && line_md.is_empty() {
            // trailing empty document
        }

        lines.push(line_md);

        if !had_line {
            break;
        }

        let mut next_iter = line_end.clone();
        if !next_iter.forward_char() {
            break;
        }
        iter = next_iter;
    }

    if ends_with_newline {
        lines.push(String::new());
    }

    lines.join("\n")
}

pub fn word_count(buffer: &TextBuffer) -> usize {
    let text = buffer
        .text(&buffer.start_iter(), &buffer.end_iter(), false)
        .to_string();
    text.split_whitespace().count()
}

fn split_lines_preserve_blank(input: &str) -> Vec<&str> {
    if input.is_empty() {
        return vec![""];
    }

    let mut lines: Vec<&str> = input.split('\n').collect();
    if input.ends_with('\n') {
        lines.push("");
    }
    lines
}

fn char_count(s: &str) -> i32 {
    s.chars().count() as i32
}

fn parse_inline(line: &str) -> Vec<InlineSegment> {
    let mut segments = Vec::new();
    let mut current = InlineSegment::empty();
    let mut chars = line.chars().peekable();
    let mut bold = false;
    let mut italic = false;
    let mut strike = false;
    let mut code = false;

    while let Some(ch) = chars.next() {
        if code {
            if ch == '`' {
                if !current.text.is_empty() {
                    segments.push(current.clone());
                    current.text.clear();
                }
                code = false;
                continue;
            }
            current.text.push(ch);
            continue;
        }

        match ch {
            '*' => {
                if let Some('*') = chars.peek() {
                    chars.next();
                    if !current.text.is_empty() {
                        segments.push(current.clone());
                        current.text.clear();
                    }
                    bold = !bold;
                    current.bold = bold;
                    current.italic = italic;
                    current.strike = strike;
                    continue;
                } else {
                    if !current.text.is_empty() {
                        segments.push(current.clone());
                        current.text.clear();
                    }
                    italic = !italic;
                    current.bold = bold;
                    current.italic = italic;
                    current.strike = strike;
                    continue;
                }
            }
            '~' => {
                if let Some('~') = chars.peek() {
                    chars.next();
                    if !current.text.is_empty() {
                        segments.push(current.clone());
                        current.text.clear();
                    }
                    strike = !strike;
                    current.bold = bold;
                    current.italic = italic;
                    current.strike = strike;
                    continue;
                }
            }
            '`' => {
                if !current.text.is_empty() {
                    segments.push(current.clone());
                    current.text.clear();
                }
                code = true;
                current.code = true;
                current.bold = false;
                current.italic = false;
                current.strike = false;
                continue;
            }
            _ => {}
        }

        current.bold = bold;
        current.italic = italic;
        current.strike = strike;
        current.code = code;
        current.text.push(ch);
    }

    if !current.text.is_empty() {
        segments.push(current);
    }

    segments
}

struct TagLookup {
    bold: Option<TextTag>,
    italic: Option<TextTag>,
    strike: Option<TextTag>,
    code: Option<TextTag>,
    heading1: Option<TextTag>,
    heading2: Option<TextTag>,
    list_item: Option<TextTag>,
    blockquote: Option<TextTag>,
}

impl TagLookup {
    fn new(buffer: &TextBuffer) -> Self {
        let table = buffer.tag_table();
        Self {
            bold: table.lookup(Tags::BOLD),
            italic: table.lookup(Tags::ITALIC),
            strike: table.lookup(Tags::STRIKE),
            code: table.lookup(Tags::CODE),
            heading1: table.lookup(Tags::HEADING1),
            heading2: table.lookup(Tags::HEADING2),
            list_item: table.lookup(Tags::LIST_ITEM),
            blockquote: table.lookup(Tags::BLOCKQUOTE),
        }
    }
}

fn collect_segments(
    buffer: &TextBuffer,
    start: &TextIter,
    end: &TextIter,
    tags: &TagLookup,
) -> Vec<InlineSegment> {
    let mut segments = Vec::new();
    let mut iter = start.clone();

    while iter.compare(end) < 0 {
        let mut segment_end = iter.clone();
        if !segment_end.forward_to_tag_toggle(None) || segment_end.compare(end) > 0 {
            segment_end = end.clone();
        }

        let text = buffer.text(&iter, &segment_end, false).to_string();
        if text.is_empty() {
            iter = segment_end;
            continue;
        }

        let segment = InlineSegment {
            text,
            bold: tags.bold.as_ref().map_or(false, |tag| iter.has_tag(tag)),
            italic: tags.italic.as_ref().map_or(false, |tag| iter.has_tag(tag)),
            strike: tags.strike.as_ref().map_or(false, |tag| iter.has_tag(tag)),
            code: tags.code.as_ref().map_or(false, |tag| iter.has_tag(tag)),
        };

        let same_style = segments.last().map_or(false, |last| {
            last.bold == segment.bold
                && last.italic == segment.italic
                && last.strike == segment.strike
                && last.code == segment.code
        });

        if same_style {
            if let Some(last) = segments.last_mut() {
                last.text.push_str(&segment.text);
            }
        } else {
            segments.push(segment);
        }

        iter = segment_end;
    }

    segments
}

fn line_has_tag(start: &TextIter, end: &TextIter, tag: Option<&TextTag>) -> bool {
    let Some(tag) = tag else {
        return false;
    };

    if start.equal(end) {
        return false;
    }

    if !start.has_tag(tag) {
        return false;
    }

    let mut probe = end.clone();
    if probe.backward_char() {
        probe.has_tag(tag)
    } else {
        false
    }
}

fn build_markdown_line(
    segments: &[InlineSegment],
    heading1: bool,
    heading2: bool,
    list_item: bool,
    blockquote: bool,
) -> String {
    if segments.is_empty() {
        if list_item {
            return "- ".to_string();
        }
        return String::new();
    }

    let mut line = String::new();

    for seg in segments {
        let mut text = escape_markdown(&seg.text, seg.code);

        if seg.code {
            text = format!("`{text}`");
        } else {
            if seg.bold {
                text = format!("**{text}**");
            }
            if seg.italic {
                text = format!("*{text}*");
            }
            if seg.strike {
                text = format!("~~{text}~~");
            }
        }

        line.push_str(&text);
    }

    let trimmed = line.trim_start();
    let mut with_blocks = trimmed.to_string();

    if heading1 {
        with_blocks = format!("# {}", with_blocks);
    } else if heading2 {
        with_blocks = format!("## {}", with_blocks);
    }

    if list_item {
        with_blocks = format!("- {}", with_blocks);
    }

    if blockquote {
        with_blocks = format!("> {}", with_blocks);
    }

    with_blocks
}

fn escape_markdown(text: &str, code: bool) -> String {
    if code {
        return text.replace('`', "\\`");
    }

    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '*' => escaped.push_str("\\*"),
            '_' => escaped.push_str("\\_"),
            '~' => escaped.push_str("\\~"),
            '`' => escaped.push_str("\\`"),
            _ => escaped.push(ch),
        }
    }
    escaped
}
