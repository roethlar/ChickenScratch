import { useEditor, EditorContent } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import Placeholder from "@tiptap/extension-placeholder";
import CharacterCount from "@tiptap/extension-character-count";
import { Markdown } from "tiptap-markdown";
import { useEffect, useRef, useCallback } from "react";
import { useProjectStore } from "../../stores/projectStore";

export function Editor() {
  const activeDoc = useProjectStore((s) => s.activeDoc);
  const updateContent = useProjectStore((s) => s.updateContent);
  const saveActiveDoc = useProjectStore((s) => s.saveActiveDoc);
  const saving = useProjectStore((s) => s.saving);
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

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
      Markdown,
    ],
    content: "",
    editorProps: {
      attributes: {
        class: "editor-content",
      },
    },
    onUpdate: ({ editor }) => {
      const md = editor.storage.markdown.getMarkdown();
      updateContent(md);
      debouncedSave();
    },
  });

  // Load document content when active doc changes
  useEffect(() => {
    if (!editor) return;
    if (!activeDoc) {
      editor.commands.clearContent();
      return;
    }
    // Only set content if it's a different document
    const currentMd = editor.storage.markdown.getMarkdown();
    if (currentMd !== activeDoc.content) {
      editor.commands.setContent(activeDoc.content || "");
    }
  }, [activeDoc?.id, editor]);

  // Cleanup save timer
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
