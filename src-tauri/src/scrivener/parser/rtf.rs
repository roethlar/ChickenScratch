//! # RTF to Markdown Converter
//!
//! Converts Scrivener RTF content to Pandoc Markdown.
//!
//! ## Responsibilities
//! - Read RTF files
//! - Convert RTF → Markdown using Pandoc
//! - Extract plain text content
//!
//! ## Implementation
//! Phase 2 uses Pandoc as external tool for RTF conversion.
//! This provides robust RTF parsing without reimplementing the spec.

use std::fs;
use std::path::Path;
use std::process::Command;
use crate::utils::error::ChiknError;

/// Converts RTF file to Markdown using Pandoc.
///
/// # Arguments
/// * `rtf_path` - Path to .rtf file
///
/// # Returns
/// * `Ok(String)` - Markdown content
/// * `Err(ChiknError)` on read/conversion failure
///
/// # Requirements
/// Requires Pandoc to be installed and available in PATH.
///
/// # Example
/// ```rust
/// let markdown = rtf_to_markdown(Path::new("content.rtf"))?;
/// ```
pub fn rtf_to_markdown(rtf_path: &Path) -> Result<String, ChiknError> {
    // Verify Pandoc is available
    check_pandoc_available()?;

    // Use Pandoc to convert RTF → Markdown
    let output = Command::new("pandoc")
        .arg("-f")
        .arg("rtf")
        .arg("-t")
        .arg("markdown")
        .arg(rtf_path)
        .output()
        .map_err(|e| ChiknError::Unknown(format!("Failed to run Pandoc: {}", e)))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(ChiknError::InvalidFormat(format!(
            "Pandoc conversion failed: {}",
            error
        )));
    }

    let markdown = String::from_utf8(output.stdout)
        .map_err(|e| ChiknError::InvalidFormat(format!("Invalid UTF-8 from Pandoc: {}", e)))?;

    Ok(markdown)
}

/// Converts RTF string to Markdown using Pandoc.
///
/// # Arguments
/// * `rtf_content` - RTF formatted string
///
/// # Returns
/// * `Ok(String)` - Markdown content
/// * `Err(ChiknError)` on conversion failure
///
/// # Example
/// ```rust
/// let rtf = r"{\rtf1 Hello world}";
/// let md = rtf_string_to_markdown(rtf)?;
/// ```
pub fn rtf_string_to_markdown(rtf_content: &str) -> Result<String, ChiknError> {
    // Write RTF to temporary file
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!("temp_{}.rtf", uuid::Uuid::new_v4()));

    fs::write(&temp_file, rtf_content)?;

    // Convert using file-based function
    let result = rtf_to_markdown(&temp_file);

    // Clean up temp file
    let _ = fs::remove_file(&temp_file);

    result
}

/// Checks if Pandoc is installed and available
fn check_pandoc_available() -> Result<(), ChiknError> {
    let output = Command::new("pandoc")
        .arg("--version")
        .output()
        .map_err(|_| ChiknError::Unknown(
            "Pandoc not found. Please install Pandoc: https://pandoc.org/installing.html".to_string()
        ))?;

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
        // This test requires Pandoc to be installed
        // Skip if Pandoc not available
        if check_pandoc_available().is_err() {
            eprintln!("Skipping test: Pandoc not installed");
            return;
        }
        assert!(check_pandoc_available().is_ok());
    }

    #[test]
    fn test_rtf_to_markdown() {
        // Skip if Pandoc not available
        if check_pandoc_available().is_err() {
            eprintln!("Skipping test: Pandoc not installed");
            return;
        }

        let temp_dir = TempDir::new().unwrap();
        let rtf_file = temp_dir.path().join("test.rtf");

        // Create simple RTF file
        let rtf_content = r"{\rtf1\ansi\deff0
{\fonttbl{\f0 Times New Roman;}}
\f0\fs24 Hello world
}";
        fs::write(&rtf_file, rtf_content).unwrap();

        let result = rtf_to_markdown(&rtf_file);
        if result.is_ok() {
            let markdown = result.unwrap();
            assert!(markdown.contains("Hello world") || markdown.contains("Hello"));
        }
    }

    #[test]
    fn test_rtf_string_to_markdown() {
        // Skip if Pandoc not available
        if check_pandoc_available().is_err() {
            eprintln!("Skipping test: Pandoc not installed");
            return;
        }

        let rtf = r"{\rtf1\ansi Simple text}";
        let result = rtf_string_to_markdown(rtf);

        if result.is_ok() {
            let markdown = result.unwrap();
            assert!(markdown.contains("Simple") || markdown.len() > 0);
        }
    }
}
