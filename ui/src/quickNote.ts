/**
 * Quick note (PLAN_RELOAD_SAFETY_MOBILE slice 3): from the project list,
 * one tap opens a project, adds a dated note at its root and puts the cursor
 * in it. Built for a phone, useful anywhere.
 */
import * as docCmd from "./commands/document";
import { requestEditorFocus } from "./components/editor/editorRef";
import { toastError } from "./components/shared/Toast";
import { useProjectStore } from "./stores/projectStore";

/** "Note 2026-10-25 14:03" in local time. */
export function quickNoteName(now: Date = new Date()): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `Note ${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())} ${pad(
    now.getHours()
  )}:${pad(now.getMinutes())}`;
}

/**
 * Open `projectPath` and start a new note in it. Resolves true when the
 * project is open afterwards (even if the note could not be added — the
 * writer is told and can add one by hand); false when the open itself
 * failed, in which case the store carries the error for Welcome to show.
 */
export async function quickNote(projectPath: string): Promise<boolean> {
  await useProjectStore.getState().openProject(projectPath);
  const opened = useProjectStore.getState();
  if (!opened.project) return false;
  if (opened.readOnly) {
    toastError("This project opened read-only, so a note cannot be added to it.");
    return true;
  }
  const before = new Set(Object.keys(opened.project.documents));
  try {
    const updated = await docCmd.createDocument(projectPath, quickNoteName());
    const store = useProjectStore.getState();
    store.setProject(updated);
    const created = Object.keys(updated.documents).find((id) => !before.has(id));
    if (created) {
      requestEditorFocus();
      store.selectDocument(created);
    }
  } catch (e) {
    toastError(`The project is open, but the note could not be added: ${e}`);
  }
  return true;
}
