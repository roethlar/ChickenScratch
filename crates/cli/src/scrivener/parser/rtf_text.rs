//! Plain-text fallback for RTF that Pandoc cannot read (PLAN_SCRIVENER_FIDELITY
//! slice 1).
//!
//! When Pandoc rejects a document, the import must not die and must not
//! drop the document: this extractor yields the text as Markdown paragraphs
//! with no formatting, and the caller names the document in the warnings.
//! Hyperlink fields survive as Markdown links so the `scrivcmt://` anchors
//! slice 4 relies on are kept (review finding plan-3 R1-5).

use super::rtf_prepare::{declared_default_font, document_encoding, font_encodings};
use std::collections::HashMap;

/// Groups whose content is never document text.
const SKIPPED_DESTINATIONS: &[&[u8]] = &[
    b"fonttbl",
    b"colortbl",
    b"stylesheet",
    b"info",
    b"pict",
    b"header",
    b"headerl",
    b"headerr",
    b"headerf",
    b"footer",
    b"footerl",
    b"footerr",
    b"footerf",
    b"listtable",
    b"listoverridetable",
    b"revtbl",
    b"rsidtbl",
    b"xmlnstbl",
    b"themedata",
    b"colorschememapping",
    b"datastore",
    b"latentstyles",
];

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[derive(Default)]
struct Field {
    /// Text of the `\fldinst` destination (e.g. `HYPERLINK "url"`).
    instruction: String,
    /// Text of the `\fldrslt` group.
    result: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Sink {
    /// Ordinary document text.
    Body,
    /// Inside a group that is skipped entirely.
    Skip,
    /// Inside `{\*\fldinst …}` of the innermost field.
    FieldInstruction,
    /// Inside `{\fldrslt …}` of the innermost field.
    FieldResult,
}

struct Group {
    sink: Sink,
    uc: u32,
    /// Active font (`\fN`); its encoding, when the font table gives one,
    /// decides how this group's code-page bytes decode.
    font: Option<i64>,
    /// Set on the `{` of a `\field` group so its `}` can emit the link.
    opens_field: bool,
    /// True until the first control word of the group has been seen
    /// (destination detection happens there).
    fresh: bool,
}

/// Extracts the document text of `input` as Markdown paragraphs. Never fails.
pub fn rtf_to_plain_markdown(input: &[u8]) -> String {
    let mut text = String::new();
    // The active font starts as the document default (`\deffN`); `\plain`
    // goes back to it (review finding s1-3, second re-review).
    let default_font = declared_default_font(input);
    let mut groups: Vec<Group> = vec![Group {
        sink: Sink::Body,
        uc: 1,
        font: default_font,
        opens_field: false,
        fresh: false,
    }];
    let mut fields: Vec<Field> = Vec::new();
    let mut skip_fallback: u32 = 0;
    // Text bytes (hex escapes and literal bytes alike) are gathered and
    // decoded as one run, in the active font's encoding when the font table
    // gives one and the document's otherwise, so a two-byte character
    // decodes whole and Cyrillic is Cyrillic (review finding s1-3).
    let document = document_encoding(input);
    let fonts: HashMap<i64, &'static encoding_rs::Encoding> = font_encodings(input);
    let mut pending: Vec<u8> = Vec::new();
    // A high surrogate from `\uN` waits for its low half (review finding
    // s1-4): RTF carries anything above the BMP as a UTF-16 pair.
    let mut pending_high: Option<u16> = None;

    fn flush_pending(
        pending: &mut Vec<u8>,
        fonts: &HashMap<i64, &'static encoding_rs::Encoding>,
        document: &'static encoding_rs::Encoding,
        groups: &[Group],
        fields: &mut [Field],
        text: &mut String,
    ) {
        if pending.is_empty() {
            return;
        }
        let encoding = groups
            .last()
            .and_then(|g| g.font)
            .and_then(|f| fonts.get(&f).copied())
            .unwrap_or(document);
        let (decoded, _had_errors) = encoding.decode_without_bom_handling(pending);
        for c in decoded.chars() {
            push(c, groups, fields, text);
        }
        pending.clear();
    }

    // Where a character goes, given the current group. Body and link text
    // are escaped for Markdown as they are pushed (review finding plan-3
    // R2-3: literal stars or angle brackets in prose must stay visible);
    // hyperlink targets are not.
    fn push(c: char, groups: &[Group], fields: &mut [Field], text: &mut String) {
        match groups.last().map(|g| g.sink).unwrap_or(Sink::Body) {
            Sink::Body => push_escaped(c, text),
            Sink::Skip => {}
            Sink::FieldInstruction => {
                if let Some(f) = fields.last_mut() {
                    f.instruction.push(c);
                }
            }
            Sink::FieldResult => {
                if let Some(f) = fields.last_mut() {
                    push_escaped(c, &mut f.result);
                }
            }
        }
    }

    // A high surrogate still waiting when text is about to be produced, or
    // when the group holding it closes, is a lone half: show U+FFFD in that
    // place rather than drop it silently. Formatting controls and bare line
    // ends between the halves leave it waiting (review finding s1-4,
    // re-review), and it never escapes its own destination.
    fn resolve_lone_high(
        pending_high: &mut Option<u16>,
        groups: &[Group],
        fields: &mut [Field],
        text: &mut String,
    ) {
        if pending_high.take().is_some() {
            push('\u{FFFD}', groups, fields, text);
        }
    }

    let n = input.len();
    let mut i = 0;
    while i < n {
        let b = input[i];
        // A brace or a control sequence other than a hex escape ends the
        // pending text run; plain bytes extend it and bare line ends are
        // ignored inside it, as RTF readers ignore them.
        let continues_run = match b {
            b'{' | b'}' => false,
            b'\\' => input.get(i + 1) == Some(&b'\''),
            _ => true,
        };
        if !continues_run {
            flush_pending(
                &mut pending,
                &fonts,
                document,
                &groups,
                &mut fields,
                &mut text,
            );
        }
        match b {
            b'{' => {
                // An ordinary group shares its parent's destination, so a
                // waiting half stays pending across it (third re-review of
                // s1-4); it is resolved where the destination changes.
                let parent = groups.last().unwrap();
                groups.push(Group {
                    sink: parent.sink,
                    uc: parent.uc,
                    font: parent.font,
                    opens_field: false,
                    fresh: true,
                });
                skip_fallback = 0;
                i += 1;
            }
            b'}' => {
                skip_fallback = 0;
                // Closing a destination (a group whose sink differs from its
                // parent's) resolves a waiting half inside it, so it never
                // leaks out; closing an ordinary group keeps it pending.
                let closes_destination = groups.len() > 1
                    && groups[groups.len() - 1].sink != groups[groups.len() - 2].sink;
                if closes_destination {
                    resolve_lone_high(&mut pending_high, &groups, &mut fields, &mut text);
                }
                if groups.len() > 1 {
                    let g = groups.pop().unwrap();
                    if g.opens_field {
                        if let Some(f) = fields.pop() {
                            let link = render_field(&f);
                            push_str(&link, &groups, &mut fields, &mut text);
                        }
                    }
                }
                i += 1;
            }
            b'\\' => {
                let next = input.get(i + 1).copied();
                match next {
                    Some(b'\n') | Some(b'\r') => {
                        i += 2;
                        if next == Some(b'\r') && input.get(i) == Some(&b'\n') {
                            i += 1;
                        }
                        // A control symbol is one fallback item while a
                        // fallback is pending (third re-review of s1-4).
                        if skip_fallback > 0 {
                            skip_fallback -= 1;
                            continue;
                        }
                        // A paragraph break produces text: a waiting half is
                        // resolved before it, as `\par` does.
                        resolve_lone_high(&mut pending_high, &groups, &mut fields, &mut text);
                        paragraph_break(&groups, &mut fields, &mut text);
                    }
                    Some(b'\'') => {
                        let hi = input.get(i + 2).copied().and_then(hex_val);
                        let lo = input.get(i + 3).copied().and_then(hex_val);
                        if let (Some(hi), Some(lo)) = (hi, lo) {
                            if skip_fallback > 0 {
                                skip_fallback -= 1;
                            } else {
                                resolve_lone_high(
                                    &mut pending_high,
                                    &groups,
                                    &mut fields,
                                    &mut text,
                                );
                                pending.push((hi << 4) | lo);
                            }
                            i += 4;
                        } else {
                            i += 2;
                        }
                    }
                    Some(c @ (b'\\' | b'{' | b'}')) => {
                        if skip_fallback > 0 {
                            skip_fallback -= 1;
                        } else {
                            resolve_lone_high(&mut pending_high, &groups, &mut fields, &mut text);
                            push(c as char, &groups, &mut fields, &mut text);
                        }
                        i += 2;
                    }
                    // Control symbols that stand for a character are
                    // fallback items too (review finding s1-1).
                    Some(c @ (b'~' | b'-' | b'_')) => {
                        if skip_fallback > 0 {
                            skip_fallback -= 1;
                        } else {
                            resolve_lone_high(&mut pending_high, &groups, &mut fields, &mut text);
                            match c {
                                b'~' => push('\u{A0}', &groups, &mut fields, &mut text),
                                // Optional hyphen: invisible unless the line breaks.
                                b'-' => {}
                                _ => push('\u{2011}', &groups, &mut fields, &mut text),
                            }
                        }
                        i += 2;
                    }
                    Some(b'*') => {
                        // `{\*\word …}`: a destination readers may ignore.
                        // Keep `\fldinst` (hyperlink targets); skip the rest.
                        skip_fallback = 0;
                        let start = i + 2;
                        let mut j = start;
                        if j < n && input[j] == b'\\' {
                            j += 1;
                            let ws = j;
                            while j < n && input[j].is_ascii_alphabetic() {
                                j += 1;
                            }
                            let word = &input[ws..j];
                            // Entering a destination: a waiting half is
                            // resolved in the enclosing one first.
                            resolve_lone_high(&mut pending_high, &groups, &mut fields, &mut text);
                            if let Some(g) = groups.last_mut() {
                                g.fresh = false;
                                if word == b"fldinst" && fields.last().is_some() {
                                    g.sink = Sink::FieldInstruction;
                                } else {
                                    g.sink = Sink::Skip;
                                }
                            }
                            // Swallow the control word's parameter and delimiter.
                            while j < n && (input[j] == b'-' || input[j].is_ascii_digit()) {
                                j += 1;
                            }
                            if j < n && input[j] == b' ' {
                                j += 1;
                            }
                            i = j;
                        } else {
                            i += 2;
                        }
                    }
                    Some(c) if c.is_ascii_alphabetic() => {
                        let start = i + 1;
                        let mut j = start;
                        while j < n && input[j].is_ascii_alphabetic() {
                            j += 1;
                        }
                        let word = &input[start..j];
                        let mut k = j;
                        if k < n && input[k] == b'-' {
                            k += 1;
                        }
                        while k < n && input[k].is_ascii_digit() {
                            k += 1;
                        }
                        let param: Option<i64> = if k > j {
                            std::str::from_utf8(&input[j..k])
                                .ok()
                                .and_then(|s| s.parse().ok())
                        } else {
                            None
                        };
                        let mut end = k;
                        if end < n && input[end] == b' ' {
                            end += 1;
                        }

                        // `\binN` is followed by N raw bytes that belong to
                        // it; skip them with the control word.
                        if word == b"bin" {
                            let len = param.unwrap_or(0).max(0) as usize;
                            end = (end + len).min(n);
                        }
                        // Pending `\uN` fallback (review finding s1-1,
                        // repaired twice after re-review): every control
                        // word, `\binN` with its payload and `\ucN`
                        // included, is one fallback item, swallowed and not
                        // applied, and the count carries on. Only `\u` is
                        // exempt: it starts a new Unicode sequence.
                        if skip_fallback > 0 && word != b"u" {
                            skip_fallback -= 1;
                            i = end;
                            continue;
                        }

                        let fresh = groups.last().map(|g| g.fresh).unwrap_or(false);
                        if let Some(g) = groups.last_mut() {
                            g.fresh = false;
                        }

                        if fresh && SKIPPED_DESTINATIONS.contains(&word) {
                            resolve_lone_high(&mut pending_high, &groups, &mut fields, &mut text);
                            if let Some(g) = groups.last_mut() {
                                g.sink = Sink::Skip;
                            }
                        } else if fresh && word == b"field" {
                            resolve_lone_high(&mut pending_high, &groups, &mut fields, &mut text);
                            fields.push(Field::default());
                            if let Some(g) = groups.last_mut() {
                                g.opens_field = true;
                                // Text directly inside `\field` (outside
                                // fldinst/fldrslt) is not shown.
                                g.sink = Sink::Skip;
                            }
                        } else if fresh && word == b"fldrslt" && fields.last().is_some() {
                            resolve_lone_high(&mut pending_high, &groups, &mut fields, &mut text);
                            if let Some(g) = groups.last_mut() {
                                g.sink = Sink::FieldResult;
                            }
                        } else {
                            // Words that produce text resolve a waiting
                            // surrogate half first; formatting words do not.
                            if matches!(
                                word,
                                b"par"
                                    | b"line"
                                    | b"tab"
                                    | b"emdash"
                                    | b"endash"
                                    | b"lquote"
                                    | b"rquote"
                                    | b"ldblquote"
                                    | b"rdblquote"
                                    | b"bullet"
                            ) {
                                resolve_lone_high(
                                    &mut pending_high,
                                    &groups,
                                    &mut fields,
                                    &mut text,
                                );
                            }
                            match word {
                                b"par" => paragraph_break(&groups, &mut fields, &mut text),
                                // A Markdown hard line break (backslash at
                                // the end of the line), not a bare newline,
                                // which renders as a space (review finding
                                // s1-6).
                                b"line" => push_str("\\\n", &groups, &mut fields, &mut text),
                                b"tab" => push('\t', &groups, &mut fields, &mut text),
                                b"emdash" => push('\u{2014}', &groups, &mut fields, &mut text),
                                b"endash" => push('\u{2013}', &groups, &mut fields, &mut text),
                                b"lquote" => push('\u{2018}', &groups, &mut fields, &mut text),
                                b"rquote" => push('\u{2019}', &groups, &mut fields, &mut text),
                                b"ldblquote" => push('\u{201C}', &groups, &mut fields, &mut text),
                                b"rdblquote" => push('\u{201D}', &groups, &mut fields, &mut text),
                                b"bullet" => push('\u{2022}', &groups, &mut fields, &mut text),
                                b"uc" => {
                                    if let Some(g) = groups.last_mut() {
                                        g.uc = param.unwrap_or(1).max(0) as u32;
                                    }
                                }
                                b"f" if param.is_some() => {
                                    if let Some(g) = groups.last_mut() {
                                        g.font = param;
                                    }
                                }
                                b"plain" => {
                                    if let Some(g) = groups.last_mut() {
                                        g.font = default_font;
                                    }
                                }
                                b"u" => {
                                    if let Some(p) = param {
                                        let unit = (if p < 0 { p + 65536 } else { p }) as u32;
                                        match (pending_high.take(), unit) {
                                            (Some(hi), 0xDC00..=0xDFFF) => {
                                                let cp = 0x10000
                                                    + ((hi as u32 - 0xD800) << 10)
                                                    + (unit - 0xDC00);
                                                push(
                                                    char::from_u32(cp).unwrap_or('\u{FFFD}'),
                                                    &groups,
                                                    &mut fields,
                                                    &mut text,
                                                );
                                            }
                                            (high, 0xD800..=0xDBFF) => {
                                                if high.is_some() {
                                                    push(
                                                        '\u{FFFD}',
                                                        &groups,
                                                        &mut fields,
                                                        &mut text,
                                                    );
                                                }
                                                pending_high = Some(unit as u16);
                                            }
                                            (high, _) => {
                                                if high.is_some() {
                                                    push(
                                                        '\u{FFFD}',
                                                        &groups,
                                                        &mut fields,
                                                        &mut text,
                                                    );
                                                }
                                                push(
                                                    char::from_u32(unit).unwrap_or('\u{FFFD}'),
                                                    &groups,
                                                    &mut fields,
                                                    &mut text,
                                                );
                                            }
                                        }
                                        skip_fallback = groups.last().map(|g| g.uc).unwrap_or(1);
                                    }
                                }
                                _ => {}
                            }
                        }
                        i = end;
                    }
                    Some(_) => {
                        // Any other control symbol is one fallback item when
                        // a fallback is pending; otherwise it has no text.
                        skip_fallback = skip_fallback.saturating_sub(1);
                        i += 2;
                    }
                    None => {
                        i += 1;
                    }
                }
            }
            b'\r' | b'\n' => {
                i += 1;
            }
            _ => {
                if skip_fallback > 0 {
                    skip_fallback -= 1;
                } else {
                    // Literal text joins the pending run so a multi-byte
                    // character split between an escape and a literal byte
                    // decodes whole.
                    resolve_lone_high(&mut pending_high, &groups, &mut fields, &mut text);
                    pending.push(b);
                }
                i += 1;
            }
        }
    }
    flush_pending(
        &mut pending,
        &fonts,
        document,
        &groups,
        &mut fields,
        &mut text,
    );
    if pending_high.is_some() {
        push('\u{FFFD}', &groups, &mut fields, &mut text);
    }

    // A truncated document ends inside open groups. Treat the end of input
    // as closing every brace so a field cut off mid-way still yields its
    // result text and target (review finding s1-2).
    while groups.len() > 1 {
        let g = groups.pop().unwrap();
        if g.opens_field {
            if let Some(f) = fields.pop() {
                let link = render_field(&f);
                push_str(&link, &groups, &mut fields, &mut text);
            }
        }
    }
    while let Some(f) = fields.pop() {
        let link = render_field(&f);
        push_str(&link, &groups, &mut fields, &mut text);
    }

    finish(text)
}

/// The document text as plain text, not Markdown: the same extraction with
/// the Markdown escapes and hard-break markers taken back out. Used for
/// comment and footnote bodies, which the model stores as plain text.
pub fn rtf_to_plain_text(input: &[u8]) -> String {
    let markdown = rtf_to_plain_markdown(input);
    let mut out = String::with_capacity(markdown.len());
    let mut chars = markdown.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.peek() {
                Some(&next) if next.is_ascii_punctuation() => {
                    out.push(next);
                    chars.next();
                }
                Some('\n') => {
                    // A hard-break marker: the newline itself stays.
                }
                _ => out.push(c),
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Literal-text serialisation for Pandoc Markdown: every ASCII punctuation
/// character is backslash-escaped. No allow-list (review finding plan-3
/// R2-3, four rounds): Pandoc reads a backslash before any ASCII
/// punctuation as that character, so this is complete, and CommonMark (the
/// editor's parser) agrees.
fn push_escaped(c: char, out: &mut String) {
    if c.is_ascii_punctuation() {
        out.push('\\');
    }
    out.push(c);
}

/// Appends already-escaped (or deliberately raw) text to the current sink.
fn push_str(s: &str, groups: &[Group], fields: &mut [Field], text: &mut String) {
    match groups.last().map(|g| g.sink).unwrap_or(Sink::Body) {
        Sink::Body => text.push_str(s),
        Sink::Skip => {}
        Sink::FieldInstruction => {
            if let Some(f) = fields.last_mut() {
                f.instruction.push_str(s);
            }
        }
        Sink::FieldResult => {
            if let Some(f) = fields.last_mut() {
                f.result.push_str(s);
            }
        }
    }
}

fn paragraph_break(groups: &[Group], fields: &mut [Field], text: &mut String) {
    push_str("\n\n", groups, fields, text);
}

/// Removes hard-break markers (an odd run of trailing backslashes) and
/// blank lines from the end of `lines`; an even run is escaped literal
/// backslashes and stays. A break with nothing after it would read as a
/// literal backslash (review finding s1-6, re-review).
fn strip_trailing_breaks(lines: &mut Vec<String>) -> usize {
    let mut removed = 0;
    while let Some(last) = lines.last_mut() {
        let trimmed_len = last.trim_end().len();
        last.truncate(trimmed_len);
        let trailing = last.chars().rev().take_while(|c| *c == '\\').count();
        if trailing % 2 == 1 {
            last.pop();
            removed += 1;
            if last.trim().is_empty() {
                lines.pop();
            }
            continue;
        }
        if last.trim().is_empty() {
            lines.pop();
            continue;
        }
        break;
    }
    removed
}

/// `HYPERLINK "url"` + result text → a Markdown link; any other field →
/// its result text. A break at the very end of the result is moved after
/// the link so the link syntax stays intact and the break still happens
/// when text follows (review finding s1-6, second re-review); the
/// paragraph-end cleanup removes it when nothing follows.
fn render_field(f: &Field) -> String {
    let inst = f.instruction.trim();
    let mut result_lines: Vec<String> = f
        .result
        .trim()
        .lines()
        .map(|l| l.trim().to_string())
        .collect();
    let breaks = strip_trailing_breaks(&mut result_lines);
    let result = result_lines.join("\n");
    let result = result.as_str();
    let mut rendered = render_link(inst, result);
    // Every break that ended the link text is re-emitted after the link
    // (third re-review of s1-6: two breaks are two breaks); the paragraph
    // cleanup drops them when nothing follows.
    if !result.is_empty() {
        for _ in 0..breaks {
            rendered.push_str("\\\n");
        }
    }
    rendered
}

/// Marks a Scrivener anchor (`scrivcmt://`, `scrivfn://`, `scrivannot://`)
/// the way the Pandoc filter does, so the importer resolves both paths the
/// same way (review finding s4-5, seventh repair):
/// U+E000 scheme ":" id U+E001 text U+E002.
pub const ANCHOR_OPEN: char = '\u{E000}';
pub const ANCHOR_SEP: char = '\u{E001}';
pub const ANCHOR_CLOSE: char = '\u{E002}';
pub const ANCHOR_SCHEMES: [&str; 3] = ["scrivcmt", "scrivfn", "scrivannot"];

fn render_link(inst: &str, result: &str) -> String {
    let url = inst
        .strip_prefix("HYPERLINK")
        .map(|rest| rest.trim().trim_matches('"').trim())
        .filter(|u| !u.is_empty() && !u.contains(char::is_whitespace));
    if let Some((scheme, id)) = url.and_then(|u| u.split_once("://")) {
        if ANCHOR_SCHEMES.contains(&scheme) && !id.is_empty() {
            return format!("{ANCHOR_OPEN}{scheme}:{id}{ANCHOR_SEP}{result}{ANCHOR_CLOSE}");
        }
    }
    match url {
        Some(url) if !result.is_empty() => {
            format!("[{result}]({})", markdown_destination(url))
        }
        Some(url) => format!("<{}>", markdown_destination(url)),
        None => result.to_string(),
    }
}

/// Percent-encodes the characters that end or break a Markdown link
/// destination (review finding s1-5: a `)` in the target truncated the
/// link and leaked text into the prose). Anything already percent-encoded
/// is left alone; `scrivcmt://<UUID>` targets never contain these.
fn markdown_destination(url: &str) -> String {
    let mut out = String::with_capacity(url.len());
    for c in url.chars() {
        match c {
            '(' => out.push_str("%28"),
            ')' => out.push_str("%29"),
            '<' => out.push_str("%3C"),
            '>' => out.push_str("%3E"),
            ' ' => out.push_str("%20"),
            '\\' => out.push_str("%5C"),
            _ => out.push(c),
        }
    }
    out
}

/// Collapse runs of blank lines, trim paragraph edges, drop empty output.
/// A hard line break at the very end of a paragraph has nothing to break
/// before and would read as a literal backslash, so it is dropped.
fn finish(text: String) -> String {
    let mut paragraphs: Vec<String> = Vec::new();
    for para in text.split("\n\n") {
        let mut lines: Vec<String> = para
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
        strip_trailing_breaks(&mut lines);
        if !lines.is_empty() {
            paragraphs.push(lines.join("\n"));
        }
    }
    paragraphs.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(s: &str) -> String {
        rtf_to_plain_markdown(s.as_bytes())
    }

    #[test]
    fn cocoa_document_becomes_plain_paragraphs() {
        let src = "{\\rtf1\\ansi\\ansicpg1252\\cocoartf2509\n\
{\\fonttbl\\f0\\froman\\fcharset0 TimesNewRomanPSMT;}\n\
{\\colortbl;\\red255\\green255\\blue255;}\n\
{\\*\\expandedcolortbl;;}\n\
\\pard\\tx360\\fi360\\sl480\\slmult1\\partightenfactor0\n\
\n\
\\f0\\fs24 \\cf0 Ian was Zo\\'eb's first.\\tab Caf\\uc1\\u233\\'e9.\\\n\
Second paragraph with \\'93quotes\\'94.\\\n\
}";
        assert_eq!(
            plain(src),
            "Ian was Zoë\\'s first\\.\tCafé\\.\n\nSecond paragraph with \u{201C}quotes\u{201D}\\."
        );
    }

    #[test]
    fn hyperlink_field_becomes_a_markdown_link() {
        let src = "{\\rtf1 see {\\field{\\*\\fldinst{HYPERLINK \"scrivcmt://ABC-123\"}}{\\fldrslt this bit}} here}";
        assert_eq!(
            plain(src),
            "see \u{E000}scrivcmt:ABC-123\u{E001}this bit\u{E002} here"
        );
    }

    /// Review finding s1-2: a document cut off inside a hyperlink field
    /// still yields the field's text and target.
    #[test]
    fn truncated_field_keeps_its_text() {
        assert_eq!(
            plain("{\\rtf1 before {\\field{\\*\\fldinst{HYPERLINK \"scrivcmt://X\"}}{\\fldrslt essential text"),
            "before \u{E000}scrivcmt:X\u{E001}essential text\u{E002}"
        );
        // Cut off before the result: the target alone is kept.
        assert_eq!(
            plain("{\\rtf1 before {\\field{\\*\\fldinst{HYPERLINK \"scrivcmt://X\"}}"),
            "before \u{E000}scrivcmt:X\u{E001}\u{E002}"
        );
    }

    /// Review finding s1-5: a `)` or `<` in a hyperlink target must not end
    /// the Markdown destination early.
    #[test]
    fn link_targets_with_brackets_stay_whole() {
        assert_eq!(
            plain("{\\rtf1 {\\field{\\*\\fldinst{HYPERLINK \"https://example.com/a)b(c\"}}{\\fldrslt click}}}"),
            "[click](https://example.com/a%29b%28c)"
        );
        assert_eq!(
            plain("{\\rtf1 {\\field{\\*\\fldinst{HYPERLINK \"https://x.y/<z>\"}}}}"),
            "<https://x.y/%3Cz%3E>"
        );
        // The anchors slice 4 relies on are untouched.
        assert_eq!(
            plain(
                "{\\rtf1 {\\field{\\*\\fldinst{HYPERLINK \"scrivcmt://ABC-123\"}}{\\fldrslt t}}}"
            ),
            "\u{E000}scrivcmt:ABC-123\u{E001}t\u{E002}"
        );
    }

    /// Pandoc's rendering of Markdown as plain text, when installed.
    fn rendered(markdown: &str) -> Option<String> {
        use std::io::Write;
        use std::process::{Command, Stdio};
        let mut child = Command::new("pandoc")
            .args(["-f", "markdown", "-t", "plain"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        child.stdin.take()?.write_all(markdown.as_bytes()).ok()?;
        let out = child.wait_with_output().ok()?;
        Some(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
    }

    /// Review finding s1-6: `\line` is a hard line break, which must still be
    /// a line break once the Markdown is rendered.
    #[test]
    fn par_and_line_are_distinguished() {
        let md = plain("{\\rtf1 a\\line b\\par c}");
        assert_eq!(md, "a\\\nb\n\nc");
        if let Some(text) = rendered(&md) {
            assert_eq!(text, "a\nb\n\nc");
        } else {
            eprintln!("Skipping Pandoc check: pandoc not installed");
        }
        // A trailing break before a paragraph end is dropped, an escaped
        // backslash at a paragraph end is kept.
        assert_eq!(plain("{\\rtf1 a\\line\\par b}"), "a\n\nb");
        assert_eq!(plain("{\\rtf1 a\\\\\\par b}"), "a\\\\\n\nb");
        // Re-review: a run of trailing breaks goes entirely, and a break at
        // the end of link text does not break the link.
        assert_eq!(plain("{\\rtf1 a\\line\\line\\par b}"), "a\n\nb");
        assert_eq!(plain("{\\rtf1 a\\line\\line}"), "a");
        assert_eq!(
            plain("{\\rtf1 {\\field{\\*\\fldinst{HYPERLINK \"scrivcmt://ABC-123\"}}{\\fldrslt note\\line}}}"),
            "\u{E000}scrivcmt:ABC-123\u{E001}note\u{E002}"
        );
        // Second re-review: when text follows the field, the break at the
        // end of the link text still breaks the line, outside the link.
        let md = plain("{\\rtf1 {\\field{\\*\\fldinst{HYPERLINK \"https://example.com\"}}{\\fldrslt note\\line}}second}");
        assert_eq!(md, "[note](https://example.com)\\\nsecond");
        if let Some(text) = rendered(&md) {
            assert_eq!(text, "note\nsecond");
        }
        // Third re-review: two breaks at the end of link text are two breaks.
        let md = plain("{\\rtf1 {\\field{\\*\\fldinst{HYPERLINK \"https://example.com\"}}{\\fldrslt note\\line\\line}}second}");
        assert_eq!(md, "[note](https://example.com)\\\n\\\nsecond");
        if let Some(html) = rendered_html(&md) {
            assert_eq!(html.matches("<br").count(), 2, "{html}");
        }
        // The same two breaks outside a field, for comparison.
        let md = plain("{\\rtf1 note\\line\\line second}");
        if let Some(html) = rendered_html(&md) {
            assert_eq!(html.matches("<br").count(), 2, "{html}");
        }
    }

    /// Pandoc's HTML rendering of Markdown, when installed.
    fn rendered_html(markdown: &str) -> Option<String> {
        use std::io::Write;
        use std::process::{Command, Stdio};
        let mut child = Command::new("pandoc")
            .args(["-f", "markdown", "-t", "html"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        child.stdin.take()?.write_all(markdown.as_bytes()).ok()?;
        let out = child.wait_with_output().ok()?;
        Some(String::from_utf8_lossy(&out.stdout).to_string())
    }

    /// Review finding s1-3: hex escapes and raw high bytes follow the
    /// declared code page.
    #[test]
    fn hex_escapes_follow_the_declared_code_page() {
        assert_eq!(
            plain("{\\rtf1\\ansi\\ansicpg1251 \\'cf\\'f0\\'e8\\'e2\\'e5\\'f2}"),
            "Привет"
        );
        assert_eq!(plain("{\\rtf1\\ansi\\ansicpg932 \\'82\\'a0}"), "\u{3042}");
        assert_eq!(plain("{\\rtf1\\mac\\ansicpg10000 caf\\'8e}"), "café");
        // No declaration: Windows-1252, as before.
        assert_eq!(plain("{\\rtf1 it\\'92s}"), "it\u{2019}s");
        // A raw high byte outside an escape is code-page text too.
        assert_eq!(
            rtf_to_plain_markdown(b"{\\rtf1\\ansi\\ansicpg1251 \xcf\xf0}"),
            "Пр"
        );
    }

    /// Re-review of s1-3: a font's own encoding wins over the document's,
    /// and a two-byte character may end in a literal byte.
    #[test]
    fn font_encoding_and_mixed_runs() {
        assert_eq!(
            plain("{\\rtf1\\ansi\\ansicpg1251{\\fonttbl{\\f0\\fcharset0\\cpg1252 Arial;}{\\f1\\fcharset204 Arial;}}\\f0 caf\\'e9 \\f1 \\'cf}"),
            "café П"
        );
        // The font is scoped to its group.
        assert_eq!(
            plain(
                "{\\rtf1\\ansi\\ansicpg1251{\\fonttbl{\\f0\\fcharset0 Arial;}}{\\f0 \\'e9}\\'e9}"
            ),
            "éй"
        );
        // CP932 `ア` is the escaped byte 0x83 followed by a literal `A`.
        assert_eq!(plain("{\\rtf1\\ansi\\ansicpg932 \\'83A!}"), "\u{30A2}\\!");
        // Second re-review: the default font is the font before any `\fN`,
        // and `\plain` goes back to it.
        assert_eq!(
            plain("{\\rtf1\\ansi\\ansicpg1251\\deff0{\\fonttbl{\\f0\\fcharset0 A;}{\\f1\\fcharset204 B;}} caf\\'e9 \\f1\\plain caf\\'e9}"),
            "café café"
        );
        // A bare line end inside the run is ignored, as readers ignore it.
        assert_eq!(plain("{\\rtf1\\ansi\\ansicpg932 \\'83\nA}"), "\u{30A2}");
    }

    /// Review finding s1-4: a UTF-16 surrogate pair is one character; a
    /// lone half shows U+FFFD instead of vanishing.
    #[test]
    fn surrogate_pairs_combine() {
        let src = format!(
            "{{\\rtf1\\ansi\\uc1 before \\u{} ?\\u{} ? after}}",
            -10179, -8704
        );
        assert_eq!(plain(&src), "before \u{1F600} after");
        let lone = format!("{{\\rtf1\\ansi\\uc1 x\\u{} ?y}}", -10179);
        assert_eq!(plain(&lone), "x\u{FFFD}y");
        let trailing = format!("{{\\rtf1\\ansi\\uc1 x\\u{} ?}}", -10179);
        assert_eq!(plain(&trailing), "x\u{FFFD}");

        // Re-review: ignored syntax between the halves (a bare line end, a
        // formatting word) does not break the pair.
        let split = format!(
            "{{\\rtf1\\ansi\\uc1 before \\u{} ?\r\n\\u{} ? after}}",
            -10179, -8704
        );
        assert_eq!(plain(&split), "before \u{1F600} after");
        let split = format!(
            "{{\\rtf1\\ansi\\uc1 before \\u{} ?\\b \\u{} ? after}}",
            -10179, -8704
        );
        assert_eq!(plain(&split), "before \u{1F600} after");
        // A lone half stays in its own destination: skipped there, kept
        // in a field result, never leaked into the body.
        let skipped = format!("{{\\rtf1\\uc1 {{\\*\\unknown \\u{} ?}}body}}", -10179);
        assert_eq!(plain(&skipped), "body");
        let in_field = format!(
            "{{\\rtf1\\uc1 {{\\field{{\\*\\fldinst{{HYPERLINK \"u\"}}}}{{\\fldrslt \\u{} ?}}}}}}",
            -10179
        );
        assert_eq!(plain(&in_field), "[\u{FFFD}](u)");

        // Second re-review: a half waiting in the body is resolved there
        // before a field or a skipped group opens, and before a
        // backslash-newline paragraph break.
        let before_field = format!(
            "{{\\rtf1\\uc1 x\\u{} ?{{\\field{{\\*\\fldinst{{HYPERLINK \"u\"}}}}{{\\fldrslt label}}}}y}}",
            -10179
        );
        assert_eq!(plain(&before_field), "x\u{FFFD}[label](u)y");
        let before_skipped = format!("{{\\rtf1\\uc1 x\\u{} ?{{\\*\\unknown hidden}}y}}", -10179);
        assert_eq!(plain(&before_skipped), "x\u{FFFD}y");
        let before_par = format!("{{\\rtf1\\uc1 x\\u{} ?\\\ny}}", -10179);
        assert_eq!(plain(&before_par), "x\u{FFFD}\n\ny");
        let across_par = format!("{{\\rtf1\\uc1 x\\u{} ?\\\n\\u{} ?y}}", -10179, -8704);
        assert_eq!(plain(&across_par), "x\u{FFFD}\n\n\u{FFFD}y");

        // Third re-review: an ordinary group shares the destination, so a
        // pair split by it still combines; a backslash-newline in the
        // fallback slot is a fallback item, not a paragraph break.
        let in_group = format!("{{\\rtf1\\uc1 x\\u{} ?{{\\u{} ?}}y}}", -10179, -8704);
        assert_eq!(plain(&in_group), "x\u{1F600}y");
        let in_bold_group = format!("{{\\rtf1\\uc1 x\\u{} ?{{\\b \\u{} ?}}y}}", -10179, -8704);
        assert_eq!(plain(&in_bold_group), "x\u{1F600}y");
        let in_field_group = format!(
            "{{\\rtf1\\uc1 {{\\field{{\\*\\fldinst{{HYPERLINK \"u\"}}}}{{\\fldrslt \\u{} ?{{\\u{} ?}}}}}}}}",
            -10179, -8704
        );
        assert_eq!(plain(&in_field_group), "[\u{1F600}](u)");
        let newline_fallback = format!("{{\\rtf1\\uc1 x\\u{}\\\n\\u{} ?y}}", -10179, -8704);
        assert_eq!(plain(&newline_fallback), "x\u{1F600}y");
    }

    #[test]
    fn unicode_fallbacks_are_skipped() {
        assert_eq!(plain("{\\rtf1\\uc2 x\\u8217\\'92\\'92y}"), "x\u{2019}y");
        assert_eq!(plain("{\\rtf1\\uc0 x\\u8217 y}"), "x\u{2019}y");
    }

    /// Review finding s1-1: a character control word after `\uN` is the
    /// fallback and is swallowed; a formatting word ends the run so the
    /// letter after it survives.
    #[test]
    fn character_control_word_is_the_fallback_and_letters_survive() {
        let src = format!("{{\\rtf1\\ansi\\uc1 a\\u{}\\endash word}}", 8211);
        assert_eq!(plain(&src), "a\u{2013}word");
        assert_eq!(plain("{\\rtf1\\ansi\\uc1 a\\u233\\b b}"), "aéb");
        assert_eq!(plain("{\\rtf1\\ansi\\uc1 a\\u233\\~b}"), "aéb");
        assert_eq!(plain("{\\rtf1\\ansi\\uc1 a\\u233\\tab b}"), "aéb");
        // Re-review case: `\uc2` skips the formatting word and the letter.
        assert_eq!(
            plain("{\\rtf1\\ansi {\\uc2 caf\\u233\\b e} noir}"),
            "café noir"
        );
        // `\u` after `\u` starts a new character, never a fallback (the
        // `b` after the second one is its fallback and goes).
        assert_eq!(plain("{\\rtf1\\ansi\\uc1 a\\u233\\u235 b}"), "aéë");
        // Second re-review: `\ucN` and `\binN` with its payload are each
        // one fallback item; the text after them survives.
        assert_eq!(plain("{\\rtf1\\ansi\\uc1 a\\u233\\uc0 word}"), "aéword");
        assert_eq!(plain("{\\rtf1\\ansi\\uc1 a\\u233\\bin3 XYZword}"), "aéword");
        assert_eq!(
            plain("{\\rtf1\\ansi\\uc1 a\\u233\\bin8 {\\*\\foo word}"),
            "aéword"
        );
        // Outside a fallback the payload is skipped and nothing else is.
        assert_eq!(plain("{\\rtf1 a\\bin3 XYZb}"), "ab");
    }

    #[test]
    fn skipped_destinations_leave_no_text() {
        assert_eq!(
            plain("{\\rtf1{\\info{\\title Secret}}{\\*\\listtable junk}Body}"),
            "Body"
        );
    }

    #[test]
    fn escaped_braces_and_backslashes_are_literal() {
        assert_eq!(plain("{\\rtf1 a\\{b\\}c\\\\d}"), "a\\{b\\}c\\\\d");
    }

    #[test]
    fn markdown_syntax_in_prose_is_escaped() {
        // Review finding plan-3 R2-3.
        assert_eq!(
            plain("{\\rtf1 Keep *these stars* and <redacted> and [this] literally.}"),
            "Keep \\*these stars\\* and \\<redacted\\> and \\[this\\] literally\\."
        );
        assert_eq!(
            plain("{\\rtf1 - not a list\\par 1. not numbered\\par # not a heading}"),
            "\\- not a list\n\n1\\. not numbered\n\n\\# not a heading"
        );
        // Every ASCII punctuation character is escaped: title blocks,
        // definition lists, entities, math and smart punctuation included.
        assert_eq!(
            plain("{\\rtf1 % 80% gone\\par Term\\par : definition\\par \"hi\" -- paused... &copy; $x$}"),
            "\\% 80\\% gone\n\nTerm\n\n\\: definition\n\n\\\"hi\\\" \\-\\- paused\\.\\.\\. \\&copy\\; \\$x\\$"
        );
        // Link text is escaped, the target is not.
        assert_eq!(
            plain("{\\rtf1 {\\field{\\*\\fldinst{HYPERLINK \"scrivcmt://X_1\"}}{\\fldrslt a*b}}}"),
            "\u{E000}scrivcmt:X_1\u{E001}a\\*b\u{E002}"
        );
    }

    #[test]
    fn font_and_colour_tables_and_pictures_leave_no_text() {
        // Review finding plan-3 R2-4: an actual sample header shape.
        let src = "{\\rtf1\\ansi\\ansicpg1252\\cocoartf2509\n\
{\\fonttbl\\f0\\fnil\\fcharset0 Palatino-Roman;\\f1\\froman\\fcharset0 TimesNewRomanPSMT;}\n\
{\\colortbl;\\red255\\green255\\blue255;\\red0\\green0\\blue0;}\n\
{\\*\\expandedcolortbl;;\\cssrgb\\c0\\c0\\c0;}\n\
{\\pict\\picw100\\pich100 0123456789abcdef}\n\
\\pard\\f0\\fs24 Only this.}";
        assert_eq!(plain(src), "Only this\\.");
    }

    #[test]
    fn empty_input_gives_empty_output() {
        assert_eq!(plain("{\\rtf1 }"), "");
        assert_eq!(plain(""), "");
    }
}
