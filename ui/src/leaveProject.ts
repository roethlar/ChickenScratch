/**
 * The Projects button and the reload reopen
 * (docs/plans/PLAN_RELOAD_SAFETY_MOBILE.md, slice 1).
 *
 * `leaveProject` is the first way back to the project list that is not a
 * reload: it saves what the editor holds, runs the same close-time backup the
 * window-close path runs, hands the single-writer lease back at once (web
 * shell), forgets the remembered place and clears the store.
 *
 * `reopenLastProject` is what the web shell does on a page load when a
 * project was open: open it again and select the remembered note; a failure
 * leaves the store's error for Welcome to show and forgets the entry so a
 * dead project cannot loop.
 */
import { isBarrierActive } from "./commands/barrier";
import { invoke, isWeb } from "./commands/transport";
import { releaseProject } from "./commands/web";
import { flushPendingEditorSave } from "./components/editor/editorRef";
import { toastError } from "./components/shared/Toast";
import { forgetProject, getLastDocument, getLastProject } from "./lastOpen";
import { useProjectStore } from "./stores/projectStore";

/** Resolves true when the project was left; false when it stays open. */
export async function leaveProject(): Promise<boolean> {
  if (isBarrierActive()) {
    toastError("Wait for the current operation to finish before leaving the project.");
    return false;
  }
  try {
    await flushPendingEditorSave();
  } catch {
    toastError(
      "Your latest changes could not be saved, so the project stays open. Please retry."
    );
    return false;
  }
  const store = useProjectStore.getState();
  const path = store.project?.path;
  if (!path) return true;
  try {
    await invoke("backup_on_close", { projectPath: path });
  } catch {
    // Best effort, as on window close: a failed backup never traps the writer.
  }
  if (isWeb()) {
    releaseProject(path).catch(() => {
      /* the lease expires on its own after the idle period */
    });
  }
  forgetProject();
  useProjectStore.getState().closeProject();
  return true;
}

/** Web shell: reopen the remembered project and note, if any. */
export async function reopenLastProject(): Promise<void> {
  const path = getLastProject();
  if (!path) return;
  await useProjectStore.getState().openProject(path);
  const store = useProjectStore.getState();
  if (!store.project) {
    forgetProject();
    return;
  }
  const docId = getLastDocument(path);
  if (docId && store.project.documents[docId]) {
    store.selectDocument(docId);
  }
}
