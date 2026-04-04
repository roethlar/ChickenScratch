import { invoke } from "@tauri-apps/api/core";

export interface AiSettings {
  provider: string;
  model: string;
  endpoint?: string | null;
  api_key?: string | null;
}

export async function getAiSettings(): Promise<AiSettings> {
  return invoke("get_ai_settings");
}

export async function saveAiSettings(settings: AiSettings): Promise<void> {
  return invoke("save_ai_settings", { settings });
}

export async function aiSummarize(content: string): Promise<string> {
  return invoke("ai_summarize", { content });
}
