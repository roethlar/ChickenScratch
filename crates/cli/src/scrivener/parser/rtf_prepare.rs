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

/// The declared code page's encoding, Windows-1252 when absent or unknown.
pub(crate) fn document_encoding(input: &[u8]) -> &'static encoding_rs::Encoding {
    declared_code_page(input)
        .and_then(encoding_for_code_page)
        .unwrap_or(encoding_rs::WINDOWS_1252)
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

/// Control words that stand for one character. After `\uN`, one of these
/// is the fallback (RTF writers may emit `舑\endash`) and is swallowed;
/// the fallback count is in characters, and these are characters.
pub(crate) fn is_character_control_word(word: &[u8]) -> bool {
    matches!(
        word,
        b"endash"
            | b"emdash"
            | b"lquote"
            | b"rquote"
            | b"ldblquote"
            | b"rdblquote"
            | b"bullet"
            | b"tab"
            | b"emspace"
            | b"enspace"
            | b"qmspace"
            | b"zwj"
            | b"zwnj"
            | b"lbr"
    )
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
    // an unknown page leaves them for Pandoc.
    let encoding = declared_code_page(input).map_or(Some(encoding_rs::WINDOWS_1252), |cp| {
        encoding_for_code_page(cp)
    });

    let mut out: Vec<u8> = Vec::with_capacity(input.len() + input.len() / 16);
    // `\ucN` per group; the document default is 1.
    let mut uc_stack: Vec<u32> = vec![1];
    // Fallback items still to swallow after a `\uN`.
    let mut skip_fallback: u32 = 0;

    let mut i = 0;
    let n = input.len();
    while i < n {
        let b = input[i];
        match b {
            b'{' => {
                let uc = *uc_stack.last().unwrap_or(&1);
                uc_stack.push(uc);
                skip_fallback = 0;
                out.push(b);
                i += 1;
            }
            b'}' => {
                if uc_stack.len() > 1 {
                    uc_stack.pop();
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
                    // Hex escapes: the whole run of consecutive escapes is
                    // one byte string in the declared code page, so a
                    // two-byte character (Shift JIS, GBK) decodes whole.
                    Some(b'\'') => {
                        let mut bytes: Vec<u8> = Vec::new();
                        let mut j = i;
                        while j + 3 < n && input[j] == b'\\' && input[j + 1] == b'\'' {
                            match (hex_val(input[j + 2]), hex_val(input[j + 3])) {
                                (Some(hi), Some(lo)) => {
                                    bytes.push((hi << 4) | lo);
                                    j += 4;
                                }
                                _ => break,
                            }
                        }
                        if bytes.is_empty() {
                            // Malformed; copy the two bytes and move on.
                            out.extend_from_slice(&input[i..(i + 2).min(n)]);
                            i += 2;
                            continue;
                        }
                        // Fallback bytes after `\uN` are swallowed one by one
                        // (`\ucN` counts bytes, two for a double-byte page).
                        let mut first = 0;
                        while skip_fallback > 0 && first < bytes.len() {
                            skip_fallback -= 1;
                            first += 1;
                        }
                        let rest = &bytes[first..];
                        let rest_start = j - 4 * rest.len();
                        match encoding {
                            Some(enc) if !rest.is_empty() => {
                                let (decoded, had_errors) = enc.decode_without_bom_handling(rest);
                                if had_errors {
                                    out.extend_from_slice(&input[rest_start..j]);
                                } else {
                                    for c in decoded.chars() {
                                        out.extend_from_slice(
                                            rtf_unicode_escape(c as u32).as_bytes(),
                                        );
                                    }
                                }
                            }
                            _ => out.extend_from_slice(&input[rest_start..j]),
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
                        // A pending `\uN` fallback (review finding s1-1):
                        // a control word that stands for a character is one
                        // fallback item and is swallowed; any other control
                        // word ends the fallback run so no later letter can
                        // be eaten.
                        if skip_fallback > 0 {
                            if is_character_control_word(word) {
                                skip_fallback -= 1;
                                i = end;
                                continue;
                            }
                            skip_fallback = 0;
                        }
                        match word {
                            b"uc" => {
                                let uc = param.unwrap_or(1).max(0) as u32;
                                if let Some(top) = uc_stack.last_mut() {
                                    *top = uc;
                                }
                                out.extend_from_slice(b"\\uc1 ");
                            }
                            b"u" if param.is_some() => {
                                out.extend_from_slice(&input[i..k]);
                                out.extend_from_slice(b" ?");
                                // The delimiter space after `\uN`, if any,
                                // is not a fallback item; `end` already
                                // steps over it.
                                skip_fallback = *uc_stack.last().unwrap_or(&1);
                            }
                            _ => out.extend_from_slice(&input[i..end]),
                        }
                        i = end;
                    }
                    // Control symbols that stand for a character (`\~`,
                    // `\-`, `\_`) are fallback items like any character.
                    Some(b'~') | Some(b'-') | Some(b'_') => {
                        if skip_fallback > 0 {
                            skip_fallback -= 1;
                        } else {
                            out.extend_from_slice(&input[i..i + 2]);
                        }
                        i += 2;
                    }
                    // Any other control symbol (`\*`, `\:` …) or a lone
                    // backslash at the end: copy; it ends a fallback run.
                    Some(_) => {
                        skip_fallback = 0;
                        out.extend_from_slice(&input[i..i + 2]);
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
    /// character control word is the fallback and is swallowed; a
    /// formatting control word ends the fallback run instead.
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
        assert_eq!(prepared, "{\\rtf1\\ansi\\uc1 a\\u233 ?\\b b}");
        assert_reads(&prepared, "aéb");

        let prepared = prep("{\\rtf1\\ansi\\uc1 a\\u233\\~b}");
        assert_eq!(prepared, "{\\rtf1\\ansi\\uc1 a\\u233 ?b}");
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
