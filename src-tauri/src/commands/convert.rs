//! HTML ↔ Markdown conversion via Pandoc subprocess.
//! Used by the Tauri editor: it operates on HTML (TipTap native) but
//! the canonical storage is Pandoc markdown.

use chickenscratch_core::ChiknError;
use std::io::Write;
use std::process::{Command, Stdio};

/// Convert Pandoc-flavored markdown to HTML (used when loading a doc into TipTap).
#[tauri::command]
pub fn markdown_to_html(markdown: String) -> Result<String, ChiknError> {
    if markdown.trim().is_empty() {
        return Ok(String::new());
    }
    run_pandoc(&markdown, "markdown", "html")
}

/// Convert HTML from TipTap to Pandoc markdown (used on save).
#[tauri::command]
pub fn html_to_markdown(html: String) -> Result<String, ChiknError> {
    if html.trim().is_empty() {
        return Ok(String::new());
    }
    run_pandoc(&html, "html", "markdown")
}

fn run_pandoc(input: &str, from: &str, to: &str) -> Result<String, ChiknError> {
    let pandoc = find_pandoc_bin()?;

    let mut cmd = Command::new(&pandoc)
        .arg("-f")
        .arg(from)
        .arg("-t")
        .arg(to)
        .arg("--wrap=none")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| ChiknError::Unknown(format!("Failed to start Pandoc: {}", e)))?;

    if let Some(mut stdin) = cmd.stdin.take() {
        stdin
            .write_all(input.as_bytes())
            .map_err(|e| ChiknError::Unknown(format!("Failed to write to Pandoc: {}", e)))?;
    }

    let output = cmd
        .wait_with_output()
        .map_err(|e| ChiknError::Unknown(format!("Pandoc wait failed: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ChiknError::Unknown(format!("Pandoc failed: {}", stderr)));
    }

    String::from_utf8(output.stdout)
        .map_err(|e| ChiknError::Unknown(format!("Invalid UTF-8 from Pandoc: {}", e)))
}

fn find_pandoc_bin() -> Result<String, ChiknError> {
    let settings = super::settings::get_app_settings();
    if let Some(ref p) = settings.general.pandoc_path {
        if !p.is_empty() {
            return Ok(p.clone());
        }
    }
    for candidate in &[
        "pandoc",
        "/usr/local/bin/pandoc",
        "/opt/homebrew/bin/pandoc",
        "/usr/bin/pandoc",
    ] {
        if Command::new(candidate)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Ok(candidate.to_string());
        }
    }
    Err(ChiknError::Unknown(
        "Pandoc is not installed. Required for document editing.".to_string(),
    ))
}
