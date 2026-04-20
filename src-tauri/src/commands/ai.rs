use chickenscratch_core::ChiknError;

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
        return Err(ChiknError::Unknown(
            "AI features are disabled. Enable in Settings.".to_string(),
        ));
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

    call_ai(&settings.ai, &prompt, 100)
}

/// Transform selected text with AI (polish, expand, simplify, brainstorm)
#[tauri::command]
pub fn ai_transform(content: String, operation: String) -> Result<String, ChiknError> {
    let settings = get_app_settings();
    if !settings.ai.enabled {
        return Err(ChiknError::Unknown(
            "AI features are disabled. Enable in Settings.".to_string(),
        ));
    }

    let plain = strip_html(&content);
    if plain.trim().is_empty() {
        return Ok(String::new());
    }

    let excerpt = if plain.len() > 4000 {
        &plain[..4000]
    } else {
        &plain
    };

    let instruction = match operation.as_str() {
        "polish" => "Improve the writing quality of this text. Fix grammar, improve word choice, and enhance clarity. Keep the same meaning and tone. Return only the improved text, no commentary.",
        "expand" => "Expand this text with more detail, description, and depth. Keep the same style and voice. Return only the expanded text, no commentary.",
        "simplify" => "Simplify this text. Use shorter sentences, clearer language, and remove unnecessary complexity. Keep the meaning. Return only the simplified text, no commentary.",
        "brainstorm" => "Generate 3-5 alternative ways to express or continue this passage. Number each option. Be creative but stay in the same genre/style.",
        _ => "Improve this text. Return only the improved version.",
    };

    let prompt = format!("{}\n\n{}", instruction, excerpt);
    call_ai(&settings.ai, &prompt, 2000)
}

fn strip_html(html: &str) -> String {
    regex::Regex::new(r"<[^>]*>")
        .unwrap()
        .replace_all(html, "")
        .to_string()
}

/// Unified AI call using reqwest
fn call_ai(settings: &AiSettings, prompt: &str, max_tokens: u32) -> Result<String, ChiknError> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| ChiknError::Unknown(format!("HTTP client error: {}", e)))?;

    match settings.provider.as_str() {
        "ollama" => call_ollama(&client, settings, prompt),
        "anthropic" => call_anthropic(&client, settings, prompt, max_tokens),
        "openai" => call_openai(&client, settings, prompt, max_tokens),
        other => Err(ChiknError::Unknown(format!(
            "Unknown AI provider: {}",
            other
        ))),
    }
}

fn call_ollama(
    client: &reqwest::blocking::Client,
    settings: &AiSettings,
    prompt: &str,
) -> Result<String, ChiknError> {
    let endpoint = settings
        .endpoint
        .as_deref()
        .unwrap_or("http://localhost:11434");
    let url = format!("{}/api/generate", endpoint);

    let body = serde_json::json!({
        "model": settings.model,
        "prompt": prompt,
        "stream": false
    });

    let resp = client.post(&url).json(&body).send().map_err(|e| {
        ChiknError::Unknown(format!("Ollama request failed: {}. Is Ollama running?", e))
    })?;

    let json: serde_json::Value = resp
        .json()
        .map_err(|e| ChiknError::Unknown(format!("Invalid Ollama response: {}", e)))?;

    Ok(json["response"].as_str().unwrap_or("").trim().to_string())
}

fn call_anthropic(
    client: &reqwest::blocking::Client,
    settings: &AiSettings,
    prompt: &str,
    max_tokens: u32,
) -> Result<String, ChiknError> {
    let api_key = settings.api_key.as_deref().ok_or_else(|| {
        ChiknError::Unknown(
            "Anthropic API key not configured. Set it in Settings > AI.".to_string(),
        )
    })?;

    let body = serde_json::json!({
        "model": settings.model,
        "max_tokens": max_tokens,
        "messages": [{"role": "user", "content": prompt}]
    });

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("Content-Type", "application/json")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .map_err(|e| ChiknError::Unknown(format!("Anthropic request failed: {}", e)))?;

    let json: serde_json::Value = resp
        .json()
        .map_err(|e| ChiknError::Unknown(format!("Invalid Anthropic response: {}", e)))?;

    if let Some(err) = json["error"]["message"].as_str() {
        return Err(ChiknError::Unknown(format!("Anthropic error: {}", err)));
    }

    Ok(json["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string())
}

fn call_openai(
    client: &reqwest::blocking::Client,
    settings: &AiSettings,
    prompt: &str,
    max_tokens: u32,
) -> Result<String, ChiknError> {
    let api_key = settings.api_key.as_deref().ok_or_else(|| {
        ChiknError::Unknown("OpenAI API key not configured. Set it in Settings > AI.".to_string())
    })?;

    let endpoint = settings
        .endpoint
        .as_deref()
        .unwrap_or("https://api.openai.com");

    let body = serde_json::json!({
        "model": settings.model,
        "max_tokens": max_tokens,
        "messages": [{"role": "user", "content": prompt}]
    });

    let resp = client
        .post(format!("{}/v1/chat/completions", endpoint))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .map_err(|e| ChiknError::Unknown(format!("OpenAI request failed: {}", e)))?;

    let json: serde_json::Value = resp
        .json()
        .map_err(|e| ChiknError::Unknown(format!("Invalid OpenAI response: {}", e)))?;

    if let Some(err) = json["error"]["message"].as_str() {
        return Err(ChiknError::Unknown(format!("OpenAI error: {}", err)));
    }

    Ok(json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string())
}
