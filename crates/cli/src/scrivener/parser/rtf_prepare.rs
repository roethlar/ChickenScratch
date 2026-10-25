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
    let needle = b"\\fonttbl";
    let Some(pos) = input.windows(needle.len()).position(|w| w == needle) else {
        return map;
    };
    let n = input.len();
    let mut depth = 0i32;
    let mut font: Option<i64> = None;
    let mut i = pos + needle.len();
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
    let head = &input[..input.len().min(512)];
    let needle = b"\\deff";
    let pos = head.windows(needle.len()).position(|w| w == needle)?;
    let mut digits = String::new();
    for &b in &head[pos + needle.len()..] {
        if b.is_ascii_digit() && digits.len() < 6 {
            digits.push(b as char);
        } else {
            break;
        }
    }
    digits.parse().ok()
}

/// Appends decoded run text to `out` as RTF text: a backslash or brace is
/// re-escaped, a non-ASCII character becomes `\uN ?`, and a space delimiter
/// goes first when `out` ends inside a control word, so `\b` followed by a
/// decoded `a` stays `\b a` and never `\ba` (review finding s1-3, second
/// re-review).
fn emit_run_text(out: &mut Vec<u8>, decoded: &str) {
    let mut first = true;
    for c in decoded.chars() {
        if c.is_ascii() {
            if first && ends_in_control_word(out) {
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

/// True when `out` ends with an undelimited control word (`\b`, `\fs24`,
/// `\u-123`): letters, digits or `-` back to a backslash that is itself not
/// escaped.
fn ends_in_control_word(out: &[u8]) -> bool {
    let mut i = out.len();
    let mut saw_body = false;
    while i > 0 {
        let b = out[i - 1];
        if b.is_ascii_alphanumeric() || b == b'-' {
            saw_body = true;
            i -= 1;
            continue;
        }
        if b == b'\\' && saw_body {
            let mut preceding = 0;
            let mut k = i - 1;
            while k > 0 && out[k - 1] == b'\\' {
                preceding += 1;
                k -= 1;
            }
            return preceding % 2 == 0;
        }
        return false;
    }
    false
}

/// The `\ansicpgN` declared at the top of the file, if any.
pub(crate) fn declared_code_page(input: &[u8]) -> Option<u32> {
    let head = &input[..input.len().min(512)];
    let needle = b"\\ansicpg";
    let pos = head.windows(needle.len()).position(|w| w == needle)?;
    let mut digits = String::new();
    for &b in &head[pos + needle.len()..] {
        if b.is_ascii_digit() && digits.len() < 6 {
            digits.push(b as char);
        } else {
            break;
        }
    }
    digits.parse().ok()
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
                    // `\` + newline is `\par`.
                    Some(b'\n') | Some(b'\r') => {
                        out.extend_from_slice(b"\\par ");
                        i += 2;
                        if input.get(i) == Some(&b'\n') && next == Some(b'\r') {
                            i += 1;
                        }
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
                                    emit_run_text(&mut out, &decoded);
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
                // readability.
                out.push(b);
                i += 1;
            }
            _ => {
                if skip_fallback > 0 {
                    skip_fallback -= 1;
                } else {
                    out.push(b);
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
        // A bare line end inside the run is ignored, as readers ignore it.
        assert_eq!(
            prep("{\\rtf1\\ansi\\ansicpg932 \\'83\nA}"),
            "{\\rtf1\\ansi\\ansicpg932 \\u12450 ?}"
        );
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
