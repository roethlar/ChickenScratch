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
