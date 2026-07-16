import type { Editor } from "@tiptap/react";
import type { LeaseHandle } from "../../commands/barrier";

/** Shared reference to the currently-mounted TipTap editor instance.
 * Used by toolbar buttons / panels outside the Editor component
 * (like the comments gutter) to operate on the editor. */
let currentEditor: Editor | null = null;

export function setCurrentEditor(editor: Editor | null) {
  currentEditor = editor;
}

export function getCurrentEditor(): Editor | null {
  return currentEditor;
}

export function setCurrentEditorMarkdown(markdown: string): boolean {
  if (!currentEditor) return false;
  currentEditor.commands.setContent(markdown, { emitUpdate: false });
  return true;
}

/** Hook the Editor component publishes so external callers (e.g. the
 * beforeunload handler in App.tsx, and the barrier lifecycle's
 * pre-operation drain) can drain any pending debounced save. The barrier
 * lifecycle passes its lease so the drain's own dispatches are
 * owner-admitted through the gate (plan slice 3, round 7). Returns the
 * same promise the editor itself awaits internally — wait on it to ensure
 * on-disk state matches the editor buffer. */
let pendingFlush: ((lease?: LeaseHandle) => Promise<void>) | null = null;
export function setPendingFlush(fn: ((lease?: LeaseHandle) => Promise<void>) | null) {
  pendingFlush = fn;
}
export function flushPendingEditorSave(lease?: LeaseHandle): Promise<void> {
  return pendingFlush ? pendingFlush(lease) : Promise.resolve();
}

/** Registry of in-flight AI transform streams. The barrier lifecycle
 * cancels them at barrier entry (plan slice 3, round 4): a stream that
 * kept inserting deltas during or after a tree operation would mutate a
 * buffer the rebuild is about to replace. */
const activeAiStreamCancellers = new Set<() => void>();
export function registerAiStreamCanceller(cancel: () => void): () => void {
  activeAiStreamCancellers.add(cancel);
  return () => {
    activeAiStreamCancellers.delete(cancel);
  };
}
export function cancelActiveAiStreams(): void {
  for (const cancel of [...activeAiStreamCancellers]) {
    cancel();
  }
  activeAiStreamCancellers.clear();
}

/** Extract markdown from the TipTap editor via the tiptap-markdown extension. */
export function getEditorMarkdown(editor: Editor): string {
  // tiptap-markdown attaches `markdown.getMarkdown()` to editor.storage
  const storage = editor.storage as unknown as {
    markdown?: { getMarkdown: () => string };
  };
  return storage.markdown?.getMarkdown() ?? editor.getHTML();
}
