import { invoke } from "@tauri-apps/api/core";
import type { Project, SessionTarget } from "../types";

export interface SessionProgress {
  today_words: number;
  words_per_session: number | null;
  total_target: number | null;
  deadline: string | null;
  days_remaining: number | null;
  current_total: number;
  needed_per_day: number | null;
}

export async function getSessionProgress(
  projectPath: string
): Promise<SessionProgress> {
  return invoke("get_session_progress", { projectPath });
}

export async function updateSessionTarget(
  project: Project,
  target: SessionTarget | null
): Promise<Project> {
  // Reuses update_project_metadata to keep the ProjectMeta write atomic.
  const meta = project.metadata ?? {};
  return invoke("update_project_metadata", {
    projectPath: project.path,
    title: meta.title ?? null,
    author: meta.author ?? null,
    projectType: meta.project_type ?? null,
    genre: meta.genre ?? null,
    theme: meta.theme ?? null,
    summary: meta.summary ?? null,
    sessionTarget: target,
  });
}
