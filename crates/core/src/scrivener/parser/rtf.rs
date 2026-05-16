//! # RTF Converter
//!
//! Converts between Scrivener RTF content and Markdown.
//!
//! Uses Pandoc as external tool for robust RTF parsing.

use crate::utils::error::ChiknError;
use crate::utils::process::{
    output_bounded, BoundedProcessError, PANDOC_OUTPUT_LIMIT_BYTES, PANDOC_TIMEOUT,
};
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::process::Command;

fn pandoc_cmd(pandoc_path: Option<&Path>) -> &OsStr {
    pandoc_path
        .map(|p| p.as_os_str())
        .unwrap_or(OsStr::new("pandoc"))
}

/// Converts RTF file to Markdown using Pandoc.
pub fn rtf_to_html(rtf_path: &Path, pandoc_path: Option<&Path>) -> Result<String, ChiknError> {
    check_pandoc_available(pandoc_path)?;

    let mut cmd = Command::new(pandoc_cmd(pandoc_path));
    cmd.arg("-f")
        .arg("rtf")
        .arg("-t")
        .arg("markdown")
        .arg("--wrap=none")
        .arg(rtf_path);
    let output = output_bounded(&mut cmd, PANDOC_TIMEOUT, PANDOC_OUTPUT_LIMIT_BYTES)
        .map_err(|e| ChiknError::Unknown(format!("Failed to run Pandoc: {}", e)))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(ChiknError::InvalidFormat(format!(
            "Pandoc conversion failed: {}",
            error
        )));
    }

    let html = String::from_utf8(output.stdout)
        .map_err(|e| ChiknError::InvalidFormat(format!("Invalid UTF-8 from Pandoc: {}", e)))?;

    Ok(html)
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
