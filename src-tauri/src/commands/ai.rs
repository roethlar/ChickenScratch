use chickenscratch_core::ChiknError;

use super::settings::{get_app_settings, AiSettings};

const DEFAULT_OPENAI_CHAT_COMPLETIONS_URL: &str = "https://api.openai.com/v1/chat/completions";

/// Truncate `s` at a UTF-8 boundary so it fits within `max_chars` codepoints.
///
/// Byte slicing (`&s[..n]`) panics if the boundary lands inside a multi-byte
/// codepoint. Fiction projects routinely contain curly quotes, em dashes,
/// accents, emoji, and CJK characters, so the previous `&plain[..4000]` shape
/// would crash AI requests on perfectly valid manuscripts (F-010). We count
/// codepoints, not bytes, so the limit reads as "characters" — a friendlier
/// concept for prose than UTF-8 byte counts.
fn truncate_chars(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((boundary, _)) => &s[..boundary],
        None => s,
    }
}

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

    let plain = strip_comment_spans(&content);
    if plain.trim().is_empty() {
        return Ok(String::new());
    }

    let excerpt = truncate_chars(&plain, 2000);
    let prompt = format!(
        "Summarize this scene in one sentence (max 20 words). Just the summary, no preamble:\n\n{}",
        excerpt
    );

    call_ai(&settings.ai, &prompt, 100)
}

/// Streaming variant — emits `ai:chunk` events with `{ id, delta }` and a
/// final `ai:done` event with `{ id }` (or `ai:error` `{ id, message }`).
/// The frontend subscribes by `request_id` so multiple in-flight calls don't
/// collide.
#[tauri::command]
pub fn ai_transform_stream(
    app: tauri::AppHandle,
    content: String,
    operation: String,
    request_id: String,
) -> Result<(), ChiknError> {
    let settings = get_app_settings();
    if !settings.ai.enabled {
        return Err(ChiknError::Unknown(
            "AI features are disabled. Enable in Settings.".to_string(),
        ));
    }

    let plain = strip_comment_spans(&content);
    if plain.trim().is_empty() {
        let _ = emit_done(&app, &request_id);
        return Ok(());
    }

    let excerpt = truncate_chars(&plain, 4000).to_string();
    let instruction = instruction_for(&operation);
    let prompt = format!("{}\n\n{}", instruction, excerpt);
    let max_tokens: u32 = 2000;

    // Run the streaming request on a worker thread so the command returns
    // immediately and the event loop stays responsive.
    let app_clone = app.clone();
    let ai = settings.ai.clone();
    let req_id = request_id.clone();
    std::thread::spawn(move || {
        let result = match ai.provider.as_str() {
            "ollama" => stream_ollama(&app_clone, &ai, &prompt, &req_id),
            "anthropic" => stream_anthropic(&app_clone, &ai, &prompt, max_tokens, &req_id),
            "openai" => stream_openai(&app_clone, &ai, &prompt, max_tokens, &req_id),
            other => Err(format!("Unknown AI provider: {}", other)),
        };
        match result {
            Ok(()) => {
                let _ = emit_done(&app_clone, &req_id);
            }
            Err(msg) => {
                let _ = emit_error(&app_clone, &req_id, &msg);
            }
        }
    });

    Ok(())
}

fn instruction_for(op: &str) -> &'static str {
    match op {
        "polish" => "Improve the writing quality of this text. Fix grammar, improve word choice, and enhance clarity. Keep the same meaning and tone. Return only the improved text, no commentary.",
        "expand" => "Expand this text with more detail, description, and depth. Keep the same style and voice. Return only the expanded text, no commentary.",
        "simplify" => "Simplify this text. Use shorter sentences, clearer language, and remove unnecessary complexity. Keep the meaning. Return only the simplified text, no commentary.",
        "brainstorm" => "Generate 3-5 alternative ways to express or continue this passage. Number each option. Be creative but stay in the same genre/style.",
        _ => "Improve this text. Return only the improved version.",
    }
}

fn emit_chunk(app: &tauri::AppHandle, id: &str, delta: &str) {
    use tauri::Emitter;
    let _ = app.emit(
        "ai:chunk",
        serde_json::json!({ "id": id, "delta": delta }),
    );
}
fn emit_done(app: &tauri::AppHandle, id: &str) -> Result<(), tauri::Error> {
    use tauri::Emitter;
    app.emit("ai:done", serde_json::json!({ "id": id }))
}
fn emit_error(app: &tauri::AppHandle, id: &str, msg: &str) -> Result<(), tauri::Error> {
    use tauri::Emitter;
    app.emit(
        "ai:error",
        serde_json::json!({ "id": id, "message": msg }),
    )
}

fn stream_ollama(
    app: &tauri::AppHandle,
    settings: &AiSettings,
    prompt: &str,
    req_id: &str,
) -> Result<(), String> {
    use std::io::{BufRead, BufReader};
    let endpoint = settings
        .endpoint
        .as_deref()
        .unwrap_or("http://localhost:11434");
    let url = format!("{}/api/generate", endpoint);
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("HTTP client: {}", e))?;
    let body = serde_json::json!({
        "model": settings.model,
        "prompt": prompt,
        "stream": true
    });
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .map_err(|e| format!("Ollama request failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("Ollama HTTP {}", resp.status()));
    }
    let reader = BufReader::new(resp);
    for line in reader.lines() {
        let line = line.map_err(|e| format!("Stream read: {}", e))?;
        if line.trim().is_empty() {
            continue;
        }
        let json: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Some(s) = json.get("response").and_then(|v| v.as_str()) {
            if !s.is_empty() {
                emit_chunk(app, req_id, s);
            }
        }
        if json.get("done").and_then(|v| v.as_bool()).unwrap_or(false) {
            break;
        }
    }
    Ok(())
}

fn stream_anthropic(
    app: &tauri::AppHandle,
    settings: &AiSettings,
    prompt: &str,
    max_tokens: u32,
    req_id: &str,
) -> Result<(), String> {
    use std::io::{BufRead, BufReader};
    let api_key = settings
        .api_key
        .as_deref()
        .ok_or_else(|| "Anthropic API key not configured.".to_string())?;
    let body = serde_json::json!({
        "model": settings.model,
        "max_tokens": max_tokens,
        "stream": true,
        "messages": [{"role": "user", "content": prompt}]
    });
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .map_err(|e| format!("HTTP client: {}", e))?;
    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .map_err(|e| format!("Anthropic request failed: {}", e))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!("Anthropic HTTP {}: {}", status, body));
    }
    let reader = BufReader::new(resp);
    for line in reader.lines() {
        let line = line.map_err(|e| format!("Stream read: {}", e))?;
        // SSE lines look like: `data: {...}` or `event: foo`. Skip non-data.
        let payload = match line.strip_prefix("data: ") {
            Some(p) => p,
            None => continue,
        };
        let json: serde_json::Value = match serde_json::from_str(payload) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let kind = json.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if kind == "content_block_delta" {
            if let Some(text) = json
                .pointer("/delta/text")
                .and_then(|v| v.as_str())
            {
                if !text.is_empty() {
                    emit_chunk(app, req_id, text);
                }
            }
        } else if kind == "message_stop" {
            break;
        } else if kind == "error" {
            let msg = json
                .pointer("/error/message")
                .and_then(|v| v.as_str())
                .unwrap_or("Anthropic error")
                .to_string();
            return Err(msg);
        }
    }
    Ok(())
}

fn stream_openai(
    app: &tauri::AppHandle,
    settings: &AiSettings,
    prompt: &str,
    max_tokens: u32,
    req_id: &str,
) -> Result<(), String> {
    use std::io::{BufRead, BufReader};
    let api_key = settings
        .api_key
        .as_deref()
        .ok_or_else(|| "OpenAI API key not configured.".to_string())?;
    let url = openai_chat_completions_url(settings.endpoint.as_deref())?;
    let body = serde_json::json!({
        "model": settings.model,
        "max_tokens": max_tokens,
        "stream": true,
        "messages": [{"role": "user", "content": prompt}]
    });
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .map_err(|e| format!("HTTP client: {}", e))?;
    let resp = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .map_err(|e| format!("OpenAI request failed: {}", e))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!("OpenAI HTTP {}: {}", status, body));
    }
    let reader = BufReader::new(resp);
    for line in reader.lines() {
        let line = line.map_err(|e| format!("Stream read: {}", e))?;
        let payload = match line.strip_prefix("data: ") {
            Some(p) => p,
            None => continue,
        };
        if payload.trim() == "[DONE]" {
            break;
        }
        let json: serde_json::Value = match serde_json::from_str(payload) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Some(text) = json
            .pointer("/choices/0/delta/content")
            .and_then(|v| v.as_str())
        {
            if !text.is_empty() {
                emit_chunk(app, req_id, text);
            }
        }
    }
    Ok(())
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

    let plain = strip_comment_spans(&content);
    if plain.trim().is_empty() {
        return Ok(String::new());
    }

    let excerpt = truncate_chars(&plain, 4000);

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

fn strip_comment_spans(html: &str) -> String {
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

    let url = openai_chat_completions_url(settings.endpoint.as_deref())
        .map_err(ChiknError::InvalidFormat)?;

    let body = serde_json::json!({
        "model": settings.model,
        "max_tokens": max_tokens,
        "messages": [{"role": "user", "content": prompt}]
    });

    let resp = client
        .post(url)
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

pub(super) fn openai_chat_completions_url(endpoint: Option<&str>) -> Result<reqwest::Url, String> {
    let endpoint = endpoint.map(str::trim).filter(|value| !value.is_empty());
    let Some(endpoint) = endpoint else {
        return reqwest::Url::parse(DEFAULT_OPENAI_CHAT_COMPLETIONS_URL)
            .map_err(|e| format!("Invalid built-in OpenAI endpoint: {}", e));
    };

    let mut url = reqwest::Url::parse(endpoint).map_err(|_| invalid_openai_endpoint_message())?;
    validate_openai_endpoint_url(&url)?;

    let path = url.path().trim_end_matches('/');
    if path.ends_with("/v1/chat/completions") {
        return Ok(url);
    }

    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| invalid_openai_endpoint_message())?;
        segments.pop_if_empty();
        segments.push("v1");
        segments.push("chat");
        segments.push("completions");
    }

    Ok(url)
}

fn validate_openai_endpoint_url(url: &reqwest::Url) -> Result<(), String> {
    if url.scheme() != "https" {
        return Err(invalid_openai_endpoint_message());
    }
    if url.host_str().is_none() {
        return Err(invalid_openai_endpoint_message());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(invalid_openai_endpoint_message());
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(invalid_openai_endpoint_message());
    }

    Ok(())
}

fn invalid_openai_endpoint_message() -> String {
    "Invalid OpenAI endpoint: use an HTTPS URL with a host and no username, password, query, or fragment.".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_chars_handles_emoji_at_boundary() {
        // 4-byte codepoints right at the truncation point were the original
        // panic mode (F-010). With byte slicing, &s[..2000] would land 1-3
        // bytes inside an emoji and crash.
        let s: String = "🌊".repeat(2050);
        let truncated = truncate_chars(&s, 2000);
        assert_eq!(truncated.chars().count(), 2000);
    }

    #[test]
    fn truncate_chars_returns_input_when_short() {
        let s = "hello";
        assert_eq!(truncate_chars(s, 100), "hello");
    }

    #[test]
    fn truncate_chars_handles_combining_marks() {
        // Combining marks count as separate codepoints — we don't try to be
        // grapheme-cluster aware, just UTF-8-boundary safe.
        let s = "ñü\u{0301}!".repeat(1000);
        let n = s.chars().count();
        let truncated = truncate_chars(&s, n);
        assert_eq!(truncated, s);
        let truncated = truncate_chars(&s, n - 1);
        assert!(truncated.chars().count() == n - 1);
    }

    #[test]
    fn openai_chat_url_defaults_to_official_chat_completions_endpoint() {
        let url = openai_chat_completions_url(None).unwrap();
        assert_eq!(url.as_str(), "https://api.openai.com/v1/chat/completions");

        let url = openai_chat_completions_url(Some("   ")).unwrap();
        assert_eq!(url.as_str(), "https://api.openai.com/v1/chat/completions");
    }

    #[test]
    fn openai_chat_url_accepts_https_endpoint() {
        let url = openai_chat_completions_url(Some("https://api.openai.com")).unwrap();
        assert_eq!(url.as_str(), "https://api.openai.com/v1/chat/completions");

        let url = openai_chat_completions_url(Some("https://gateway.example/openai/")).unwrap();
        assert_eq!(
            url.as_str(),
            "https://gateway.example/openai/v1/chat/completions"
        );
    }

    #[test]
    fn openai_chat_url_rejects_http_endpoint() {
        let result = openai_chat_completions_url(Some("http://api.openai.com"));
        assert!(result.is_err());
    }

    #[test]
    fn openai_chat_url_rejects_malformed_and_no_scheme_endpoint() {
        assert!(openai_chat_completions_url(Some("api.openai.com")).is_err());
        assert!(openai_chat_completions_url(Some("https://")).is_err());
    }

    #[test]
    fn openai_chat_url_rejects_userinfo_query_and_fragment() {
        assert!(openai_chat_completions_url(Some("https://user@example.com")).is_err());
        assert!(openai_chat_completions_url(Some("https://api.openai.com?x=1")).is_err());
        assert!(openai_chat_completions_url(Some("https://api.openai.com#token")).is_err());
    }
}
