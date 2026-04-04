import { useEditor, EditorContent } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import Placeholder from "@tiptap/extension-placeholder";
import CharacterCount from "@tiptap/extension-character-count";
import { Underline } from "@tiptap/extension-underline";
import { TextStyle } from "@tiptap/extension-text-style";
import { Color } from "@tiptap/extension-color";
import { useEffect, useRef, useCallback } from "react";
import { useProjectStore } from "../../stores/projectStore";

export function Editor() {
  const activeDoc = useProjectStore((s) => s.activeDoc);
  const updateContent = useProjectStore((s) => s.updateContent);
  const saveActiveDoc = useProjectStore((s) => s.saveActiveDoc);
  const saving = useProjectStore((s) => s.saving);
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const docIdRef = useRef<string | null>(null);

  const debouncedSave = useCallback(() => {
    if (saveTimer.current) clearTimeout(saveTimer.current);
    saveTimer.current = setTimeout(() => {
      saveActiveDoc();
    }, 2000);
  }, [saveActiveDoc]);

  const editor = useEditor({
    extensions: [
      StarterKit.configure({
        heading: { levels: [1, 2, 3, 4] },
      }),
      Placeholder.configure({
        placeholder: "Start writing...",
      }),
      CharacterCount,
      Underline,
      TextStyle,
      Color,
    ],
    content: "",
    editorProps: {
      attributes: {
        class: "editor-content",
      },
    },
    onUpdate: ({ editor }) => {
      const html = editor.getHTML();
      updateContent(html);
      debouncedSave();
    },
  });

  // Load document content when active doc changes
  useEffect(() => {
    if (!editor) return;
    if (!activeDoc) {
      editor.commands.clearContent();
      docIdRef.current = null;
      return;
    }
    // Only set content when switching documents
    if (docIdRef.current !== activeDoc.id) {
      docIdRef.current = activeDoc.id;
      editor.commands.setContent(activeDoc.content || "");
    }
  }, [activeDoc?.id, editor]);

  useEffect(() => {
    return () => {
      if (saveTimer.current) clearTimeout(saveTimer.current);
    };
  }, []);

  if (!activeDoc) {
    return (
      <div className="editor-empty">
        <p>Select a document to start writing</p>
      </div>
    );
  }

  const words = editor?.storage.characterCount.words() ?? 0;

  return (
    <div className="editor-pane">
      <div className="editor-scroll">
        <EditorContent editor={editor} />
      </div>
      <div className="editor-status">
        <span>{words.toLocaleString()} words</span>
        <span>{saving ? "Saving..." : "Saved"}</span>
      </div>
    </div>
  );
}
