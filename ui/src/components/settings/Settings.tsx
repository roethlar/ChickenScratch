import { useState, useEffect, useCallback } from "react";
import { X } from "lucide-react";
import {
  getAppSettings,
  saveAppSettings,
  type AppSettings,
} from "../../commands/settings";
import { useSettingsStore } from "../../stores/settingsStore";
import { toastSuccess, toastError } from "../shared/Toast";

type Tab = "general" | "writing" | "backup" | "ai" | "compile";

export function Settings({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [tab, setTab] = useState<Tab>("general");
  const { setTheme } = useSettingsStore();

  useEffect(() => {
    if (open) {
      getAppSettings().then(setSettings).catch(() => {});
    }
  }, [open]);

  const save = useCallback(async () => {
    if (!settings) return;
    try {
      await saveAppSettings(settings);
      // Apply theme immediately
      setTheme(settings.general.theme as "light" | "dark" | "sepia");
      toastSuccess("Settings saved");
    } catch (e) {
      toastError("Failed to save settings: " + e);
    }
  }, [settings, setTheme]);

  const update = <K extends keyof AppSettings>(
    section: K,
    field: string,
    value: unknown
  ) => {
    if (!settings) return;
    setSettings({
      ...settings,
      [section]: { ...settings[section], [field]: value },
    });
  };

  if (!open || !settings) return null;

  return (
    <div className="settings-overlay" onClick={onClose}>
      <div className="settings-panel" onClick={(e) => e.stopPropagation()}>
        <div className="settings-header">
          <h2>Settings</h2>
          <button className="settings-close" onClick={onClose}>
            <X size={18} />
          </button>
        </div>

        <div className="settings-body">
          <div className="settings-tabs">
            {(
              [
                ["general", "General"],
                ["writing", "Writing"],
                ["backup", "Backup"],
                ["ai", "AI"],
                ["compile", "Compile"],
              ] as [Tab, string][]
            ).map(([key, label]) => (
              <button
                key={key}
                className={`settings-tab ${tab === key ? "active" : ""}`}
                onClick={() => setTab(key)}
              >
                {label}
              </button>
            ))}
          </div>

          <div className="settings-content">
            {tab === "general" && (
              <div className="settings-section">
                <Field label="Theme">
                  <select
                    value={settings.general.theme}
                    onChange={(e) => update("general", "theme", e.target.value)}
                  >
                    <option value="light">Light</option>
                    <option value="dark">Dark</option>
                    <option value="sepia">Sepia</option>
                  </select>
                </Field>
                <Field label="Recent Projects Limit">
                  <input
                    type="number"
                    min={1}
                    max={50}
                    value={settings.general.recent_projects_limit}
                    onChange={(e) =>
                      update("general", "recent_projects_limit", parseInt(e.target.value) || 10)
                    }
                  />
                </Field>
                <Field label="Pandoc Path (leave empty for auto-detect)">
                  <input
                    type="text"
                    value={settings.general.pandoc_path || ""}
                    onChange={(e) =>
                      update("general", "pandoc_path", e.target.value || null)
                    }
                    placeholder="/usr/local/bin/pandoc"
                  />
                </Field>
              </div>
            )}

            {tab === "writing" && (
              <div className="settings-section">
                <Field label="Font Family">
                  <select
                    value={settings.writing.font_family}
                    onChange={(e) => update("writing", "font_family", e.target.value)}
                  >
                    <option value="Literata Variable">Literata</option>
                    <option value="Georgia">Georgia</option>
                    <option value="Times New Roman">Times New Roman</option>
                    <option value="Palatino">Palatino</option>
                    <option value="system-ui">System Default</option>
                  </select>
                </Field>
                <Field label="Font Size (px)">
                  <input
                    type="number"
                    min={12}
                    max={28}
                    value={settings.writing.font_size}
                    onChange={(e) =>
                      update("writing", "font_size", parseFloat(e.target.value) || 18)
                    }
                  />
                </Field>
                <Field label="Paragraph Style">
                  <select
                    value={settings.writing.paragraph_style}
                    onChange={(e) => update("writing", "paragraph_style", e.target.value)}
                  >
                    <option value="block">Block (spacing between paragraphs)</option>
                    <option value="indent">Indent (first-line indent, no spacing)</option>
                  </select>
                </Field>
                <Field label="Auto-Save Delay (seconds)">
                  <input
                    type="number"
                    min={1}
                    max={30}
                    value={settings.writing.auto_save_seconds}
                    onChange={(e) =>
                      update("writing", "auto_save_seconds", parseInt(e.target.value) || 2)
                    }
                  />
                </Field>
              </div>
            )}

            {tab === "backup" && (
              <div className="settings-section">
                <Field label="Backup Directory">
                  <input
                    type="text"
                    value={settings.backup.backup_directory || ""}
                    onChange={(e) =>
                      update("backup", "backup_directory", e.target.value || null)
                    }
                    placeholder="~/ChickenScratchBackups"
                  />
                  <p className="settings-hint">
                    Each project gets a git backup repository in this folder.
                    Set this to a cloud-synced folder (Dropbox, iCloud, etc.)
                    for automatic offsite backup.
                  </p>
                </Field>
                <Field label="Auto-Backup on Close">
                  <input
                    type="checkbox"
                    checked={settings.backup.auto_backup_on_close}
                    onChange={(e) =>
                      update("backup", "auto_backup_on_close", e.target.checked)
                    }
                  />
                </Field>
                <Field label="Auto-Backup Interval (minutes)">
                  <input
                    type="number"
                    min={5}
                    max={120}
                    value={settings.backup.auto_backup_minutes}
                    onChange={(e) =>
                      update("backup", "auto_backup_minutes", parseInt(e.target.value) || 30)
                    }
                  />
                </Field>
              </div>
            )}

            {tab === "ai" && (
              <div className="settings-section">
                <Field label="Enable AI Features">
                  <input
                    type="checkbox"
                    checked={settings.ai.enabled}
                    onChange={(e) => update("ai", "enabled", e.target.checked)}
                  />
                </Field>
                {settings.ai.enabled && (
                  <>
                    <Field label="Provider">
                      <select
                        value={settings.ai.provider}
                        onChange={(e) => update("ai", "provider", e.target.value)}
                      >
                        <option value="ollama">Ollama (local, no API key)</option>
                        <option value="anthropic">Anthropic (Claude)</option>
                        <option value="openai">OpenAI (ChatGPT)</option>
                      </select>
                    </Field>
                    <Field label="Model">
                      <input
                        type="text"
                        value={settings.ai.model}
                        onChange={(e) => update("ai", "model", e.target.value)}
                        placeholder={
                          settings.ai.provider === "ollama"
                            ? "llama3.2"
                            : settings.ai.provider === "anthropic"
                            ? "claude-sonnet-4-6"
                            : "gpt-4o"
                        }
                      />
                    </Field>
                    <Field label="Endpoint URL">
                      <input
                        type="text"
                        value={settings.ai.endpoint || ""}
                        onChange={(e) =>
                          update("ai", "endpoint", e.target.value || null)
                        }
                        placeholder={
                          settings.ai.provider === "ollama"
                            ? "http://localhost:11434"
                            : "Leave empty for default"
                        }
                      />
                    </Field>
                    {settings.ai.provider !== "ollama" && (
                      <Field label="API Key">
                        <input
                          type="password"
                          value={settings.ai.api_key || ""}
                          onChange={(e) =>
                            update("ai", "api_key", e.target.value || null)
                          }
                          placeholder="sk-..."
                        />
                        <p className="settings-hint">
                          Your API key is stored locally and never shared.
                        </p>
                      </Field>
                    )}
                  </>
                )}
              </div>
            )}

            {tab === "compile" && (
              <div className="settings-section">
                <Field label="Default Export Format">
                  <select
                    value={settings.compile.default_format}
                    onChange={(e) => update("compile", "default_format", e.target.value)}
                  >
                    <option value="docx">Word (.docx)</option>
                    <option value="pdf">PDF</option>
                    <option value="epub">EPUB</option>
                    <option value="html">HTML</option>
                    <option value="odt">OpenDocument (.odt)</option>
                  </select>
                </Field>
                <Field label="Manuscript Font">
                  <input
                    type="text"
                    value={settings.compile.font}
                    onChange={(e) => update("compile", "font", e.target.value)}
                  />
                </Field>
                <Field label="Font Size (pt)">
                  <input
                    type="number"
                    min={8}
                    max={18}
                    value={settings.compile.font_size}
                    onChange={(e) =>
                      update("compile", "font_size", parseFloat(e.target.value) || 12)
                    }
                  />
                </Field>
                <Field label="Line Spacing">
                  <select
                    value={settings.compile.line_spacing}
                    onChange={(e) =>
                      update("compile", "line_spacing", parseFloat(e.target.value))
                    }
                  >
                    <option value={1}>Single</option>
                    <option value={1.5}>1.5</option>
                    <option value={2}>Double</option>
                  </select>
                </Field>
                <Field label="Margins (inches)">
                  <input
                    type="number"
                    min={0.5}
                    max={2}
                    step={0.25}
                    value={settings.compile.margin_inches}
                    onChange={(e) =>
                      update("compile", "margin_inches", parseFloat(e.target.value) || 1)
                    }
                  />
                </Field>
              </div>
            )}
          </div>
        </div>

        <div className="settings-footer">
          <button className="settings-save-btn" onClick={save}>
            Save Settings
          </button>
        </div>
      </div>
    </div>
  );
}

function Field({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="settings-field">
      <label>{label}</label>
      {children}
    </div>
  );
}
