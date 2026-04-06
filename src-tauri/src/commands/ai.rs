use chickenscratch_core::ChiknError;
use std::process::Command;

use super::settings::{get_app_settings, AiSettings};

/// Get AI settings from the unified app settings
#[tauri::command]
pub fn get_ai_settings() -> AiSettings {
    get_app_settings().ai
}

/// Save AI settings back to the unified app settings
#[tauri::command]
pub fn save_ai_settings(ai: AiSettings) -> Result<(), ChiknError> {
    let mut settings = get_app_settings();
    settings.ai = ai;
    super::settings::save_app_settings(settings)
}

#[tauri::command]
pub fn ai_summarize(content: String) -> Result<String, ChiknError> {
    let settings = get_app_settings();
    if !settings.ai.enabled {
        return Err(ChiknError::Unknown("AI features are disabled. Enable in Settings.".to_string()));
    }

    let plain = strip_html(&content);
    if plain.trim().is_empty() {
        return Ok(String::new());
    }

    let excerpt = if plain.len() > 2000 {
        &plain[..2000]
    } else {
        &plain
    };

    let prompt = format!(
        "Summarize this scene in one sentence (max 20 words). Just the summary, no preamble:\n\n{}",
        excerpt
    );

    match settings.ai.provider.as_str() {
        "ollama" => call_ollama(&settings.ai, &prompt),
        "anthropic" => call_anthropic(&settings.ai, &prompt),
        "openai" => call_openai(&settings.ai, &prompt),
        other => Err(ChiknError::Unknown(format!("Unknown AI provider: {}", other))),
    }
}

fn strip_html(html: &str) -> String {
    regex::Regex::new(r"<[^>]*>").unwrap().replace_all(html, "").to_string()
}

fn call_ollama(settings: &AiSettings, prompt: &str) -> Result<String, ChiknError> {
    let endpoint = settings.endpoint.as_deref().unwrap_or("http://localhost:11434");
    let url = format!("{}/api/generate", endpoint);

    let body = serde_json::json!({
        "model": settings.model,
        "prompt": prompt,
        "stream": false
    });

    let output = Command::new("curl")
        .arg("-s").arg("-X").arg("POST").arg(&url)
        .arg("-H").arg("Content-Type: application/json")
        .arg("-d").arg(body.to_string())
        .output()
        .map_err(|e| ChiknError::Unknown(format!("Failed to call Ollama: {}", e)))?;

    if !output.status.success() {
        return Err(ChiknError::Unknown("Ollama request failed. Is Ollama running?".to_string()));
    }

    let response: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| ChiknError::Unknown(format!("Invalid Ollama response: {}", e)))?;

    Ok(response["response"].as_str().unwrap_or("").trim().to_string())
}

fn call_anthropic(settings: &AiSettings, prompt: &str) -> Result<String, ChiknError> {
    let api_key = settings.api_key.as_deref()
        .ok_or_else(|| ChiknError::Unknown("Anthropic API key not configured. Set it in Settings > AI.".to_string()))?;

    let body = serde_json::json!({
        "model": settings.model,
        "max_tokens": 100,
        "messages": [{"role": "user", "content": prompt}]
    });

    let output = Command::new("curl")
        .arg("-s").arg("-X").arg("POST")
        .arg("https://api.anthropic.com/v1/messages")
        .arg("-H").arg("Content-Type: application/json")
        .arg("-H").arg(format!("x-api-key: {}", api_key))
        .arg("-H").arg("anthropic-version: 2023-06-01")
        .arg("-d").arg(body.to_string())
        .output()
        .map_err(|e| ChiknError::Unknown(format!("Failed to call Anthropic: {}", e)))?;

    let response: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| ChiknError::Unknown(format!("Invalid Anthropic response: {}", e)))?;

    Ok(response["content"][0]["text"].as_str().unwrap_or("").trim().to_string())
}

fn call_openai(settings: &AiSettings, prompt: &str) -> Result<String, ChiknError> {
    let api_key = settings.api_key.as_deref()
        .ok_or_else(|| ChiknError::Unknown("OpenAI API key not configured. Set it in Settings > AI.".to_string()))?;

    let endpoint = settings.endpoint.as_deref().unwrap_or("https://api.openai.com");

    let body = serde_json::json!({
        "model": settings.model,
        "max_tokens": 100,
        "messages": [{"role": "user", "content": prompt}]
    });

    let output = Command::new("curl")
        .arg("-s").arg("-X").arg("POST")
        .arg(format!("{}/v1/chat/completions", endpoint))
        .arg("-H").arg("Content-Type: application/json")
        .arg("-H").arg(format!("Authorization: Bearer {}", api_key))
        .arg("-d").arg(body.to_string())
        .output()
        .map_err(|e| ChiknError::Unknown(format!("Failed to call OpenAI: {}", e)))?;

    let response: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| ChiknError::Unknown(format!("Invalid OpenAI response: {}", e)))?;

    Ok(response["choices"][0]["message"]["content"].as_str().unwrap_or("").trim().to_string())
}
