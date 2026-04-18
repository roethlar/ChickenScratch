import { useEditor, EditorContent, type Editor as TipTapEditor } from "@tiptap/react";
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
import { CommentMark } from "../comments/CommentMark";
import { FootnoteNode } from "./FootnoteNode";
import { setCurrentEditor, getEditorMarkdown } from "./editorRef";
import { Markdown } from "tiptap-markdown";
import * as docCmd from "../../commands/document";
import { toastError } from "../shared/Toast";

export function Editor() {
  const activeDoc = useProjectStore((s) => s.activeDoc);
  const saving = useProjectStore((s) => s.saving);
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const docIdRef = useRef<string | null>(null);
  const [dirty, setDirty] = useState(false);
  const [findOpen, setFindOpen] = useState(false);
  const [findReplace, setFindReplace] = useState(false);
  const editorRef = useRef<TipTapEditor | null>(null);

  const saveCurrent = useCallback(async () => {
    const editor = editorRef.current;
    const p = useProjectStore.getState().project;
    const d = useProjectStore.getState().activeDoc;
    if (!editor || !p || !d) return;
    useProjectStore.setState({ saving: true });
    try {
      // tiptap-markdown serializes in-process; no subprocess
      const markdown = getEditorMarkdown(editor);
      await docCmd.updateDocumentContent(p.path, d.id, markdown);
      useProjectStore.setState({
        activeDoc: { ...d, content: markdown },
      });
      setDirty(false);
    } catch (e) {
      toastError(`Save failed: ${e}`);
    } finally {
      useProjectStore.setState({ saving: false });
    }
  }, []);

  const debouncedSave = useCallback(() => {
    setDirty(true);
    if (saveTimer.current) clearTimeout(saveTimer.current);
    saveTimer.current = setTimeout(() => {
      saveCurrent();
    }, 2000);
  }, [saveCurrent]);

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
      CommentMark,
      FootnoteNode,
      Markdown.configure({
        html: true,            // allow inline HTML to pass through untouched
        tightLists: true,
        bulletListMarker: "-",
        linkify: false,
        breaks: false,
        transformPastedText: false,
        transformCopiedText: false,
      }),
    ],
    content: "",
    editorProps: {
      attributes: {
        class: "editor-content",
        spellcheck: "true",
      },
    },
    onUpdate: () => {
      debouncedSave();
    },
  });

  useEffect(() => { editorRef.current = editor; }, [editor]);

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

  // Load document content when active doc changes — markdown → HTML via pandoc.
  useEffect(() => {
    if (!editor) return;
    if (!activeDoc) {
      editor.commands.clearContent();
      docIdRef.current = null;
      return;
    }
    if (docIdRef.current !== activeDoc.id) {
      docIdRef.current = activeDoc.id;
      const md = activeDoc.content || "";
      // tiptap-markdown parses markdown directly when setContent is given markdown string
      editor.commands.setContent(md);
      setDirty(false);
    }
  }, [activeDoc?.id, editor]);

  // Search highlight: find and select first match when navigating from search
  const searchHighlight = useProjectStore((s) => s.searchHighlight);
  useEffect(() => {
    if (!editor || !searchHighlight || !activeDoc) return;
    setTimeout(() => {
      const text = editor.state.doc.textContent;
      const idx = text.toLowerCase().indexOf(searchHighlight.toLowerCase());
      if (idx >= 0) {
        let found = false;
        editor.state.doc.descendants((node, nodePos) => {
          if (found || !node.isText || !node.text) return;
          const nodeIdx = node.text.toLowerCase().indexOf(searchHighlight.toLowerCase());
          if (nodeIdx >= 0) {
            const from = nodePos + nodeIdx;
            const to = from + searchHighlight.length;
            editor.chain().focus().setTextSelection({ from, to }).run();
            found = true;
          }
        });
      }
      useProjectStore.setState({ searchHighlight: null });
    }, 100);
  }, [searchHighlight, activeDoc?.id, editor]);

  useEffect(() => {
    return () => {
      if (saveTimer.current) clearTimeout(saveTimer.current);
    };
  }, []);

  useEffect(() => {
    setCurrentEditor(editor);
    return () => setCurrentEditor(null);
  }, [editor]);

  const project = useProjectStore((s) => s.project);
  const sessionStartWords = useProjectStore((s) => s.sessionStartWords);

  if (!activeDoc) {
    return (
      <div className="editor-empty">
        <p>Select a document to start writing</p>
      </div>
    );
  }

  const words = editor?.storage.characterCount.words() ?? 0;
  const saveLabel = saving ? "Saving..." : dirty ? "Modified" : "Saved";
  const totalProjectWords = project
    ? Object.values(project.documents).reduce((sum, doc) => {
        return sum + (doc.content || "").split(/\s+/).filter(Boolean).length;
      }, 0)
    : 0;
  const sessionWords = Math.max(0, totalProjectWords - sessionStartWords);

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
        <span>
          {words.toLocaleString()} words
          {sessionWords > 0 && ` · +${sessionWords.toLocaleString()} this session`}
        </span>
        <span>{saveLabel}</span>
      </div>
    </div>
  );
}
