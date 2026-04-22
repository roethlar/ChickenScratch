import { invoke } from "@tauri-apps/api/core";

export interface AppSettings {
  general: {
    theme: string;
    recent_projects_limit: number;
    pandoc_path: string | null;
  };
  writing: {
    font_family: string;
    font_size: number;
    paragraph_style: string;
    auto_save_seconds: number;
    spell_check: boolean;
  };
  backup: {
    backup_directory: string | null;
    auto_backup_on_close: boolean;
    auto_backup_minutes: number;
  };
  remote: {
    url: string | null;
    username: string | null;
    token: string | null;
    auto_push_on_revision: boolean;
  };
  ai: {
    enabled: boolean;
    provider: string;
    endpoint: string | null;
    api_key: string | null;
    model: string;
  };
  compile: {
    default_format: string;
    font: string;
    font_size: number;
    line_spacing: number;
    margin_inches: number;
  };
  shortcuts: Record<string, string>;
}

export interface RecentProject {
  name: string;
  path: string;
}

export async function getAppSettings(): Promise<AppSettings> {
  return invoke("get_app_settings");
}

export async function saveAppSettings(settings: AppSettings): Promise<void> {
  return invoke("save_app_settings", { settings });
}

export async function getRecentProjects(): Promise<RecentProject[]> {
  return invoke("get_recent_projects");
}

export async function addRecentProject(name: string, path: string): Promise<void> {
  return invoke("add_recent_project", { name, path });
}

export async function checkPandoc(): Promise<string> {
  return invoke("check_pandoc");
}
