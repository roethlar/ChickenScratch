//! Prepares Scrivener RTF for Pandoc's RTF reader (PLAN_SCRIVENER_FIDELITY
//! slice 1).
//!
//! Three things Pandoc 3.x gets wrong on Cocoa-written RTF are fixed before
//! the file reaches it; everything else passes through byte for byte:
//!
//! 1. Cocoa ends every paragraph with a backslash and a newline. The RTF
//!    spec defines that as `\par`; Pandoc reads it as a line break, so a
//!    whole document became one paragraph of hard breaks. Rewritten to
//!    `\par `.
//! 2. A hex escape followed by an apostrophe (`\'eb's`) makes Pandoc's
//!    reader fail on the whole file. Every stand-alone `\'xx` is rewritten
//!    to the equivalent `\uN?` using the file's `\ansicpg` (1252 when
//!    absent; other code pages are left alone and Pandoc decodes them).
//! 3. `\uN` carries a fallback for readers without Unicode: the next
//!    `\ucN` items (default one). Rewriting a fallback `\'e9` as a second
//!    `\uN` would double the character (review finding plan-3 R1-1), so the
//!    fallback sequence is replaced by exactly one `?` and every `\ucN` is
//!    emitted as `\uc1` so the reader and this pass agree on the count.
//!    `\ucN` is tracked per group as the spec requires.
//!
//! The pass is a small tokenizer, not a regex: it walks groups and control
//! words so a fallback that is itself an escaped brace, or a `\uN` inside a
//! hyperlink field, is handled like any other.

use std::collections::HashMap;

/// Per-group state the prepare pass tracks: the `\ucN` fallback count and
/// the active font (`\fN`), both scoped to the RTF group like the spec says.
#[derive(Clone, Copy)]
struct PrepGroup {
    uc: u32,
    font: Option<i64>,
}

/// The encoding behind an RTF `\ansicpgN` number, for the pages
/// `encoding_rs` knows (review finding s1-3: a Cyrillic document declared
/// `\ansicpg1251` was decoded as Windows-1252 into mojibake).
pub(crate) fn encoding_for_code_page(code_page: u32) -> Option<&'static encoding_rs::Encoding> {
    let label: String = match code_page {
        1250..=1258 => format!("windows-{code_page}"),
        874 => "windows-874".to_string(),
        932 => "shift_jis".to_string(),
        936 => "gbk".to_string(),
        949 => "euc-kr".to_string(),
        950 => "big5".to_string(),
        10000 => "macintosh".to_string(),
        10007 => "x-mac-cyrillic".to_string(),
        20866 => "koi8-r".to_string(),
        21866 => "koi8-u".to_string(),
        28591..=28606 => format!("iso-8859-{}", code_page - 28590),
        65001 => "utf-8".to_string(),
        _ => return None,
    };
    encoding_rs::Encoding::for_label(label.as_bytes())
}

/// The Windows code page behind an RTF `\fcharsetN` value.
fn charset_code_page(charset: u32) -> Option<u32> {
    Some(match charset {
        0 => 1252,
        77 => 10000,
        128 => 932,
        129 => 949,
        134 => 936,
        136 => 950,
        161 => 1253,
        162 => 1254,
        163 => 1258,
        177 => 1255,
        178 => 1256,
        186 => 1257,
        204 => 1251,
        222 => 874,
        238 => 1250,
        _ => return None,
    })
}

/// Per-font encodings from the `\fonttbl` group: `\cpgN` when a font
/// declares one, else the page behind its `\fcharsetN`. A font's encoding
/// overrides the document's `\ansicpgN` for text set in that font (review
/// finding s1-3, re-review: a 1251 document with a `\fcharset0` font set
/// `caf\'e9` in Windows-1252).
pub(crate) fn font_encodings(input: &[u8]) -> HashMap<i64, &'static encoding_rs::Encoding> {
    let mut map = HashMap::new();
    // The font table is the group whose first control word is `\fonttbl`;
    // found by tokenizing, so literal text never passes for it.
    let Some(pos) = find_control_word(input, b"fonttbl", None).map(|t| t.end) else {
        return map;
    };
    let n = input.len();
    let mut depth = 0i32;
    let mut font: Option<i64> = None;
    let mut i = pos;
    while i < n {
        match input[i] {
            b'{' => {
                depth += 1;
                i += 1;
            }
            b'}' => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
                i += 1;
            }
            b'\\' => {
                let mut j = i + 1;
                while j < n && input[j].is_ascii_alphabetic() {
                    j += 1;
                }
                if j == i + 1 {
                    // Control symbol or escape: two bytes, no font meaning.
                    i = (i + 2).min(n);
                    continue;
                }
                let word = &input[i + 1..j];
                let mut k = j;
                if k < n && input[k] == b'-' {
                    k += 1;
                }
                while k < n && input[k].is_ascii_digit() {
                    k += 1;
                }
                let param: Option<i64> = std::str::from_utf8(&input[j..k])
                    .ok()
                    .and_then(|s| s.parse().ok());
                match (word, param) {
                    (b"f", Some(p)) => font = Some(p),
                    (b"cpg", Some(p)) => {
                        if let (Some(f), Some(enc)) = (font, encoding_for_code_page(p as u32)) {
                            map.insert(f, enc);
                        }
                    }
                    (b"fcharset", Some(p)) => {
                        if let (Some(f), Some(enc)) = (
                            font,
                            charset_code_page(p as u32).and_then(encoding_for_code_page),
                        ) {
                            map.entry(f).or_insert(enc);
                        }
                    }
                    _ => {}
                }
                i = k;
            }
            _ => i += 1,
        }
    }
    map
}

/// The declared code page's encoding, Windows-1252 when absent or unknown.
pub(crate) fn document_encoding(input: &[u8]) -> &'static encoding_rs::Encoding {
    declared_code_page(input)
        .and_then(encoding_for_code_page)
        .unwrap_or(encoding_rs::WINDOWS_1252)
}

/// The default font `\deffN` declared at the top of the file, if any. Text
/// before any `\fN`, and text after `\plain`, is set in it (review finding
/// s1-3, second re-review).
pub(crate) fn declared_default_font(input: &[u8]) -> Option<i64> {
    find_control_word(&input[..input.len().min(1024)], b"deff", Some(1)).and_then(|t| t.param)
}

/// A control word found by tokenizing (never by substring search, which
/// mistook the literal text `\\deff1` for a declaration: review finding
/// s1-3, third re-review).
struct ControlWordHit {
    /// Byte position just after the word and its parameter.
    end: usize,
    param: Option<i64>,
}

/// Finds the first control word `name` by walking tokens: escapes (`\\`,
/// `\{`, `\}`, `\'xx`) are skipped as units and group depth is tracked.
/// `depth` restricts the match to that group depth (1 is the root group).
fn find_control_word(input: &[u8], name: &[u8], depth: Option<i32>) -> Option<ControlWordHit> {
    let n = input.len();
    let mut i = 0;
    let mut current_depth = 0i32;
    while i < n {
        match input[i] {
            b'{' => {
                current_depth += 1;
                i += 1;
            }
            b'}' => {
                current_depth -= 1;
                i += 1;
            }
            b'\\' => {
                let next = input.get(i + 1).copied();
                match next {
                    Some(b'\'') => i += 4,
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
                        if word == name && depth.is_none_or(|d| d == current_depth) {
                            let param = std::str::from_utf8(&input[j..k])
                                .ok()
                                .and_then(|s| s.parse().ok());
                            return Some(ControlWordHit { end: k, param });
                        }
                        i = k;
                    }
                    // Any control symbol or escaped character: two bytes.
                    Some(_) => i += 2,
                    None => i += 1,
                }
            }
            _ => i += 1,
        }
    }
    None
}

/// Appends decoded run text to `out` as RTF text: a backslash or brace is
/// re-escaped, a non-ASCII character becomes `\uN ?`, and a space delimiter
/// goes first when the last thing written was an undelimited control word,
/// so `\b` followed by a decoded `a` stays `\b a` and never `\ba` (review
/// finding s1-3, second re-review). The caller knows whether a control word
/// is open; a backward scan over the output mistook prose for one (third
/// re-review).
fn emit_run_text(
    out: &mut Vec<u8>,
    decoded: &str,
    after_open_control_word: bool,
    after_delimiter_space: bool,
) {
    let mut first = true;
    for c in decoded.chars() {
        if c.is_ascii() {
            if first && c == ' ' && (after_open_control_word || after_delimiter_space) {
                // A text space right after a control word's delimiter space
                // would be two spaces, and Pandoc 3.11 drops both
                // (`before\bin0\'20after` and `before\bin0 \'20after` both
                // read as `beforeafter`), so the space is written as an
                // escape instead (review finding s1-3, fourth and fifth
                // re-reviews).
                out.extend_from_slice(b"\\u32 ?");
                first = false;
                continue;
            }
            if first && after_open_control_word {
                out.push(b' ');
            }
            match c as u8 {
                b'\\' => out.extend_from_slice(b"\\\\"),
                b'{' => out.extend_from_slice(b"\\{"),
                b'}' => out.extend_from_slice(b"\\}"),
                b => out.push(b),
            }
        } else {
            out.extend_from_slice(rtf_unicode_escape(c as u32).as_bytes());
        }
        first = false;
    }
}

/// The `\ansicpgN` declared at the top of the file, if any.
pub(crate) fn declared_code_page(input: &[u8]) -> Option<u32> {
    find_control_word(&input[..input.len().min(1024)], b"ansicpg", Some(1))
        .and_then(|t| t.param)
        .and_then(|p| u32::try_from(p).ok())
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// `\uN` wants a signed 16-bit decimal. The emitted form is `\uN ?`: an
/// explicit space delimiter, then the one-character fallback. Pandoc 3.11
/// takes whatever character follows the number as the delimiter, so the
/// shorter `\uN?` would make it swallow the real character after the `?`
/// (measured: `caf\u233? fin` → `caféfin`, `caf\u233 ? fin` → `café fin`).
/// A spec-following reader treats both forms the same.
fn rtf_unicode_escape(code_point: u32) -> String {
    if code_point > 0xFFFF {
        // RTF carries anything above the BMP as a UTF-16 surrogate pair.
        let mut buf = [0u16; 2];
        let units = char::from_u32(code_point)
            .map(|c| c.encode_utf16(&mut buf).to_vec())
            .unwrap_or_default();
        return units
            .iter()
            .map(|&u| rtf_unicode_escape(u as u32))
            .collect::<String>();
    }
    let n = if code_point > 32767 {
        code_point as i64 - 65536
    } else {
        code_point as i64
    };
    format!("\\u{n} ?")
}

/// An inline annotation or inline footnote lifted out of the RTF before
/// Pandoc sees it (PLAN_SCRIVENER_FIDELITY slice 4). Scrivener writes them
/// as literal text with escaped braces, `{\Scrv_annot \color={…} \text=…
/// \end_Scrv_annot}` and `{\Scrv_fn=…\end_Scrv_fn}`; other tools write the
/// same as raw RTF groups. Both forms are replaced by a hyperlink field to
/// `scrivannot://<id>` or `scrivfn://<id>` whose text is a zero-width
/// space, so the body never reaches Pandoc and the position survives
/// conversion as a Markdown link the importer resolves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineNote {
    pub id: String,
    /// Plain-text body.
    pub body: String,
    pub footnote: bool,
}

/// [`prepare_rtf`] plus the inline notes lifted out first.
pub fn prepare_rtf_with_notes(input: &[u8]) -> (Vec<u8>, Vec<InlineNote>) {
    let (lifted, notes) = lift_inline_notes(input);
    (prepare_rtf(&lifted), notes)
}

/// Replaces every inline annotation/footnote with its marker field and
/// returns the bodies. Never fails: an unterminated marker is left alone.
pub fn lift_inline_notes(input: &[u8]) -> (Vec<u8>, Vec<InlineNote>) {
    struct InlineForm {
        start: &'static [u8],
        end: &'static [u8],
        footnote: bool,
        /// Text before the body inside the marker (`\text=` for annotations).
        body_prefix: &'static [u8],
    }
    const FORMS: [InlineForm; 4] = [
        InlineForm {
            start: b"\\{\\\\Scrv_annot",
            end: b"\\\\end_Scrv_annot\\}",
            footnote: false,
            body_prefix: b"\\\\text=",
        },
        InlineForm {
            start: b"{\\Scrv_annot",
            end: b"\\end_Scrv_annot}",
            footnote: false,
            body_prefix: b"\\text=",
        },
        InlineForm {
            start: b"\\{\\\\Scrv_fn=",
            end: b"\\\\end_Scrv_fn\\}",
            footnote: true,
            body_prefix: b"",
        },
        InlineForm {
            start: b"{\\Scrv_fn=",
            end: b"\\end_Scrv_fn}",
            footnote: true,
            body_prefix: b"",
        },
    ];
    let header = document_header(input);
    let mut out: Vec<u8> = Vec::with_capacity(input.len());
    let mut notes = Vec::new();
    let mut i = 0;
    let n = input.len();
    while i < n {
        // Earliest start marker from here.
        let mut best: Option<(usize, usize)> = None; // (position, form index)
        for (k, form) in FORMS.iter().enumerate() {
            if let Some(pos) = find(&input[i..], form.start) {
                let pos = i + pos;
                if best.is_none_or(|(p, _)| pos < p) {
                    best = Some((pos, k));
                }
            }
        }
        let Some((start, k)) = best else {
            out.extend_from_slice(&input[i..]);
            break;
        };
        let InlineForm {
            start: start_marker,
            end: end_marker,
            footnote,
            body_prefix,
        } = FORMS[k];
        let body_from = start + start_marker.len();
        let Some(end_rel) = find(&input[body_from..], end_marker) else {
            // Unterminated: leave the rest untouched.
            out.extend_from_slice(&input[i..]);
            break;
        };
        let end = body_from + end_rel;
        let mut body = &input[body_from..end];
        if !body_prefix.is_empty() {
            if let Some(p) = find(body, body_prefix) {
                body = &body[p + body_prefix.len()..];
            }
        }
        let id = format!("inline-{}", notes.len() + 1);
        // Both forms are RTF text read once: `\\`, `\{` and `\}` are the
        // writer's own backslash and braces, `\'xx` is a character, and a
        // control word is formatting. The literal form used to be unescaped
        // first and then read again, which turned `C:\\temp` into a control
        // word and `\{braces\}` into a group (review finding s4-3). The
        // body is read under the document's own header, so its hex escapes
        // and font switches decode as they would in place (review finding
        // s4-4: a `\ansicpg1251` note read `Привет` as `Ïðèâåò`).
        let mut wrapped = header.clone();
        // The font in force where the note sits carries into the wrapper:
        // with `\deff1` Cyrillic and `\f0` Western selected before the
        // note, `caf\'e9` is `café`, not `cafй` (s4-4 re-review).
        if let Some(font) = active_font_at(input, start) {
            wrapped.extend_from_slice(format!("\\f{font} ").as_bytes());
        }
        wrapped.extend_from_slice(body);
        wrapped.push(b'}');
        let text = super::rtf_text::rtf_to_plain_text(&wrapped);
        out.extend_from_slice(&input[i..start]);
        let scheme = if footnote { "scrivfn" } else { "scrivannot" };
        out.extend_from_slice(
            format!(
                "{{\\field{{\\*\\fldinst{{HYPERLINK \"{scheme}://{id}\"}}}}{{\\fldrslt \\u{} ?}}}}",
                8203
            )
            .as_bytes(),
        );
        notes.push(InlineNote {
            id,
            body: text,
            footnote,
        });
        i = end + end_marker.len();
    }
    (out, notes)
}

/// The opening of a document that reads text the way `input` does: its
/// `\ansicpgN`, its `\deffN` and its font table, so a fragment lifted out
/// of `input` decodes under the same encodings (review finding s4-4).
/// Ends with a space delimiter; the caller appends the fragment and `}`.
fn document_header(input: &[u8]) -> Vec<u8> {
    let mut header = b"{\\rtf1\\ansi".to_vec();
    if let Some(cp) = declared_code_page(input) {
        header.extend_from_slice(format!("\\ansicpg{cp}").as_bytes());
    }
    if let Some(f) = declared_default_font(input) {
        header.extend_from_slice(format!("\\deff{f}").as_bytes());
    }
    if let Some(table) = font_table_body(input) {
        header.extend_from_slice(b"{\\fonttbl");
        header.extend_from_slice(table);
        header.push(b'}');
    }
    header.push(b' ');
    header
}

/// The font selected by `\fN` in force at byte `pos`, honouring group scope
/// (a `\fN` inside a closed group is gone) and `\plain` (back to the
/// document default, so `None`). `None` also when no font was ever
/// selected; the document's `\deffN` then applies.
fn active_font_at(input: &[u8], pos: usize) -> Option<i64> {
    // The same token rules as `prepare_rtf` (s4-4 second re-review: a `\f1`
    // sitting in a `\uN` fallback slot was applied, and `\binN` always
    // swallowed a byte after its number, a closing brace included).
    struct FontGroup {
        uc: u32,
        font: Option<i64>,
    }
    let end = pos.min(input.len());
    let n = input.len();
    let mut groups: Vec<FontGroup> = vec![FontGroup { uc: 1, font: None }];
    let mut skip_fallback: u32 = 0;
    let mut i = 0;
    while i < end {
        match input[i] {
            b'{' => {
                let (uc, font) = groups.last().map_or((1, None), |g| (g.uc, g.font));
                groups.push(FontGroup { uc, font });
                skip_fallback = 0;
                i += 1;
            }
            b'}' => {
                if groups.len() > 1 {
                    groups.pop();
                }
                skip_fallback = 0;
                i += 1;
            }
            b'\\' => {
                let next = input.get(i + 1).copied();
                match next {
                    // `\` + newline: `\par`, or one fallback item.
                    Some(b'\n') | Some(b'\r') => {
                        i += 2;
                        if input.get(i) == Some(&b'\n') && next == Some(b'\r') {
                            i += 1;
                        }
                        skip_fallback = skip_fallback.saturating_sub(1);
                    }
                    // A hex escape is one character, so one fallback item.
                    Some(b'\'') => {
                        i += 4;
                        skip_fallback = skip_fallback.saturating_sub(1);
                    }
                    Some(c) if c.is_ascii_alphabetic() => {
                        let word_start = i + 1;
                        let mut j = word_start;
                        while j < n && input[j].is_ascii_alphabetic() {
                            j += 1;
                        }
                        let word = &input[word_start..j];
                        let mut k = j;
                        if k < n && input[k] == b'-' {
                            k += 1;
                        }
                        while k < n && input[k].is_ascii_digit() {
                            k += 1;
                        }
                        let param: Option<i64> = std::str::from_utf8(&input[j..k])
                            .ok()
                            .and_then(|s| s.parse().ok());
                        // One space after a control word is its delimiter;
                        // `\binN` then owns the next N bytes.
                        let mut end = k;
                        if end < n && input[end] == b' ' {
                            end += 1;
                        }
                        if word == b"bin" {
                            let len = usize::try_from(param.unwrap_or(0)).unwrap_or(0);
                            end = (end + len).min(n);
                        }
                        // A control word in a fallback slot is swallowed, not
                        // applied; only `\u` itself is exempt.
                        if skip_fallback > 0 && word != b"u" {
                            skip_fallback -= 1;
                            i = end;
                            continue;
                        }
                        match word {
                            b"uc" => {
                                if let Some(top) = groups.last_mut() {
                                    top.uc = param.unwrap_or(1).max(0) as u32;
                                }
                            }
                            b"u" if param.is_some() => {
                                skip_fallback = groups.last().map_or(1, |g| g.uc);
                            }
                            b"f" if param.is_some() => {
                                if let Some(top) = groups.last_mut() {
                                    top.font = param;
                                }
                            }
                            b"plain" => {
                                if let Some(top) = groups.last_mut() {
                                    top.font = None;
                                }
                            }
                            _ => {}
                        }
                        i = end;
                    }
                    // `\*` opens a destination and ends any fallback run.
                    Some(b'*') => {
                        skip_fallback = 0;
                        i += 2;
                    }
                    // Any other control symbol: one fallback item.
                    Some(_) => {
                        skip_fallback = skip_fallback.saturating_sub(1);
                        i += 2;
                    }
                    None => i += 1,
                }
            }
            // Bare line ends are ignored by RTF readers.
            b'\r' | b'\n' => i += 1,
            // A plain character is one fallback item.
            _ => {
                skip_fallback = skip_fallback.saturating_sub(1);
                i += 1;
            }
        }
    }
    groups.last().and_then(|g| g.font)
}

/// The contents of the `\fonttbl` group after the word itself, up to but
/// not including the brace that closes the group.
fn font_table_body(input: &[u8]) -> Option<&[u8]> {
    let start = find_control_word(input, b"fonttbl", None)?.end;
    let mut depth = 0i32;
    let mut i = start;
    while i < input.len() {
        match input[i] {
            b'{' => depth += 1,
            b'}' => {
                if depth == 0 {
                    return Some(&input[start..i]);
                }
                depth -= 1;
            }
            b'\\' => i += 1,
            _ => {}
        }
        i += 1;
    }
    None
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Rewrites the input as described in the module documentation. Never
/// fails: anything it does not understand is copied through unchanged.
pub fn prepare_rtf(input: &[u8]) -> Vec<u8> {
    // Hex escapes are rewritten for every code page `encoding_rs` knows;
    // an unknown page leaves them for Pandoc. A font's own encoding wins
    // over the document's for text set in that font.
    let document = declared_code_page(input).map_or(Some(encoding_rs::WINDOWS_1252), |cp| {
        encoding_for_code_page(cp)
    });
    let fonts = font_encodings(input);
    let default_font = declared_default_font(input);

    let mut out: Vec<u8> = Vec::with_capacity(input.len() + input.len() / 16);
    // Per-group `\ucN` (document default 1) and active font, which starts
    // as the document's default font.
    let mut groups: Vec<PrepGroup> = vec![PrepGroup {
        uc: 1,
        font: default_font,
    }];
    // Fallback items still to swallow after a `\uN`.
    let mut skip_fallback: u32 = 0;
    // Output length right after an undelimited control word was written;
    // decoded text appended at exactly that length needs a space first.
    let mut open_control_word_end: Option<usize> = None;
    // Where the output last ended with a control word's delimiter space: a
    // decoded space written there must be escaped (s1-3, fifth re-review).
    let mut delimited_control_word_end: Option<usize> = None;

    let mut i = 0;
    let n = input.len();
    while i < n {
        let b = input[i];
        match b {
            b'{' => {
                let top = *groups.last().unwrap_or(&PrepGroup {
                    uc: 1,
                    font: default_font,
                });
                groups.push(top);
                skip_fallback = 0;
                out.push(b);
                i += 1;
            }
            b'}' => {
                if groups.len() > 1 {
                    groups.pop();
                }
                skip_fallback = 0;
                out.push(b);
                i += 1;
            }
            b'\\' => {
                let next = input.get(i + 1).copied();
                match next {
                    // `\` + newline is `\par`; while a `\uN` fallback is
                    // pending it is one fallback item instead (the spec
                    // counts control symbols; s1-4 third re-review).
                    Some(b'\n') | Some(b'\r') => {
                        i += 2;
                        if input.get(i) == Some(&b'\n') && next == Some(b'\r') {
                            i += 1;
                        }
                        if skip_fallback > 0 {
                            skip_fallback -= 1;
                            continue;
                        }
                        out.extend_from_slice(b"\\par ");
                    }
                    // Hex escapes start a text run: the escapes and the plain
                    // bytes that follow them (up to the next brace or
                    // control sequence) are one byte string in the active
                    // encoding, so a two-byte character decodes whole even
                    // when its second byte is a literal (`\'83A` is `ア` in
                    // CP932). Bare line ends inside the run are ignored, as
                    // RTF readers ignore them.
                    Some(b'\'') => {
                        // (byte, start, end) of each text item in the run.
                        let mut items: Vec<(u8, usize, usize)> = Vec::new();
                        let mut j = i;
                        while j < n {
                            let c = input[j];
                            if c == b'\\' {
                                if input.get(j + 1) == Some(&b'\'') {
                                    match (
                                        input.get(j + 2).copied().and_then(hex_val),
                                        input.get(j + 3).copied().and_then(hex_val),
                                    ) {
                                        (Some(hi), Some(lo)) => {
                                            items.push(((hi << 4) | lo, j, j + 4));
                                            j += 4;
                                            continue;
                                        }
                                        _ => break,
                                    }
                                }
                                break;
                            }
                            if c == b'{' || c == b'}' {
                                break;
                            }
                            if c == b'\r' || c == b'\n' {
                                j += 1;
                                continue;
                            }
                            items.push((c, j, j + 1));
                            j += 1;
                        }
                        if items.is_empty() {
                            // Malformed; copy the two bytes and move on.
                            out.extend_from_slice(&input[i..(i + 2).min(n)]);
                            i += 2;
                            continue;
                        }
                        // Fallback bytes after `\uN` are swallowed one by one
                        // (`\ucN` counts bytes, two for a double-byte page).
                        let mut first = 0;
                        while skip_fallback > 0 && first < items.len() {
                            skip_fallback -= 1;
                            first += 1;
                        }
                        let rest = &items[first..];
                        if rest.is_empty() {
                            i = j;
                            continue;
                        }
                        let slice_start = rest[0].1;
                        let bytes: Vec<u8> = rest.iter().map(|item| item.0).collect();
                        let encoding = groups
                            .last()
                            .and_then(|g| g.font)
                            .and_then(|f| fonts.get(&f).copied())
                            .or(document);
                        match encoding {
                            Some(enc) => {
                                let (decoded, had_errors) = enc.decode_without_bom_handling(&bytes);
                                if had_errors {
                                    out.extend_from_slice(&input[slice_start..j]);
                                } else {
                                    let after_open = open_control_word_end == Some(out.len());
                                    let after_delimiter =
                                        delimited_control_word_end == Some(out.len());
                                    emit_run_text(&mut out, &decoded, after_open, after_delimiter);
                                }
                            }
                            None => out.extend_from_slice(&input[slice_start..j]),
                        }
                        i = j;
                    }
                    // Escaped literal characters count as fallback items.
                    Some(b'\\') | Some(b'{') | Some(b'}') => {
                        if skip_fallback > 0 {
                            skip_fallback -= 1;
                        } else {
                            out.extend_from_slice(&input[i..i + 2]);
                        }
                        i += 2;
                    }
                    // Control word.
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
                        // One space after a control word is its delimiter.
                        let mut end = k;
                        if end < n && input[end] == b' ' {
                            end += 1;
                        }
                        // `\binN` is followed by N raw bytes that belong to
                        // it; they are never text (and never fallback items
                        // of their own).
                        if word == b"bin" {
                            let len = param.unwrap_or(0).max(0) as usize;
                            end = (end + len).min(n);
                        }
                        // A pending `\uN` fallback (review finding s1-1,
                        // repaired twice after re-review): the spec counts
                        // every control word, `\binN` with its payload and
                        // `\ucN` included, as one fallback item, so it is
                        // swallowed, not applied, and the remaining count is
                        // kept. Only `\u` itself is exempt: it starts a new
                        // Unicode sequence.
                        if skip_fallback > 0 && word != b"u" {
                            skip_fallback -= 1;
                            i = end;
                            continue;
                        }
                        match word {
                            b"uc" => {
                                let uc = param.unwrap_or(1).max(0) as u32;
                                if let Some(top) = groups.last_mut() {
                                    top.uc = uc;
                                }
                                out.extend_from_slice(b"\\uc1 ");
                            }
                            b"u" if param.is_some() => {
                                out.extend_from_slice(&input[i..k]);
                                out.extend_from_slice(b" ?");
                                // The delimiter space after `\uN`, if any,
                                // is not a fallback item; `end` already
                                // steps over it.
                                skip_fallback = groups.last().map(|g| g.uc).unwrap_or(1);
                            }
                            b"f" if param.is_some() => {
                                if let Some(top) = groups.last_mut() {
                                    top.font = param;
                                }
                                out.extend_from_slice(&input[i..end]);
                            }
                            // `\plain` resets character formatting, the font
                            // included, to the document default.
                            b"plain" => {
                                if let Some(top) = groups.last_mut() {
                                    top.font = default_font;
                                }
                                out.extend_from_slice(&input[i..end]);
                            }
                            _ => out.extend_from_slice(&input[i..end]),
                        }
                        // A control word copied without its delimiter space
                        // is open: decoded text written right after it needs
                        // a space. `\binN` with a payload is not open (the
                        // payload follows the number); `\bin0` is, or its
                        // number would swallow a decoded digit (s1-3, fourth
                        // re-review). `\uc` and `\u` are always written with
                        // their own terminator.
                        if end == k && word != b"uc" && word != b"u" {
                            open_control_word_end = Some(out.len());
                        }
                        // Whitespace after a control word is swallowed by
                        // Pandoc however much there is (delimiter space, bare
                        // line ends, further spaces: s1-3 sixth re-review),
                        // so the marker starts here and the whitespace arms
                        // below carry it forward.
                        if word != b"u" {
                            delimited_control_word_end = Some(out.len());
                        }
                        i = end;
                    }
                    // `\*` opens a destination: structural, so it is kept and
                    // ends any fallback run rather than being swallowed.
                    Some(b'*') => {
                        skip_fallback = 0;
                        out.extend_from_slice(&input[i..i + 2]);
                        i += 2;
                    }
                    // Any other control symbol (`\~`, `\-`, `\_`, `\:` …) is
                    // one fallback item when a fallback is pending; copied
                    // otherwise.
                    Some(_) => {
                        if skip_fallback > 0 {
                            skip_fallback -= 1;
                        } else {
                            out.extend_from_slice(&input[i..i + 2]);
                        }
                        i += 2;
                    }
                    None => {
                        out.push(b);
                        i += 1;
                    }
                }
            }
            b'\r' | b'\n' => {
                // Bare line ends are ignored by RTF readers; keep for
                // readability. Whitespace right after a control word keeps
                // the delimiter marker alive.
                let after_control_word = delimited_control_word_end == Some(out.len());
                out.push(b);
                if after_control_word {
                    delimited_control_word_end = Some(out.len());
                }
                i += 1;
            }
            _ => {
                if skip_fallback > 0 {
                    skip_fallback -= 1;
                } else {
                    let after_control_word =
                        b == b' ' && delimited_control_word_end == Some(out.len());
                    out.push(b);
                    if after_control_word {
                        delimited_control_word_end = Some(out.len());
                    }
                }
                i += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prep(s: &str) -> String {
        String::from_utf8(prepare_rtf(s.as_bytes())).unwrap()
    }

    /// Pandoc's own reading of the prepared RTF, when Pandoc is installed.
    fn pandoc_plain(rtf: &str) -> Option<String> {
        use std::io::Write;
        use std::process::{Command, Stdio};
        let mut child = Command::new("pandoc")
            .args(["-f", "rtf", "-t", "plain"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        child.stdin.take()?.write_all(rtf.as_bytes()).ok()?;
        let out = child.wait_with_output().ok()?;
        if !out.status.success() {
            return Some(format!(
                "<pandoc failed: {}>",
                String::from_utf8_lossy(&out.stdout)
            ));
        }
        Some(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
    }

    fn assert_reads(prepared: &str, expected: &str) {
        match pandoc_plain(prepared) {
            Some(got) => assert_eq!(got, expected, "Pandoc reading of {prepared}"),
            None => eprintln!("Skipping Pandoc check: pandoc not installed"),
        }
    }

    #[test]
    fn backslash_newline_becomes_par() {
        assert_eq!(prep("{\\rtf1 one\\\ntwo}"), "{\\rtf1 one\\par two}");
        assert_eq!(prep("{\\rtf1 one\\\r\ntwo}"), "{\\rtf1 one\\par two}");
        assert_reads(&prep("{\\rtf1 one\\\ntwo}"), "one\n\ntwo");
    }

    #[test]
    fn hex_escape_before_apostrophe_becomes_unicode() {
        // The Zoe 2 failure shape: `Zo\'eb's`.
        let prepared = prep("{\\rtf1\\ansi\\ansicpg1252 Zo\\'eb's}");
        assert_eq!(prepared, "{\\rtf1\\ansi\\ansicpg1252 Zo\\u235 ?'s}");
        assert_reads(&prepared, "Zoë's");
        // Code page 1252 high half: 0x92 is a right single quote.
        let prepared = prep("{\\rtf1 it\\'92s}");
        assert_eq!(prepared, "{\\rtf1 it\\u8217 ?s}");
        assert_reads(&prepared, "it\u{2019}s");
    }

    /// Review finding s1-3: the declared code page decides what a hex
    /// escape means; Pandoc itself cannot read 1251, so the rewrite is what
    /// makes Cyrillic documents import at all.
    #[test]
    fn hex_escapes_follow_the_declared_code_page() {
        let prepared = prep("{\\rtf1\\ansi\\ansicpg1251 \\'cf\\'f0\\'e8\\'e2\\'e5\\'f2}");
        assert_eq!(
            prepared,
            "{\\rtf1\\ansi\\ansicpg1251 \\u1055 ?\\u1088 ?\\u1080 ?\\u1074 ?\\u1077 ?\\u1090 ?}"
        );
        assert_reads(&prepared, "Привет");
        // Mac Roman: 0x8E is é.
        assert_eq!(
            prep("{\\rtf1\\mac\\ansicpg10000 caf\\'8e}"),
            "{\\rtf1\\mac\\ansicpg10000 caf\\u233 ?}"
        );
        // Shift JIS: one character, two escapes.
        assert_eq!(
            prep("{\\rtf1\\ansi\\ansicpg932 \\'82\\'a0}"),
            "{\\rtf1\\ansi\\ansicpg932 \\u12354 ?}"
        );
    }

    /// Re-review of s1-3: a font's own encoding wins over the document's,
    /// and a two-byte character may end in a literal byte.
    #[test]
    fn font_encoding_and_mixed_runs() {
        let src = "{\\rtf1\\ansi\\ansicpg1251{\\fonttbl{\\f0\\fcharset0\\cpg1252 Arial;}{\\f1\\fcharset204 Arial;}}\\f0 caf\\'e9 \\f1 \\'cf}";
        let prepared = prep(src);
        assert_eq!(
            prepared,
            "{\\rtf1\\ansi\\ansicpg1251{\\fonttbl{\\f0\\fcharset0\\cpg1252 Arial;}{\\f1\\fcharset204 Arial;}}\\f0 caf\\u233 ? \\f1 \\u1055 ?}"
        );
        assert_reads(&prepared, "café П");
        // The font is scoped to its group.
        let src = "{\\rtf1\\ansi\\ansicpg1251{\\fonttbl{\\f0\\fcharset0 Arial;}}{\\f0 \\'e9}\\'e9}";
        assert_eq!(
            prep(src),
            "{\\rtf1\\ansi\\ansicpg1251{\\fonttbl{\\f0\\fcharset0 Arial;}}{\\f0 \\u233 ?}\\u1081 ?}"
        );
        // CP932 `ア` is the escaped byte 0x83 followed by a literal `A`.
        assert_eq!(
            prep("{\\rtf1\\ansi\\ansicpg932 \\'83A!}"),
            "{\\rtf1\\ansi\\ansicpg932 \\u12450 ?!}"
        );
        // Second re-review: the default font (`\deffN`) is the font before
        // any `\fN`, and `\plain` goes back to it.
        let src = "{\\rtf1\\ansi\\ansicpg1251\\deff0{\\fonttbl{\\f0\\fcharset0 A;}{\\f1\\fcharset204 B;}} caf\\'e9 \\f1\\plain caf\\'e9}";
        let prepared = prep(src);
        assert_eq!(
            prepared,
            "{\\rtf1\\ansi\\ansicpg1251\\deff0{\\fonttbl{\\f0\\fcharset0 A;}{\\f1\\fcharset204 B;}} caf\\u233 ? \\f1\\plain caf\\u233 ?}"
        );
        assert_reads(&prepared, "café café");
    }

    /// Second re-review of s1-3: decoded text is written back as RTF text,
    /// so it never merges into a control word or turns into syntax.
    #[test]
    fn decoded_text_stays_text() {
        let prepared = prep("{\\rtf1\\ansi \\b\\'61bc}");
        assert_eq!(prepared, "{\\rtf1\\ansi \\b abc}");
        assert_reads(&prepared, "abc");
        let prepared = prep("{\\rtf1\\ansi \\'5cpar tail}");
        assert_eq!(prepared, "{\\rtf1\\ansi \\\\par tail}");
        assert_reads(&prepared, "\\par tail");
        assert_eq!(
            prep("{\\rtf1\\ansi x\\'7by\\'7d}"),
            "{\\rtf1\\ansi x\\{y\\}}"
        );
        // An escaped backslash before the run is not a control word.
        assert_eq!(prep("{\\rtf1\\ansi \\\\\\'61}"), "{\\rtf1\\ansi \\\\a}");
        // Third re-review: prose after a control word is not the control
        // word, so no space is inserted inside words.
        assert_eq!(
            prep("{\\rtf1\\ansi \\b0text\\'61}"),
            "{\\rtf1\\ansi \\b0texta}"
        );
        assert_eq!(
            prep("{\\rtf1\\ansi co\\-operat\\'65}"),
            "{\\rtf1\\ansi co\\-operate}"
        );
        assert_eq!(prep("{\\rtf1\\ansi \\b\\'61}"), "{\\rtf1\\ansi \\b a}");
        // Fourth re-review: a zero-length `\bin0` is an open control word
        // too; a decoded digit after it must not become its byte count.
        let prepared = prep("{\\rtf1\\ansi before\\bin0\\'31after}");
        assert_eq!(prepared, "{\\rtf1\\ansi before\\bin0 1after}");
        assert_reads(&prepared, "before1after");
        // Fifth re-review: a decoded space right after an open control word
        // is written as an escape, because Pandoc drops a text space that
        // follows the delimiter space (`before\bin0  after` → `beforeafter`).
        let prepared = prep("{\\rtf1\\ansi before\\bin0\\'20after}");
        assert_eq!(prepared, "{\\rtf1\\ansi before\\bin0\\u32 ?after}");
        assert_reads(&prepared, "before after");
        // (Only the first space needs the escape; Pandoc's plain writer
        // collapses space runs, so the reading is not asserted here.)
        let prepared = prep("{\\rtf1\\ansi a\\b\\'20\\'20b}");
        assert_eq!(prepared, "{\\rtf1\\ansi a\\b\\u32 ? b}");
        // Fifth re-review: the same when the control word already has its
        // delimiter space (`before\bin0 \'20after` also read `beforeafter`).
        let prepared = prep("{\\rtf1\\ansi before\\bin0 \\'20after}");
        assert_eq!(prepared, "{\\rtf1\\ansi before\\bin0 \\u32 ?after}");
        assert_reads(&prepared, "before after");
        let prepared = prep("{\\rtf1\\ansi a\\b \\'20b}");
        assert_eq!(prepared, "{\\rtf1\\ansi a\\b \\u32 ?b}");
        assert_reads(&prepared, "a b");
        // A decoded space after ordinary text is written as itself.
        assert_eq!(prep("{\\rtf1\\ansi a\\'20b}"), "{\\rtf1\\ansi a b}");
        // Sixth re-review: whitespace between the control word and the
        // escape (a bare line end, CRLF, a second space) is swallowed by
        // Pandoc too, so the escape still applies across it.
        let prepared = prep("{\\rtf1\\ansi before\\bin0 \n\\'20after}");
        assert_eq!(prepared, "{\\rtf1\\ansi before\\bin0 \n\\u32 ?after}");
        assert_reads(&prepared, "before after");
        let prepared = prep("{\\rtf1\\ansi before\\bin0 \r\n\\'20after}");
        assert_eq!(prepared, "{\\rtf1\\ansi before\\bin0 \r\n\\u32 ?after}");
        assert_reads(&prepared, "before after");
        let prepared = prep("{\\rtf1\\ansi before\\bin0  \\'20after}");
        assert_eq!(prepared, "{\\rtf1\\ansi before\\bin0  \\u32 ?after}");
        assert_reads(&prepared, "before after");
        let prepared = prep("{\\rtf1\\ansi a\\b\n\\'20b}");
        assert_eq!(prepared, "{\\rtf1\\ansi a\\b\n\\u32 ?b}");
        assert_reads(&prepared, "a b");
        // Whitespace after ordinary text does not count.
        assert_eq!(prep("{\\rtf1\\ansi a \n\\'20b}"), "{\\rtf1\\ansi a \n b}");
    }

    /// Third re-review of s1-3: header declarations are control words, found
    /// by tokenizing; literal text that looks like one is text.
    #[test]
    fn header_declarations_are_tokens_not_substrings() {
        let src = "{\\rtf1\\ansi\\ansicpg1252{\\fonttbl{\\f0\\fcharset0 Arial;}{\\f1\\fcharset204 Times;}} \\\\deff1 caf\\'e9}";
        let prepared = prep(src);
        assert_eq!(
            prepared,
            "{\\rtf1\\ansi\\ansicpg1252{\\fonttbl{\\f0\\fcharset0 Arial;}{\\f1\\fcharset204 Times;}} \\\\deff1 caf\\u233 ?}"
        );
        assert_reads(&prepared, "\\deff1 café");
        assert_eq!(declared_default_font(src.as_bytes()), None);
        assert_eq!(declared_code_page(src.as_bytes()), Some(1252));
        // Literal `\\ansicpg1251` in text does not change the page either.
        assert_eq!(
            declared_code_page("{\\rtf1\\ansi \\\\ansicpg1251 x}".as_bytes()),
            None
        );
        // A real declaration in a nested group is not the document's.
        assert_eq!(
            declared_default_font("{\\rtf1\\ansi {\\deff1 x}}".as_bytes()),
            None
        );
        // The font table is found as a group, not as text.
        let fonts = font_encodings(
            "{\\rtf1\\ansi \\\\fonttbl text {\\fonttbl{\\f0\\fcharset204 A;}}}".as_bytes(),
        );
        assert_eq!(fonts.get(&0).map(|e| e.name()), Some("windows-1251"));
        // A bare line end inside the run is ignored, as readers ignore it.
        assert_eq!(
            prep("{\\rtf1\\ansi\\ansicpg932 \\'83\nA}"),
            "{\\rtf1\\ansi\\ansicpg932 \\u12450 ?}"
        );
    }

    /// Slice 4: inline annotations and footnotes, in Scrivener's literal-text
    /// form and in the raw-group form, become marker fields with their
    /// bodies lifted out.
    #[test]
    fn inline_notes_are_lifted_into_marker_fields() {
        // Inside the literal form, `\\`, `\{` and `\}` are the writer's own
        // characters; a bare control word is formatting.
        let src = "{\\rtf1\\ansi plain \\{\\\\Scrv_annot \\\\color=\\{\\\\R=0.0\\\\G=0.0\\\\B=1.0\\} \\\\text=note \\b here\\b0\\\\end_Scrv_annot\\} after \\{\\\\Scrv_fn=foot \\i note\\i0\\\\end_Scrv_fn\\} end}";
        let (lifted, notes) = lift_inline_notes(src.as_bytes());
        let lifted = String::from_utf8(lifted).unwrap();
        let marker = |scheme: &str, id: &str| {
            format!(
                "{{\\field{{\\*\\fldinst{{HYPERLINK \"{scheme}://{id}\"}}}}{{\\fldrslt \\u{} ?}}}}",
                8203
            )
        };
        assert_eq!(
            lifted,
            format!(
                "{{\\rtf1\\ansi plain {} after {} end}}",
                marker("scrivannot", "inline-1"),
                marker("scrivfn", "inline-2")
            )
        );
        assert_eq!(
            notes,
            vec![
                InlineNote {
                    id: "inline-1".into(),
                    body: "note here".into(),
                    footnote: false
                },
                InlineNote {
                    id: "inline-2".into(),
                    body: "foot note".into(),
                    footnote: true
                },
            ]
        );

        // Raw-group form.
        let src = "{\\rtf1\\ansi a {\\Scrv_annot \\color={\\R=0\\G=0\\B=1} \\text=raw body\\end_Scrv_annot} b {\\Scrv_fn=raw foot\\end_Scrv_fn} c}";
        let (lifted, notes) = lift_inline_notes(src.as_bytes());
        assert_eq!(
            String::from_utf8(lifted).unwrap(),
            format!(
                "{{\\rtf1\\ansi a {} b {} c}}",
                marker("scrivannot", "inline-1"),
                marker("scrivfn", "inline-2")
            )
        );
        assert_eq!(notes[0].body, "raw body");
        assert_eq!(notes[1].body, "raw foot");
        assert!(notes[1].footnote);

        // Review finding s4-3: the writer's braces and backslash survive; the
        // body is read as RTF exactly once.
        let src =
            "{\\rtf1\\ansi x \\{\\\\Scrv_fn=Keep \\{braces\\} and C:\\\\temp\\\\end_Scrv_fn\\} y}";
        let (_, notes) = lift_inline_notes(src.as_bytes());
        assert_eq!(notes[0].body, "Keep {braces} and C:\\temp");

        // Review finding s4-4: a body decodes under the document's own
        // header. Code page, default font and font-table encodings all
        // carry over.
        let src = "{\\rtf1\\ansi\\ansicpg1251 a {\\Scrv_fn=\\'cf\\'f0\\end_Scrv_fn} b}";
        let (_, notes) = lift_inline_notes(src.as_bytes());
        assert_eq!(notes[0].body, "Пр");
        let src = "{\\rtf1\\ansi\\ansicpg1252\\deff1{\\fonttbl{\\f0\\fcharset0 A;}{\\f1\\fcharset204 B;}} a \\{\\\\Scrv_fn=\\'cf\\\\end_Scrv_fn\\} b}";
        let (_, notes) = lift_inline_notes(src.as_bytes());
        assert_eq!(notes[0].body, "П");
        let src = "{\\rtf1\\ansi\\ansicpg1252{\\fonttbl{\\f0\\fcharset0 A;}{\\f1\\fcharset204 B;}} a {\\Scrv_fn=caf\\'e9 \\f1 \\'cf\\end_Scrv_fn} b}";
        let (_, notes) = lift_inline_notes(src.as_bytes());
        assert_eq!(notes[0].body, "café П");

        // s4-4 re-review: the font selected before the note carries into
        // it. With a Cyrillic default font and `\f0` (Western) in force,
        // `caf\'e9` is `café`; a `\f0` inside a closed group does not reach
        // the note (`\'e9` is `й` in the default font), nor does one undone
        // by `\plain`.
        let fonts = "{\\fonttbl{\\f0\\fcharset0 Arial;}{\\f1\\fcharset204 Arial;}}";
        let src = format!(
            "{{\\rtf1\\ansi\\ansicpg1252\\deff1{fonts}\\f0 before {{\\Scrv_fn=caf\\'e9\\end_Scrv_fn}} after}}"
        );
        let (_, notes) = lift_inline_notes(src.as_bytes());
        assert_eq!(notes[0].body, "café");
        let src = format!(
            "{{\\rtf1\\ansi\\ansicpg1252\\deff1{fonts} {{\\f0 x}} {{\\Scrv_fn=\\'e9\\end_Scrv_fn}} \\f0\\plain \\{{\\\\Scrv_fn=\\'e9\\\\end_Scrv_fn\\}}}}"
        );
        let (_, notes) = lift_inline_notes(src.as_bytes());
        assert_eq!(notes[0].body, "й");
        assert_eq!(notes[1].body, "й");

        // Second re-review: a `\f1` in a `\uN` fallback slot is swallowed,
        // not applied; a `\binN` payload follows the delimiter space only
        // when there is one, so `{\f1\bin1!}` and `{\f1\bin0}` close.
        let western = format!("{{\\rtf1\\ansi\\ansicpg1252\\deff0{fonts}");
        let src = format!("{western}\\uc1\\u233\\f1 before {{\\Scrv_fn=caf\\'e9\\end_Scrv_fn}}}}");
        let (_, notes) = lift_inline_notes(src.as_bytes());
        assert_eq!(notes[0].body, "café", "fallback slot");
        let src = format!("{western}{{\\f1\\bin1!}} {{\\Scrv_fn=caf\\'e9\\end_Scrv_fn}}}}");
        let (_, notes) = lift_inline_notes(src.as_bytes());
        assert_eq!(notes[0].body, "café", "bin1 payload");
        let src = format!("{western}{{\\f1\\bin0}} {{\\Scrv_fn=caf\\'e9\\end_Scrv_fn}}}}");
        let (_, notes) = lift_inline_notes(src.as_bytes());
        assert_eq!(notes[0].body, "café", "bin0");
        // And a `\f1` that really is in force still reaches the note.
        let src = format!("{western}\\uc1\\u233 ?\\f1 {{\\Scrv_fn=\\'e9\\end_Scrv_fn}}}}");
        let (_, notes) = lift_inline_notes(src.as_bytes());
        assert_eq!(notes[0].body, "й", "font after the fallback");

        // Unterminated marker: left alone, nothing lifted.
        let src = "{\\rtf1\\ansi a {\\Scrv_fn=never closed}";
        let (lifted, notes) = lift_inline_notes(src.as_bytes());
        assert_eq!(String::from_utf8(lifted).unwrap(), src);
        assert!(notes.is_empty());

        // Pandoc turns the marker field into a link with a zero-width-space
        // text, which the importer resolves.
        let (prepared, _) =
            prepare_rtf_with_notes("{\\rtf1\\ansi w {\\Scrv_fn=f\\end_Scrv_fn} x}".as_bytes());
        if let Some(read) = pandoc_plain(&String::from_utf8(prepared).unwrap()) {
            assert_eq!(read, "w \u{200B} x");
        }
    }

    #[test]
    fn unknown_code_page_leaves_hex_escapes_alone() {
        assert_eq!(
            prep("{\\rtf1\\ansi\\ansicpg4242 \\'e0\\'e1}"),
            "{\\rtf1\\ansi\\ansicpg4242 \\'e0\\'e1}"
        );
    }

    #[test]
    fn unicode_fallback_is_collapsed_not_doubled() {
        // Review finding plan-3 R1-1: `\uc1 caf\u233\'e9 fin` must stay
        // "café fin", space included.
        let prepared = prep("{\\rtf1\\uc1 caf\\u233\\'e9 fin}");
        assert_eq!(prepared, "{\\rtf1\\uc1 caf\\u233 ? fin}");
        assert_reads(&prepared, "café fin");
    }

    #[test]
    fn uc2_swallows_two_fallback_items_and_is_rewritten_to_uc1() {
        let prepared = prep("{\\rtf1\\uc2 a\\u233\\'e9\\'e9b}");
        assert_eq!(prepared, "{\\rtf1\\uc1 a\\u233 ?b}");
        assert_reads(&prepared, "aéb");
        // Plain characters count as fallback items too.
        assert_eq!(
            prep("{\\rtf1\\uc2 a\\u233 ??b}"),
            "{\\rtf1\\uc1 a\\u233 ?b}"
        );
    }

    #[test]
    fn uc0_consumes_nothing_and_keeps_the_following_text() {
        // Cocoa writes `\uc0\u233 ` with a delimiter space; the character
        // after it is real text.
        let prepared = prep("{\\rtf1\\uc0 caf\\u233  fin}");
        assert_eq!(prepared, "{\\rtf1\\uc1 caf\\u233 ? fin}");
        assert_reads(&prepared, "café fin");
        assert_eq!(prep("{\\rtf1\\uc0 a\\u233 b}"), "{\\rtf1\\uc1 a\\u233 ?b}");
    }

    #[test]
    fn fallback_that_is_an_escaped_brace_is_swallowed() {
        let prepared = prep("{\\rtf1 a\\u123\\{b}");
        assert_eq!(prepared, "{\\rtf1 a\\u123 ?b}");
        assert_reads(&prepared, "a{b");
    }

    #[test]
    fn uc_is_tracked_per_group() {
        // uc2 inside the inner group does not leak out of it.
        let prepared = prep("{\\rtf1 {\\uc2 a\\u233\\'e9\\'e9}b\\u235\\'ebc}");
        assert_eq!(prepared, "{\\rtf1 {\\uc1 a\\u233 ?}b\\u235 ?c}");
        assert_reads(&prepared, "aébëc");
    }

    /// Review finding s1-1: `舑\endash word` must keep the `w`. The
    /// spec counts every control word or symbol after `\uN` as one fallback
    /// item: swallowed, not applied, and the count carries on, so
    /// `\uc2 caf\u233\b e` skips `\b` and `e`. `\u` itself is never fallback.
    #[test]
    fn character_control_word_after_unicode_is_the_fallback() {
        let src = format!("{{\\rtf1\\ansi\\uc1 a\\u{}\\endash word}}", 8211);
        let prepared = prep(&src);
        assert_eq!(
            prepared,
            format!("{{\\rtf1\\ansi\\uc1 a\\u{} ?word}}", 8211)
        );
        assert_reads(&prepared, "a\u{2013}word");

        let prepared = prep("{\\rtf1\\ansi\\uc1 a\\u233\\b b}");
        assert_eq!(prepared, "{\\rtf1\\ansi\\uc1 a\\u233 ?b}");
        assert_reads(&prepared, "aéb");

        let prepared = prep("{\\rtf1\\ansi\\uc1 a\\u233\\~b}");
        assert_eq!(prepared, "{\\rtf1\\ansi\\uc1 a\\u233 ?b}");

        // Re-review case: two fallback items, a formatting word and a letter.
        let prepared = prep("{\\rtf1\\ansi {\\uc2 caf\\u233\\b e} noir}");
        assert_eq!(prepared, "{\\rtf1\\ansi {\\uc1 caf\\u233 ?} noir}");
        assert_reads(&prepared, "café noir");

        // `\u` after `\u` starts a new character, never a fallback; the
        // `b` after the second one is its fallback and goes.
        let prepared = prep("{\\rtf1\\ansi\\uc1 a\\u233\\u235 b}");
        assert_eq!(prepared, "{\\rtf1\\ansi\\uc1 a\\u233 ?\\u235 ?}");
        assert_reads(&prepared, "aéë");

        // Second re-review: `\ucN` and `\binN` with its payload are each one
        // fallback item too; the text after them survives.
        let prepared = prep("{\\rtf1\\ansi\\uc1 a\\u233\\uc0 word}");
        assert_eq!(prepared, "{\\rtf1\\ansi\\uc1 a\\u233 ?word}");
        assert_reads(&prepared, "aéword");
        let prepared = prep("{\\rtf1\\ansi\\uc1 a\\u233\\bin3 XYZword}");
        assert_eq!(prepared, "{\\rtf1\\ansi\\uc1 a\\u233 ?word}");
        assert_reads(&prepared, "aéword");
    }

    #[test]
    fn binary_payloads_pass_through_untouched() {
        // Outside a fallback, `\binN` and its N bytes are copied as they are,
        // even when the payload looks like RTF syntax.
        let src = "{\\rtf1 a\\bin5 {\\'ebb}";
        assert_eq!(prep(src), src);
    }

    #[test]
    fn hyperlink_field_passes_through_byte_for_byte() {
        let field = "{\\field{\\*\\fldinst{HYPERLINK \"scrivcmt://ABC-123\"}}{\\fldrslt here}}";
        assert_eq!(
            prep(&format!("{{\\rtf1 {field}}}")),
            format!("{{\\rtf1 {field}}}")
        );
    }

    #[test]
    fn negative_unicode_values_are_written_signed() {
        // U+F8FF (private use) is above 32767.
        assert_eq!(rtf_unicode_escape(0xF8FF), "\\u-1793 ?");
    }

    #[test]
    fn control_words_and_symbols_are_untouched() {
        let src = "{\\rtf1\\ansi\\ansicpg1252\\cocoartf2509\n{\\fonttbl\\f0\\froman\\fcharset0 TimesNewRomanPSMT;}\n\\pard\\tx360\\fi360\\sl480\\slmult1\\partightenfactor0\n\\f0\\fs24 \\cf0 a\\~b\\-c\\_d \\*\\e}";
        assert_eq!(prep(src), src);
    }
}
