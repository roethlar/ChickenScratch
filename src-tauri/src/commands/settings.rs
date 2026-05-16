use chickenscratch_core::ChiknError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use super::ai::openai_chat_completions_url;

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
    pub remote: RemoteSettings,
    #[serde(default)]
    pub ai: AiSettings,
    #[serde(default)]
    pub compile: CompileSettings,
    #[serde(default = "default_shortcuts")]
    pub shortcuts: std::collections::HashMap<String, String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            general: GeneralSettings::default(),
            writing: WritingSettings::default(),
            backup: BackupSettings::default(),
            remote: RemoteSettings::default(),
            ai: AiSettings::default(),
            compile: CompileSettings::default(),
            shortcuts: default_shortcuts(),
        }
    }
}

fn default_shortcuts() -> std::collections::HashMap<String, String> {
    let mut m = std::collections::HashMap::new();
    m.insert("save".into(), "Ctrl+S".into());
    m.insert("newDocument".into(), "Ctrl+N".into());
    m.insert("search".into(), "Ctrl+Shift+P".into());
    m.insert("commandPalette".into(), "Ctrl+K".into());
    m.insert("focusMode".into(), "Ctrl+Shift+F".into());
    m.insert("toggleBinder".into(), "Ctrl+\\".into());
    m.insert("toggleInspector".into(), "Ctrl+Shift+I".into());
    m.insert("find".into(), "Ctrl+F".into());
    m.insert("findReplace".into(), "Ctrl+H".into());
    m.insert("print".into(), "Ctrl+P".into());
    m
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RemoteSettings {
    /// Git URL (https://…, git@host:…, or file://… for local testing).
    #[serde(default)]
    pub url: Option<String>,
    /// HTTPS username. For GitHub PATs, any non-empty value works.
    #[serde(default)]
    pub username: Option<String>,
    /// HTTPS personal access token. Stored in the OS keyring.
    #[serde(default)]
    pub token: Option<String>,
    /// Whether a token is configured in the OS keyring.
    #[serde(default)]
    pub token_in_keyring: bool,
    /// Auto-push to the remote on named revision (like auto-backup).
    #[serde(default)]
    pub auto_push_on_revision: bool,
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
    #[serde(default)]
    pub api_key_in_keyring: bool,
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
            api_key_in_keyring: false,
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

fn default_true() -> bool {
    true
}
fn default_theme() -> String {
    "dark".to_string()
}
fn default_ten() -> usize {
    10
}
fn default_font() -> String {
    "Literata Variable".to_string()
}
fn default_font_size() -> f32 {
    18.0
}
fn default_paragraph_style() -> String {
    "block".to_string()
}
fn default_auto_save() -> u32 {
    2
}
fn default_backup_interval() -> u32 {
    30
}
fn default_provider() -> String {
    "ollama".to_string()
}
fn default_model() -> String {
    "llama3.2".to_string()
}
fn default_format() -> String {
    "docx".to_string()
}
fn default_compile_font() -> String {
    "Times New Roman".to_string()
}
fn default_twelve() -> f32 {
    12.0
}
fn default_double() -> f32 {
    2.0
}
fn default_one() -> f32 {
    1.0
}

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

// ── Secret Storage ────────────────────────────────────

const KEYRING_ACCOUNT: &str = "default";
const REMOTE_TOKEN_SERVICE: &str = "chickenscratch.remote.token.sync";

trait SecretStore {
    fn get_secret(&self, service: &str, account: &str) -> Result<Option<String>, ChiknError>;
    fn set_secret(&self, service: &str, account: &str, secret: &str) -> Result<(), ChiknError>;
    fn delete_secret(&self, service: &str, account: &str) -> Result<(), ChiknError>;
}

struct OsKeyring;

impl OsKeyring {
    fn entry(service: &str, account: &str) -> Result<keyring::Entry, ChiknError> {
        keyring::Entry::new(service, account).map_err(keyring_error)
    }
}

impl SecretStore for OsKeyring {
    fn get_secret(&self, service: &str, account: &str) -> Result<Option<String>, ChiknError> {
        match Self::entry(service, account)?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(err) => Err(keyring_error(err)),
        }
    }

    fn set_secret(&self, service: &str, account: &str, secret: &str) -> Result<(), ChiknError> {
        Self::entry(service, account)?
            .set_password(secret)
            .map_err(keyring_error)
    }

    fn delete_secret(&self, service: &str, account: &str) -> Result<(), ChiknError> {
        match Self::entry(service, account)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(keyring_error(err)),
        }
    }
}

fn keyring_error(err: keyring::Error) -> ChiknError {
    ChiknError::Unknown(format!("Keyring error: {}", err))
}

fn non_empty_secret(secret: &Option<String>) -> Option<&str> {
    secret.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

fn keyring_component(value: &str) -> String {
    let normalized: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = normalized.trim_matches('-');
    if trimmed.is_empty() {
        "default".to_string()
    } else {
        trimmed.to_string()
    }
}

fn ai_api_key_service(provider: &str) -> String {
    format!("chickenscratch.ai.api_key.{}", keyring_component(provider))
}

fn get_keyring_secret(
    store: &impl SecretStore,
    service: &str,
) -> Result<Option<String>, ChiknError> {
    store.get_secret(service, KEYRING_ACCOUNT)
}

fn set_keyring_secret(
    store: &impl SecretStore,
    service: &str,
    secret: &str,
) -> Result<(), ChiknError> {
    store.set_secret(service, KEYRING_ACCOUNT, secret)
}

fn delete_keyring_secret(store: &impl SecretStore, service: &str) -> Result<(), ChiknError> {
    store.delete_secret(service, KEYRING_ACCOUNT)
}

fn read_settings_from_path(path: &Path) -> AppSettings {
    if path.exists() {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        AppSettings::default()
    }
}

fn write_settings_to_path(path: &Path, settings: &AppSettings) -> Result<(), ChiknError> {
    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| ChiknError::Unknown(format!("Failed to serialize settings: {}", e)))?;
    fs::write(path, json)?;
    Ok(())
}

fn migrate_plaintext_secrets(
    path: &Path,
    settings: &mut AppSettings,
    store: &impl SecretStore,
) -> Result<(), ChiknError> {
    let mut migrated = false;

    if let Some(api_key) = non_empty_secret(&settings.ai.api_key) {
        set_keyring_secret(store, &ai_api_key_service(&settings.ai.provider), api_key)?;
        settings.ai.api_key = None;
        settings.ai.api_key_in_keyring = true;
        migrated = true;
    }

    if let Some(token) = non_empty_secret(&settings.remote.token) {
        set_keyring_secret(store, REMOTE_TOKEN_SERVICE, token)?;
        settings.remote.token = None;
        settings.remote.token_in_keyring = true;
        migrated = true;
    }

    if migrated {
        write_settings_to_path(path, settings)?;
    }

    Ok(())
}

fn hydrate_secrets(settings: &mut AppSettings, store: &impl SecretStore) -> Result<(), ChiknError> {
    let ai_service = ai_api_key_service(&settings.ai.provider);
    let api_key = get_keyring_secret(store, &ai_service)?;
    if settings.ai.api_key.is_none() {
        settings.ai.api_key = api_key.clone();
    }
    settings.ai.api_key_in_keyring = api_key
        .as_deref()
        .is_some_and(|secret| !secret.trim().is_empty())
        || non_empty_secret(&settings.ai.api_key).is_some();

    let token = get_keyring_secret(store, REMOTE_TOKEN_SERVICE)?;
    if settings.remote.token.is_none() {
        settings.remote.token = token.clone();
    }
    settings.remote.token_in_keyring = token
        .as_deref()
        .is_some_and(|secret| !secret.trim().is_empty())
        || non_empty_secret(&settings.remote.token).is_some();
    Ok(())
}

fn redact_secrets(settings: &mut AppSettings) {
    settings.ai.api_key_in_keyring =
        settings.ai.api_key_in_keyring || non_empty_secret(&settings.ai.api_key).is_some();
    settings.remote.token_in_keyring =
        settings.remote.token_in_keyring || non_empty_secret(&settings.remote.token).is_some();
    settings.ai.api_key = None;
    settings.remote.token = None;
}

fn load_app_settings_from_path(
    path: &Path,
    store: &impl SecretStore,
    hydrate: bool,
) -> AppSettings {
    let mut settings = read_settings_from_path(path);
    let _ = migrate_plaintext_secrets(path, &mut settings, store);
    if hydrate {
        let _ = hydrate_secrets(&mut settings, store);
    }
    settings
}

fn save_app_settings_to_path(
    path: &Path,
    mut settings: AppSettings,
    store: &impl SecretStore,
) -> Result<(), ChiknError> {
    validate_app_settings(&settings)?;

    let mut existing = read_settings_from_path(path);
    migrate_plaintext_secrets(path, &mut existing, store)?;

    let ai_service = ai_api_key_service(&settings.ai.provider);
    if let Some(api_key) = non_empty_secret(&settings.ai.api_key) {
        set_keyring_secret(store, &ai_service, api_key)?;
        settings.ai.api_key_in_keyring = true;
    } else if settings.ai.api_key_in_keyring {
        settings.ai.api_key_in_keyring =
            get_keyring_secret(store, &ai_service)?.is_some_and(|secret| !secret.trim().is_empty());
    } else {
        delete_keyring_secret(store, &ai_service)?;
        settings.ai.api_key_in_keyring = false;
    }
    settings.ai.api_key = None;

    if let Some(token) = non_empty_secret(&settings.remote.token) {
        set_keyring_secret(store, REMOTE_TOKEN_SERVICE, token)?;
        settings.remote.token_in_keyring = true;
    } else if settings.remote.token_in_keyring {
        settings.remote.token_in_keyring = get_keyring_secret(store, REMOTE_TOKEN_SERVICE)?
            .is_some_and(|secret| !secret.trim().is_empty());
    } else {
        delete_keyring_secret(store, REMOTE_TOKEN_SERVICE)?;
        settings.remote.token_in_keyring = false;
    }
    settings.remote.token = None;

    write_settings_to_path(path, &settings)
}

#[tauri::command]
pub fn get_app_settings() -> AppSettings {
    let mut settings = load_app_settings_from_path(&settings_path(), &OsKeyring, true);
    redact_secrets(&mut settings);
    settings
}

pub(super) fn get_app_settings_hydrated() -> AppSettings {
    load_app_settings_from_path(&settings_path(), &OsKeyring, true)
}

#[tauri::command]
pub fn save_app_settings(settings: AppSettings) -> Result<(), ChiknError> {
    save_app_settings_to_path(&settings_path(), settings, &OsKeyring)
}

fn validate_app_settings(settings: &AppSettings) -> Result<(), ChiknError> {
    if settings.ai.provider == "openai" {
        openai_chat_completions_url(settings.ai.endpoint.as_deref())
            .map_err(ChiknError::InvalidFormat)?;
    }

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
        if let Ok(output) = std::process::Command::new(custom).arg("--version").output() {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout);
                return Ok(version.lines().next().unwrap_or("unknown").to_string());
            }
        }
    }

    // Check common install locations
    #[cfg(target_os = "windows")]
    let candidates: &[&str] = &["pandoc", "pandoc.exe"];

    #[cfg(not(target_os = "windows"))]
    let candidates: &[&str] = &[
        "pandoc",
        "/usr/local/bin/pandoc",
        "/opt/homebrew/bin/pandoc",
        "/usr/bin/pandoc",
    ];

    for pandoc in candidates {
        if let Ok(output) = std::process::Command::new(pandoc).arg("--version").output() {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout);
                return Ok(version.lines().next().unwrap_or("unknown").to_string());
            }
        }
    }

    Err(ChiknError::Unknown(
        "Pandoc is not installed. Required for Scrivener import and manuscript export.".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    fn app_settings_for_ai(provider: &str, endpoint: Option<&str>) -> AppSettings {
        AppSettings {
            ai: AiSettings {
                provider: provider.to_string(),
                endpoint: endpoint.map(str::to_string),
                ..AiSettings::default()
            },
            ..AppSettings::default()
        }
    }

    #[derive(Default)]
    struct FakeSecretStore {
        secrets: Mutex<HashMap<(String, String), String>>,
    }

    impl FakeSecretStore {
        fn secret(&self, service: &str) -> Option<String> {
            self.secrets
                .lock()
                .unwrap()
                .get(&(service.to_string(), KEYRING_ACCOUNT.to_string()))
                .cloned()
        }

        fn set_test_secret(&self, service: &str, secret: &str) {
            self.set_secret(service, KEYRING_ACCOUNT, secret).unwrap();
        }
    }

    impl SecretStore for FakeSecretStore {
        fn get_secret(&self, service: &str, account: &str) -> Result<Option<String>, ChiknError> {
            Ok(self
                .secrets
                .lock()
                .unwrap()
                .get(&(service.to_string(), account.to_string()))
                .cloned())
        }

        fn set_secret(&self, service: &str, account: &str, secret: &str) -> Result<(), ChiknError> {
            self.secrets.lock().unwrap().insert(
                (service.to_string(), account.to_string()),
                secret.to_string(),
            );
            Ok(())
        }

        fn delete_secret(&self, service: &str, account: &str) -> Result<(), ChiknError> {
            self.secrets
                .lock()
                .unwrap()
                .remove(&(service.to_string(), account.to_string()));
            Ok(())
        }
    }

    struct FailingSetSecretStore;

    impl SecretStore for FailingSetSecretStore {
        fn get_secret(&self, _service: &str, _account: &str) -> Result<Option<String>, ChiknError> {
            Ok(None)
        }

        fn set_secret(
            &self,
            _service: &str,
            _account: &str,
            _secret: &str,
        ) -> Result<(), ChiknError> {
            Err(ChiknError::Unknown("keyring unavailable".to_string()))
        }

        fn delete_secret(&self, _service: &str, _account: &str) -> Result<(), ChiknError> {
            Ok(())
        }
    }

    fn read_json_settings(path: &Path) -> AppSettings {
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn validate_app_settings_allows_empty_openai_endpoint_for_default() {
        let settings = app_settings_for_ai("openai", None);
        assert!(validate_app_settings(&settings).is_ok());

        let settings = app_settings_for_ai("openai", Some(" "));
        assert!(validate_app_settings(&settings).is_ok());
    }

    #[test]
    fn validate_app_settings_accepts_openai_https_endpoint() {
        let settings = app_settings_for_ai("openai", Some("https://api.openai.com"));
        assert!(validate_app_settings(&settings).is_ok());
    }

    #[test]
    fn validate_app_settings_rejects_openai_http_endpoint() {
        let settings = app_settings_for_ai("openai", Some("http://api.openai.com"));
        assert!(matches!(
            validate_app_settings(&settings),
            Err(ChiknError::InvalidFormat(_))
        ));
    }

    #[test]
    fn save_app_settings_rejects_openai_http_endpoint_before_write() {
        let settings = app_settings_for_ai("openai", Some("http://api.openai.com"));
        assert!(matches!(
            save_app_settings(settings),
            Err(ChiknError::InvalidFormat(_))
        ));
    }

    #[test]
    fn validate_app_settings_rejects_openai_malformed_and_no_scheme_endpoint() {
        let settings = app_settings_for_ai("openai", Some("api.openai.com"));
        assert!(matches!(
            validate_app_settings(&settings),
            Err(ChiknError::InvalidFormat(_))
        ));

        let settings = app_settings_for_ai("openai", Some("https://"));
        assert!(matches!(
            validate_app_settings(&settings),
            Err(ChiknError::InvalidFormat(_))
        ));
    }

    #[test]
    fn validate_app_settings_allows_ollama_local_http_endpoint() {
        let settings = app_settings_for_ai("ollama", Some("http://localhost:11434"));
        assert!(validate_app_settings(&settings).is_ok());

        let settings = app_settings_for_ai("ollama", Some("http://127.0.0.1:11434"));
        assert!(validate_app_settings(&settings).is_ok());
    }

    #[test]
    fn get_app_settings_migrates_plaintext_secrets_and_redacts_public_result() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let store = FakeSecretStore::default();
        let settings = AppSettings {
            ai: AiSettings {
                provider: "openai".to_string(),
                api_key: Some("sk-old".to_string()),
                ..AiSettings::default()
            },
            remote: RemoteSettings {
                token: Some("ghp-old".to_string()),
                ..RemoteSettings::default()
            },
            ..AppSettings::default()
        };
        write_settings_to_path(&path, &settings).unwrap();

        let mut public = load_app_settings_from_path(&path, &store, true);
        redact_secrets(&mut public);

        assert_eq!(public.ai.api_key, None);
        assert!(public.ai.api_key_in_keyring);
        assert_eq!(public.remote.token, None);
        assert!(public.remote.token_in_keyring);
        assert_eq!(
            store.secret(&ai_api_key_service("openai")),
            Some("sk-old".to_string())
        );
        assert_eq!(
            store.secret(REMOTE_TOKEN_SERVICE),
            Some("ghp-old".to_string())
        );

        let persisted = read_json_settings(&path);
        assert_eq!(persisted.ai.api_key, None);
        assert!(persisted.ai.api_key_in_keyring);
        assert_eq!(persisted.remote.token, None);
        assert!(persisted.remote.token_in_keyring);
    }

    #[test]
    fn hydrated_settings_reads_keyring_secrets_for_internal_callers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let store = FakeSecretStore::default();
        store.set_test_secret(&ai_api_key_service("openai"), "sk-live");
        store.set_test_secret(REMOTE_TOKEN_SERVICE, "ghp-live");
        write_settings_to_path(
            &path,
            &AppSettings {
                ai: AiSettings {
                    provider: "openai".to_string(),
                    endpoint: None,
                    api_key_in_keyring: true,
                    ..AiSettings::default()
                },
                remote: RemoteSettings {
                    token_in_keyring: true,
                    ..RemoteSettings::default()
                },
                ..AppSettings::default()
            },
        )
        .unwrap();

        let hydrated = load_app_settings_from_path(&path, &store, true);

        assert_eq!(hydrated.ai.api_key, Some("sk-live".to_string()));
        assert_eq!(hydrated.remote.token, Some("ghp-live".to_string()));
    }

    #[test]
    fn save_app_settings_preserves_keyring_secrets_when_public_fields_are_redacted() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let store = FakeSecretStore::default();
        store.set_test_secret(&ai_api_key_service("openai"), "sk-existing");
        store.set_test_secret(REMOTE_TOKEN_SERVICE, "ghp-existing");

        let incoming = AppSettings {
            general: GeneralSettings {
                theme: "light".to_string(),
                ..GeneralSettings::default()
            },
            ai: AiSettings {
                provider: "openai".to_string(),
                endpoint: None,
                api_key: None,
                api_key_in_keyring: true,
                ..AiSettings::default()
            },
            remote: RemoteSettings {
                token: None,
                token_in_keyring: true,
                ..RemoteSettings::default()
            },
            ..AppSettings::default()
        };

        save_app_settings_to_path(&path, incoming, &store).unwrap();

        assert_eq!(
            store.secret(&ai_api_key_service("openai")),
            Some("sk-existing".to_string())
        );
        assert_eq!(
            store.secret(REMOTE_TOKEN_SERVICE),
            Some("ghp-existing".to_string())
        );
        let persisted = read_json_settings(&path);
        assert_eq!(persisted.general.theme, "light");
        assert_eq!(persisted.ai.api_key, None);
        assert!(persisted.ai.api_key_in_keyring);
        assert_eq!(persisted.remote.token, None);
        assert!(persisted.remote.token_in_keyring);
    }

    #[test]
    fn save_app_settings_replaces_non_empty_incoming_secrets() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let store = FakeSecretStore::default();
        store.set_test_secret(&ai_api_key_service("openai"), "sk-existing");
        store.set_test_secret(REMOTE_TOKEN_SERVICE, "ghp-existing");

        save_app_settings_to_path(
            &path,
            AppSettings {
                ai: AiSettings {
                    provider: "openai".to_string(),
                    endpoint: None,
                    api_key: Some("sk-new".to_string()),
                    ..AiSettings::default()
                },
                remote: RemoteSettings {
                    token: Some("ghp-new".to_string()),
                    ..RemoteSettings::default()
                },
                ..AppSettings::default()
            },
            &store,
        )
        .unwrap();

        assert_eq!(
            store.secret(&ai_api_key_service("openai")),
            Some("sk-new".to_string())
        );
        assert_eq!(
            store.secret(REMOTE_TOKEN_SERVICE),
            Some("ghp-new".to_string())
        );
        let persisted = read_json_settings(&path);
        assert_eq!(persisted.ai.api_key, None);
        assert!(persisted.ai.api_key_in_keyring);
        assert_eq!(persisted.remote.token, None);
        assert!(persisted.remote.token_in_keyring);
    }

    #[test]
    fn save_app_settings_deletes_secrets_when_flags_are_cleared() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let store = FakeSecretStore::default();
        store.set_test_secret(&ai_api_key_service("openai"), "sk-existing");
        store.set_test_secret(REMOTE_TOKEN_SERVICE, "ghp-existing");

        save_app_settings_to_path(
            &path,
            AppSettings {
                ai: AiSettings {
                    provider: "openai".to_string(),
                    endpoint: None,
                    api_key: None,
                    api_key_in_keyring: false,
                    ..AiSettings::default()
                },
                remote: RemoteSettings {
                    token: Some(" ".to_string()),
                    token_in_keyring: false,
                    ..RemoteSettings::default()
                },
                ..AppSettings::default()
            },
            &store,
        )
        .unwrap();

        assert_eq!(store.secret(&ai_api_key_service("openai")), None);
        assert_eq!(store.secret(REMOTE_TOKEN_SERVICE), None);
        let persisted = read_json_settings(&path);
        assert_eq!(persisted.ai.api_key, None);
        assert!(!persisted.ai.api_key_in_keyring);
        assert_eq!(persisted.remote.token, None);
        assert!(!persisted.remote.token_in_keyring);
    }

    #[test]
    fn ai_keyring_entries_are_provider_specific() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let store = FakeSecretStore::default();
        store.set_test_secret(&ai_api_key_service("anthropic"), "sk-ant");

        write_settings_to_path(
            &path,
            &AppSettings {
                ai: AiSettings {
                    provider: "openai".to_string(),
                    endpoint: None,
                    api_key: None,
                    api_key_in_keyring: true,
                    ..AiSettings::default()
                },
                ..AppSettings::default()
            },
        )
        .unwrap();

        let openai = load_app_settings_from_path(&path, &store, true);
        assert_eq!(openai.ai.api_key, None);
        assert!(!openai.ai.api_key_in_keyring);

        save_app_settings_to_path(&path, openai, &store).unwrap();
        assert_eq!(
            store.secret(&ai_api_key_service("anthropic")),
            Some("sk-ant".to_string())
        );
        assert_eq!(store.secret(&ai_api_key_service("openai")), None);

        write_settings_to_path(
            &path,
            &AppSettings {
                ai: AiSettings {
                    provider: "anthropic".to_string(),
                    api_key: None,
                    api_key_in_keyring: false,
                    ..AiSettings::default()
                },
                ..AppSettings::default()
            },
        )
        .unwrap();

        let anthropic = load_app_settings_from_path(&path, &store, true);
        assert_eq!(anthropic.ai.api_key, Some("sk-ant".to_string()));
        assert!(anthropic.ai.api_key_in_keyring);
    }

    #[test]
    fn save_app_settings_keeps_plaintext_when_migration_fails() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        write_settings_to_path(
            &path,
            &AppSettings {
                ai: AiSettings {
                    api_key: Some("sk-plaintext".to_string()),
                    ..AiSettings::default()
                },
                ..AppSettings::default()
            },
        )
        .unwrap();

        let result = save_app_settings_to_path(
            &path,
            AppSettings {
                ai: AiSettings {
                    api_key: None,
                    api_key_in_keyring: true,
                    ..AiSettings::default()
                },
                ..AppSettings::default()
            },
            &FailingSetSecretStore,
        );

        assert!(result.is_err());
        let persisted = read_json_settings(&path);
        assert_eq!(persisted.ai.api_key, Some("sk-plaintext".to_string()));
        assert!(!persisted.ai.api_key_in_keyring);
    }
}
