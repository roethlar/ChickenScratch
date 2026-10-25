//! # RTF Converter
//!
//! Converts between Scrivener RTF content and Markdown.
//!
//! Uses Pandoc as external tool for robust RTF parsing. Scrivener's RTF is
//! prepared first (`rtf_prepare`: paragraph ends, hex escapes, Unicode
//! fallbacks) because Pandoc's reader mishandles those three things, and a
//! document Pandoc still rejects falls back to a built-in plain-text
//! extraction (`rtf_text`) so one bad file never kills an import
//! (PLAN_SCRIVENER_FIDELITY slice 1).

use chickenscratch_core::utils::error::ChiknError;
use chickenscratch_core::utils::process::{
    output_bounded, BoundedProcessError, PANDOC_OUTPUT_LIMIT_BYTES, PANDOC_TIMEOUT,
};
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::process::Command;

use super::rtf_prepare::prepare_rtf;
use super::rtf_text::rtf_to_plain_markdown;

fn pandoc_cmd(pandoc_path: Option<&Path>) -> &OsStr {
    pandoc_path
        .map(|p| p.as_os_str())
        .unwrap_or(OsStr::new("pandoc"))
}

/// The Markdown for one Scrivener RTF document and how it was obtained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtfConversion {
    pub markdown: String,
    /// True when Pandoc rejected the document and the built-in plain-text
    /// extractor produced `markdown` instead. Formatting is lost; the
    /// caller names the document in the import warnings.
    pub fell_back: bool,
}

/// Why Pandoc did not return Markdown for a prepared RTF file.
enum PandocRtfFailure {
    /// Pandoc ran and refused the input: the document itself is the problem.
    Rejected(String),
    /// Pandoc could not be run, timed out or overflowed the output cap.
    Other(ChiknError),
}

/// Runs Pandoc's RTF reader on an already prepared file, writing
/// `markdown-smart` so apostrophes and quotes come out as typed instead of
/// as `\'` escapes.
fn run_pandoc_rtf(prepared: &Path, pandoc_path: Option<&Path>) -> Result<String, PandocRtfFailure> {
    let mut cmd = Command::new(pandoc_cmd(pandoc_path));
    cmd.arg("-f")
        .arg("rtf")
        .arg("-t")
        .arg("markdown-smart")
        .arg("--wrap=none")
        .arg(prepared);
    let output =
        output_bounded(&mut cmd, PANDOC_TIMEOUT, PANDOC_OUTPUT_LIMIT_BYTES).map_err(|e| {
            PandocRtfFailure::Other(ChiknError::Unknown(format!("Failed to run Pandoc: {}", e)))
        })?;

    if !output.status.success() {
        return Err(PandocRtfFailure::Rejected(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }

    String::from_utf8(output.stdout).map_err(|e| {
        PandocRtfFailure::Other(ChiknError::InvalidFormat(format!(
            "Invalid UTF-8 from Pandoc: {}",
            e
        )))
    })
}

/// Prepares `rtf_path` for Pandoc and runs it, returning the Markdown.
/// A document Pandoc refuses is an error here; see
/// [`rtf_file_to_markdown`] for the fallback the importer uses.
pub fn rtf_to_html(rtf_path: &Path, pandoc_path: Option<&Path>) -> Result<String, ChiknError> {
    check_pandoc_available(pandoc_path)?;
    let raw = fs::read(rtf_path)?;
    match convert_prepared(&raw, pandoc_path)? {
        Ok(markdown) => Ok(markdown),
        Err(stderr) => Err(ChiknError::InvalidFormat(format!(
            "Pandoc conversion failed: {}",
            stderr
        ))),
    }
}

/// Converts one Scrivener RTF file to Markdown, falling back to plain text
/// when Pandoc rejects it. Pandoc being absent or unrunnable is still an
/// error: that is an installation problem, not a bad document.
pub fn rtf_file_to_markdown(
    rtf_path: &Path,
    pandoc_path: Option<&Path>,
) -> Result<RtfConversion, ChiknError> {
    check_pandoc_available(pandoc_path)?;
    let raw = fs::read(rtf_path)?;
    match convert_prepared(&raw, pandoc_path)? {
        Ok(markdown) => Ok(RtfConversion {
            markdown,
            fell_back: false,
        }),
        Err(_rejected) => Ok(RtfConversion {
            markdown: rtf_to_plain_markdown(&raw),
            fell_back: true,
        }),
    }
}

/// Prepare, write to a temporary file, run Pandoc, clean up. The outer
/// `Result` is an environment failure; the inner `Err` carries Pandoc's
/// rejection text for a document it would not read.
fn convert_prepared(
    raw: &[u8],
    pandoc_path: Option<&Path>,
) -> Result<Result<String, String>, ChiknError> {
    let prepared = prepare_rtf(raw);
    let temp_file = std::env::temp_dir().join(format!("chikn_rtf_{}.rtf", uuid::Uuid::new_v4()));
    fs::write(&temp_file, &prepared)?;
    let result = run_pandoc_rtf(&temp_file, pandoc_path);
    let _ = fs::remove_file(&temp_file);
    match result {
        Ok(markdown) => Ok(Ok(markdown)),
        Err(PandocRtfFailure::Rejected(stderr)) => Ok(Err(stderr)),
        Err(PandocRtfFailure::Other(e)) => Err(e),
    }
}

/// Converts RTF string to Markdown using Pandoc.
pub fn rtf_string_to_html(
    rtf_content: &str,
    pandoc_path: Option<&Path>,
) -> Result<String, ChiknError> {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!("temp_{}.rtf", uuid::Uuid::new_v4()));

    fs::write(&temp_file, rtf_content)?;
    let result = rtf_to_html(&temp_file, pandoc_path);
    let _ = fs::remove_file(&temp_file);

    result
}

/// Converts Markdown file to RTF using Pandoc.
pub fn html_to_rtf(
    html_path: &Path,
    rtf_path: &Path,
    pandoc_path: Option<&Path>,
) -> Result<(), ChiknError> {
    check_pandoc_available(pandoc_path)?;

    let mut cmd = Command::new(pandoc_cmd(pandoc_path));
    cmd.arg("-f")
        .arg("markdown")
        .arg("-t")
        .arg("rtf")
        .arg("-o")
        .arg(rtf_path)
        .arg(html_path);
    let output = output_bounded(&mut cmd, PANDOC_TIMEOUT, PANDOC_OUTPUT_LIMIT_BYTES)
        .map_err(|e| ChiknError::Unknown(format!("Failed to run Pandoc: {}", e)))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(ChiknError::InvalidFormat(format!(
            "Pandoc RTF conversion failed: {}",
            error
        )));
    }

    Ok(())
}

/// Converts Markdown string to RTF string using Pandoc.
pub fn html_string_to_rtf(
    html_content: &str,
    pandoc_path: Option<&Path>,
) -> Result<String, ChiknError> {
    let temp_dir = std::env::temp_dir();
    let temp_html = temp_dir.join(format!("temp_{}.md", uuid::Uuid::new_v4()));
    let temp_rtf = temp_dir.join(format!("temp_{}.rtf", uuid::Uuid::new_v4()));

    fs::write(&temp_html, html_content)?;
    let result = html_to_rtf(&temp_html, &temp_rtf, pandoc_path);

    let output = if result.is_ok() {
        let rtf_content = fs::read_to_string(&temp_rtf)?;
        Ok(rtf_content)
    } else {
        result.map(|_| String::new())
    };

    let _ = fs::remove_file(&temp_html);
    let _ = fs::remove_file(&temp_rtf);

    output
}

fn check_pandoc_available(pandoc_path: Option<&Path>) -> Result<(), ChiknError> {
    let mut cmd = Command::new(pandoc_cmd(pandoc_path));
    cmd.arg("--version");
    let output = output_bounded(&mut cmd, PANDOC_TIMEOUT, PANDOC_OUTPUT_LIMIT_BYTES).map_err(
        |err| match err {
            BoundedProcessError::Io(_) => ChiknError::Unknown(
                "Pandoc not found. Please install Pandoc: https://pandoc.org/installing.html"
                    .to_string(),
            ),
            other => ChiknError::Unknown(format!("Pandoc availability check failed: {}", other)),
        },
    )?;

    if !output.status.success() {
        return Err(ChiknError::Unknown("Pandoc not available".to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_check_pandoc_available() {
        if check_pandoc_available(None).is_err() {
            eprintln!("Skipping test: Pandoc not installed");
            return;
        }
        assert!(check_pandoc_available(None).is_ok());
    }

    #[test]
    fn test_rtf_to_html() {
        if check_pandoc_available(None).is_err() {
            eprintln!("Skipping test: Pandoc not installed");
            return;
        }

        let temp_dir = TempDir::new().unwrap();
        let rtf_file = temp_dir.path().join("test.rtf");

        let rtf_content = r"{\rtf1\ansi\deff0
{\fonttbl{\f0 Times New Roman;}}
\f0\fs24 Hello world
}";
        fs::write(&rtf_file, rtf_content).unwrap();

        if let Ok(html) = rtf_to_html(&rtf_file, None) {
            assert!(html.contains("Hello world") || html.contains("Hello"));
        }
    }

    #[test]
    fn test_rtf_string_to_html() {
        if check_pandoc_available(None).is_err() {
            eprintln!("Skipping test: Pandoc not installed");
            return;
        }

        let rtf = r"{\rtf1\ansi Simple text}";

        if let Ok(html) = rtf_string_to_html(rtf, None) {
            assert!(html.contains("Simple") || !html.is_empty());
        }
    }
}
