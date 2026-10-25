//! Scrivener 3 inline comments and footnotes (PLAN_SCRIVENER_FIDELITY
//! slice 4).
//!
//! Scrivener keeps a document's comments and footnotes in a side file,
//! `Files/Data/<UUID>/content.comments`:
//!
//! ```xml
//! <Comments>
//!   <Comment ID="UUID" Footnote="Yes" Color="…">{\rtf1 … body …}</Comment>
//! </Comments>
//! ```
//!
//! and marks the commented text in `content.rtf` as a hyperlink to
//! `scrivcmt://<UUID>`. Bodies are RTF (sometimes plain text); they come
//! out here as plain text.

use chickenscratch_core::utils::error::ChiknError;
use quick_xml::de::from_str;
use serde::Deserialize;
use std::fs;
use std::path::Path;

use super::rtf_text::rtf_to_plain_text;
use super::scrivx::get_data_file_path;

/// One comment or footnote from `content.comments`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrivComment {
    /// The ID the `scrivcmt://` anchor in the text refers to.
    pub id: String,
    /// Plain-text body.
    pub body: String,
    /// `Footnote="Yes"`: a footnote rather than a comment.
    pub footnote: bool,
}

#[derive(Debug, Deserialize)]
struct CommentsXml {
    #[serde(rename = "Comment", default)]
    items: Vec<CommentXml>,
}

#[derive(Debug, Deserialize)]
struct CommentXml {
    #[serde(rename = "@ID")]
    id: String,
    #[serde(rename = "@Footnote", default)]
    footnote: Option<String>,
    #[serde(rename = "$text", default)]
    body: String,
}

/// Reads an item's comments file; no file means no comments.
pub fn read_comments(scriv_path: &Path, uuid: &str) -> Result<Vec<ScrivComment>, ChiknError> {
    let path = get_data_file_path(scriv_path, uuid, "content.comments")?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let xml = fs::read_to_string(&path)?;
    parse_comments(&xml)
}

/// Parses the comments XML. A body that is RTF is reduced to plain text.
pub fn parse_comments(xml: &str) -> Result<Vec<ScrivComment>, ChiknError> {
    let parsed: CommentsXml = from_str(xml)
        .map_err(|e| ChiknError::InvalidFormat(format!("Failed to parse content.comments: {e}")))?;
    Ok(parsed
        .items
        .into_iter()
        .map(|c| ScrivComment {
            id: c.id,
            footnote: c
                .footnote
                .as_deref()
                .is_some_and(|f| f.trim().eq_ignore_ascii_case("yes")),
            body: comment_body(&c.body),
        })
        .collect())
}

/// RTF bodies become plain text; anything else is trimmed as it is.
pub fn comment_body(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with("{\\rtf") {
        rtf_to_plain_text(trimmed.as_bytes())
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_comments_and_footnotes_with_rtf_bodies() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Comments>
    <Comment ID="AAAA-1" Color="0.99 0.99 0.70"><![CDATA[{\rtf1\ansi\ansicpg1252 {\fonttbl\f0\fswiss Helvetica;}\f0\fs24 Tighten this \i scene\i0 .}]]></Comment>
    <Comment ID="BBBB-2" Footnote="Yes">{\rtf1\ansi See the 1998 edition, p.\~12.}</Comment>
    <Comment ID="CCCC-3">Plain text body</Comment>
</Comments>"#;
        let comments = parse_comments(xml).unwrap();
        assert_eq!(comments.len(), 3);
        assert_eq!(comments[0].id, "AAAA-1");
        assert!(!comments[0].footnote);
        assert_eq!(comments[0].body, "Tighten this scene.");
        assert!(comments[1].footnote);
        assert_eq!(comments[1].body, "See the 1998 edition, p.\u{A0}12.");
        assert_eq!(comments[2].body, "Plain text body");
    }

    #[test]
    fn empty_file_means_no_comments() {
        assert!(parse_comments("<Comments/>").unwrap().is_empty());
    }
}
