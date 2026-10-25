//! Plain-text fallback for RTF that Pandoc cannot read (PLAN_SCRIVENER_FIDELITY
//! slice 1).
//!
//! When Pandoc rejects a document, the import must not die and must not
//! drop the document: this extractor yields the text as Markdown paragraphs
//! with no formatting, and the caller names the document in the warnings.
//! Hyperlink fields survive as Markdown links so the `scrivcmt://` anchors
//! slice 4 relies on are kept (review finding plan-3 R1-5).

use super::rtf_prepare::is_character_control_word;

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

fn cp1252(byte: u8) -> Option<char> {
    const HIGH: [u32; 32] = [
        0x20AC, 0x0081, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160,
        0x2039, 0x0152, 0x008D, 0x017D, 0x008F, 0x0090, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022,
        0x2013, 0x2014, 0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0x009D, 0x017E, 0x0178,
    ];
    if byte < 0x80 {
        Some(byte as char)
    } else if byte < 0xA0 {
        char::from_u32(HIGH[(byte - 0x80) as usize])
    } else {
        char::from_u32(byte as u32)
    }
}

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
    /// Set on the `{` of a `\field` group so its `}` can emit the link.
    opens_field: bool,
    /// True until the first control word of the group has been seen
    /// (destination detection happens there).
    fresh: bool,
}

/// Extracts the document text of `input` as Markdown paragraphs. Never fails.
pub fn rtf_to_plain_markdown(input: &[u8]) -> String {
    let mut text = String::new();
    let mut groups: Vec<Group> = vec![Group {
        sink: Sink::Body,
        uc: 1,
        opens_field: false,
        fresh: false,
    }];
    let mut fields: Vec<Field> = Vec::new();
    let mut skip_fallback: u32 = 0;

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

    let n = input.len();
    let mut i = 0;
    while i < n {
        let b = input[i];
        match b {
            b'{' => {
                let parent = groups.last().unwrap();
                groups.push(Group {
                    sink: parent.sink,
                    uc: parent.uc,
                    opens_field: false,
                    fresh: true,
                });
                skip_fallback = 0;
                i += 1;
            }
            b'}' => {
                skip_fallback = 0;
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
                        paragraph_break(&groups, &mut fields, &mut text);
                        i += 2;
                        if next == Some(b'\r') && input.get(i) == Some(&b'\n') {
                            i += 1;
                        }
                    }
                    Some(b'\'') => {
                        let hi = input.get(i + 2).copied().and_then(hex_val);
                        let lo = input.get(i + 3).copied().and_then(hex_val);
                        if let (Some(hi), Some(lo)) = (hi, lo) {
                            if skip_fallback > 0 {
                                skip_fallback -= 1;
                            } else if let Some(c) = cp1252((hi << 4) | lo) {
                                push(c, &groups, &mut fields, &mut text);
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

                        // Pending `\uN` fallback (review finding s1-1): a
                        // character control word is one fallback item and
                        // is swallowed; any other control word ends the run.
                        if skip_fallback > 0 {
                            if is_character_control_word(word) {
                                skip_fallback -= 1;
                                i = end;
                                continue;
                            }
                            skip_fallback = 0;
                        }

                        let fresh = groups.last().map(|g| g.fresh).unwrap_or(false);
                        if let Some(g) = groups.last_mut() {
                            g.fresh = false;
                        }

                        if fresh && SKIPPED_DESTINATIONS.contains(&word) {
                            if let Some(g) = groups.last_mut() {
                                g.sink = Sink::Skip;
                            }
                        } else if fresh && word == b"field" {
                            fields.push(Field::default());
                            if let Some(g) = groups.last_mut() {
                                g.opens_field = true;
                                // Text directly inside `\field` (outside
                                // fldinst/fldrslt) is not shown.
                                g.sink = Sink::Skip;
                            }
                        } else if fresh && word == b"fldrslt" && fields.last().is_some() {
                            if let Some(g) = groups.last_mut() {
                                g.sink = Sink::FieldResult;
                            }
                        } else {
                            match word {
                                b"par" | b"line" => {
                                    if word == b"par" {
                                        paragraph_break(&groups, &mut fields, &mut text);
                                    } else {
                                        push('\n', &groups, &mut fields, &mut text);
                                    }
                                }
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
                                b"u" => {
                                    if let Some(p) = param {
                                        let cp = if p < 0 { p + 65536 } else { p } as u32;
                                        if let Some(c) = char::from_u32(cp) {
                                            push(c, &groups, &mut fields, &mut text);
                                        }
                                        skip_fallback = groups.last().map(|g| g.uc).unwrap_or(1);
                                    }
                                }
                                b"bin" => {
                                    // Binary data follows; skip it wholesale.
                                    let len = param.unwrap_or(0).max(0) as usize;
                                    end = (end + len).min(n);
                                }
                                _ => {}
                            }
                        }
                        i = end;
                    }
                    Some(_) => {
                        // Other control symbols end a fallback run.
                        skip_fallback = 0;
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
                    // Bytes above 0x7F outside an escape are code-page text.
                    if let Some(c) = cp1252(b) {
                        push(c, &groups, &mut fields, &mut text);
                    }
                }
                i += 1;
            }
        }
    }

    finish(text)
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

/// `HYPERLINK "url"` + result text → a Markdown link; any other field →
/// its result text.
fn render_field(f: &Field) -> String {
    let inst = f.instruction.trim();
    let result = f.result.trim();
    let url = inst
        .strip_prefix("HYPERLINK")
        .map(|rest| rest.trim().trim_matches('"').trim())
        .filter(|u| !u.is_empty() && !u.contains(char::is_whitespace));
    match url {
        Some(url) if !result.is_empty() => format!("[{result}]({url})"),
        Some(url) => format!("<{url}>"),
        None => result.to_string(),
    }
}

/// Collapse runs of blank lines, trim paragraph edges, drop empty output.
fn finish(text: String) -> String {
    let mut paragraphs: Vec<String> = Vec::new();
    for para in text.split("\n\n") {
        let lines: Vec<String> = para
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
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
        assert_eq!(plain(src), "see [this bit](scrivcmt://ABC-123) here");
    }

    #[test]
    fn par_and_line_are_distinguished() {
        assert_eq!(plain("{\\rtf1 a\\line b\\par c}"), "a\nb\n\nc");
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
            "[a\\*b](scrivcmt://X_1)"
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
