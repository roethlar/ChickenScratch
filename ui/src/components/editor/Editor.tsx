import { useEditor, EditorContent } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import Placeholder from "@tiptap/extension-placeholder";
import CharacterCount from "@tiptap/extension-character-count";
import { Underline } from "@tiptap/extension-underline";
import { TextStyle } from "@tiptap/extension-text-style";
import { Color } from "@tiptap/extension-color";
import { Link } from "@tiptap/extension-link";
import { useState, useEffect, useRef, useCallback } from "react";
import { useProjectStore } from "../../stores/projectStore";
import { Toolbar } from "./Toolbar";
import { FindReplace } from "./FindReplace";

export function Editor() {
  const activeDoc = useProjectStore((s) => s.activeDoc);
  const updateContent = useProjectStore((s) => s.updateContent);
  const saveActiveDoc = useProjectStore((s) => s.saveActiveDoc);
  const saving = useProjectStore((s) => s.saving);
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const docIdRef = useRef<string | null>(null);
  const [dirty, setDirty] = useState(false);
  const [findOpen, setFindOpen] = useState(false);
  const [findReplace, setFindReplace] = useState(false);

  const debouncedSave = useCallback(() => {
    setDirty(true);
    if (saveTimer.current) clearTimeout(saveTimer.current);
    saveTimer.current = setTimeout(() => {
      saveActiveDoc();
      setDirty(false);
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
      Link.configure({
        openOnClick: false,
        HTMLAttributes: {
          rel: "noopener noreferrer nofollow",
        },
      }),
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

  // Ctrl+F / Ctrl+H shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      if (mod && e.key === "f" && !e.shiftKey) {
        e.preventDefault();
        setFindOpen(true);
        setFindReplace(false);
      }
      if (mod && e.key === "h") {
        e.preventDefault();
        setFindOpen(true);
        setFindReplace(true);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  // Load document content when active doc changes
  useEffect(() => {
    if (!editor) return;
    if (!activeDoc) {
      editor.commands.clearContent();
      docIdRef.current = null;
      return;
    }
    if (docIdRef.current !== activeDoc.id) {
      docIdRef.current = activeDoc.id;
      editor.commands.setContent(activeDoc.content || "");
      setDirty(false);
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
  const chars = editor?.storage.characterCount.characters() ?? 0;
  const saveLabel = saving ? "Saving..." : dirty ? "Modified" : "Saved";

  return (
    <div className="editor-pane">
      <Toolbar editor={editor} />
      <FindReplace
        editor={editor}
        open={findOpen}
        showReplace={findReplace}
        onClose={() => setFindOpen(false)}
      />
      <div className="editor-scroll">
        <EditorContent editor={editor} />
      </div>
      <div className="editor-status">
        <span>{words.toLocaleString()} words &middot; {chars.toLocaleString()} chars</span>
        <span>{saveLabel}</span>
      </div>
    </div>
  );
}
