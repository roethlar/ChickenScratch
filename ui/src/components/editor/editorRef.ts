import type { Editor } from "@tiptap/react";

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

/** Hook the Editor component publishes so external callers (e.g. the
 * beforeunload handler in App.tsx) can drain any pending debounced save
 * before the window goes away. Returns the same promise the editor
 * itself awaits internally — wait on it to ensure on-disk state matches
 * the editor buffer. */
let pendingFlush: (() => Promise<void>) | null = null;
export function setPendingFlush(fn: (() => Promise<void>) | null) {
  pendingFlush = fn;
}
export function flushPendingEditorSave(): Promise<void> {
  return pendingFlush ? pendingFlush() : Promise.resolve();
}

/** Extract markdown from the TipTap editor via the tiptap-markdown extension. */
export function getEditorMarkdown(editor: Editor): string {
  // tiptap-markdown attaches `markdown.getMarkdown()` to editor.storage
  const storage = editor.storage as unknown as {
    markdown?: { getMarkdown: () => string };
  };
  return storage.markdown?.getMarkdown() ?? editor.getHTML();
}
