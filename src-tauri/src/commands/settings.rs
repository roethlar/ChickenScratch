use chickenscratch_core::ChiknError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// ── App Settings ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default)]
    pub general: GeneralSettings,
    #[serde(default)]
    pub writing: WritingSettings,
    #[serde(default)]
    pub backup: BackupSettings,
    #[serde(default)]
    pub ai: AiSettings,
    #[serde(default)]
    pub compile: CompileSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            general: GeneralSettings::default(),
            writing: WritingSettings::default(),
            backup: BackupSettings::default(),
            ai: AiSettings::default(),
            compile: CompileSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralSettings {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_ten")]
    pub recent_projects_limit: usize,
    #[serde(default)]
    pub pandoc_path: Option<String>,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            recent_projects_limit: 10,
            pandoc_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WritingSettings {
    #[serde(default = "default_font")]
    pub font_family: String,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    #[serde(default = "default_paragraph_style")]
    pub paragraph_style: String, // "block" or "indent"
    #[serde(default = "default_auto_save")]
    pub auto_save_seconds: u32,
    #[serde(default = "default_true")]
    pub spell_check: bool,
}

impl Default for WritingSettings {
    fn default() -> Self {
        Self {
            font_family: "Literata Variable".to_string(),
            font_size: 18.0,
            paragraph_style: "block".to_string(),
            auto_save_seconds: 2,
            spell_check: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSettings {
    #[serde(default)]
    pub backup_directory: Option<String>,
    #[serde(default = "default_true")]
    pub auto_backup_on_close: bool,
    #[serde(default = "default_backup_interval")]
    pub auto_backup_minutes: u32,
}

impl Default for BackupSettings {
    fn default() -> Self {
        Self {
            backup_directory: None,
            auto_backup_on_close: true,
            auto_backup_minutes: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_model")]
    pub model: String,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            provider: "ollama".to_string(),
            endpoint: Some("http://localhost:11434".to_string()),
            api_key: None,
            model: "llama3.2".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileSettings {
    #[serde(default = "default_format")]
    pub default_format: String,
    #[serde(default = "default_compile_font")]
    pub font: String,
    #[serde(default = "default_twelve")]
    pub font_size: f32,
    #[serde(default = "default_double")]
    pub line_spacing: f32,
    #[serde(default = "default_one")]
    pub margin_inches: f32,
}

impl Default for CompileSettings {
    fn default() -> Self {
        Self {
            default_format: "docx".to_string(),
            font: "Times New Roman".to_string(),
            font_size: 12.0,
            line_spacing: 2.0,
            margin_inches: 1.0,
        }
    }
}

fn default_true() -> bool { true }
fn default_theme() -> String { "dark".to_string() }
fn default_ten() -> usize { 10 }
fn default_font() -> String { "Literata Variable".to_string() }
fn default_font_size() -> f32 { 18.0 }
fn default_paragraph_style() -> String { "block".to_string() }
fn default_auto_save() -> u32 { 2 }
fn default_backup_interval() -> u32 { 30 }
fn default_provider() -> String { "ollama".to_string() }
fn default_model() -> String { "llama3.2".to_string() }
fn default_format() -> String { "docx".to_string() }
fn default_compile_font() -> String { "Times New Roman".to_string() }
fn default_twelve() -> f32 { 12.0 }
fn default_double() -> f32 { 2.0 }
fn default_one() -> f32 { 1.0 }

// ── Persistence ───────────────────────────────────────

fn config_dir() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("chickenscratch");
    fs::create_dir_all(&path).ok();
    path
}

fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

fn recent_path() -> PathBuf {
    config_dir().join("recent-projects.json")
}

#[tauri::command]
pub fn get_app_settings() -> AppSettings {
    let path = settings_path();
    if path.exists() {
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        AppSettings::default()
    }
}

#[tauri::command]
pub fn save_app_settings(settings: AppSettings) -> Result<(), ChiknError> {
    let json = serde_json::to_string_pretty(&settings)
        .map_err(|e| ChiknError::Unknown(format!("Failed to serialize settings: {}", e)))?;
    fs::write(settings_path(), json)?;
    Ok(())
}

// ── Recent Projects ───────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentProject {
    pub name: String,
    pub path: String,
}

#[tauri::command]
pub fn get_recent_projects() -> Vec<RecentProject> {
    let path = recent_path();
    if path.exists() {
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    }
}

#[tauri::command]
pub fn add_recent_project(name: String, path: String) -> Result<(), ChiknError> {
    let settings = get_app_settings();
    let mut recent = get_recent_projects();
    recent.retain(|r| r.path != path);
    recent.insert(0, RecentProject { name, path });
    recent.truncate(settings.general.recent_projects_limit);

    let json = serde_json::to_string_pretty(&recent)
        .map_err(|e| ChiknError::Unknown(format!("Failed to serialize: {}", e)))?;
    fs::write(recent_path(), json)?;
    Ok(())
}

// ── Pandoc ────────────────────────────────────────────

#[tauri::command]
pub fn check_pandoc() -> Result<String, ChiknError> {
    let settings = get_app_settings();

    // If user configured a specific path, try that first
    if let Some(ref custom) = settings.general.pandoc_path {
        if let Ok(output) = std::process::Command::new(custom)
            .arg("--version")
            .output()
        {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout);
                return Ok(version.lines().next().unwrap_or("unknown").to_string());
            }
        }
    }

    // Check common install locations
    let candidates = [
        "pandoc",
        "/usr/local/bin/pandoc",
        "/opt/homebrew/bin/pandoc",
        "/usr/bin/pandoc",
    ];

    for pandoc in &candidates {
        if let Ok(output) = std::process::Command::new(pandoc)
            .arg("--version")
            .output()
        {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout);
                return Ok(version.lines().next().unwrap_or("unknown").to_string());
            }
        }
    }

    Err(ChiknError::Unknown(
        "Pandoc is not installed. Required for Scrivener import and manuscript export."
            .to_string(),
    ))
}
