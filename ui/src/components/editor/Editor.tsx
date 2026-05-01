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
import * as sessionCmd from "../../commands/session";
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
    // Only want to reload content on id change; content changes flow through the editor.
    // eslint-disable-next-line react-hooks/exhaustive-deps
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
    // eslint-disable-next-line react-hooks/exhaustive-deps
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
        <SessionBadge />
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

function SessionBadge() {
  const project = useProjectStore((s) => s.project);
  const [progress, setProgress] = useState<sessionCmd.SessionProgress | null>(null);
  const [hidden, setHidden] = useState(false);
  const hideTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (!project) return;
    let cancelled = false;
    const refresh = () => {
      sessionCmd
        .getSessionProgress(project.path)
        .then((p) => { if (!cancelled) setProgress(p); })
        .catch(() => { if (!cancelled) setProgress(null); });
    };
    refresh();
    const interval = setInterval(refresh, 30_000);
    return () => { cancelled = true; clearInterval(interval); };
  }, [project]);

  // Auto-hide on idle, reappear on any keypress in the editor
  useEffect(() => {
    const reveal = () => {
      setHidden(false);
      if (hideTimer.current) clearTimeout(hideTimer.current);
      hideTimer.current = setTimeout(() => setHidden(true), 4000);
    };
    reveal();
    window.addEventListener("keydown", reveal);
    return () => {
      window.removeEventListener("keydown", reveal);
      if (hideTimer.current) clearTimeout(hideTimer.current);
    };
  }, [project?.path]);

  if (!progress) return null;
  const hasTarget =
    progress.words_per_session != null ||
    progress.total_target != null ||
    progress.deadline != null;
  if (!hasTarget) return null;

  const goal = progress.words_per_session ?? 0;
  const pct = goal > 0 ? Math.min(100, Math.round((progress.today_words / goal) * 100)) : 0;
  const reached = goal > 0 && progress.today_words >= goal;

  const parts: string[] = [];
  if (goal > 0) parts.push(`Today ${progress.today_words.toLocaleString()}/${goal.toLocaleString()}`);
  if (progress.days_remaining != null) {
    if (progress.days_remaining > 0) parts.push(`${progress.days_remaining}d left`);
    else if (progress.days_remaining === 0) parts.push("deadline today");
    else parts.push("deadline passed");
  }
  if (progress.needed_per_day != null) {
    parts.push(`${progress.needed_per_day.toLocaleString()}/day needed`);
  }

  return (
    <div className={`session-badge ${hidden ? "hidden" : ""} ${reached ? "reached" : ""}`}>
      <div className="session-badge-text">{parts.join(" · ")}</div>
      {goal > 0 && (
        <div className="session-badge-bar">
          <div className="session-badge-fill" style={{ width: `${pct}%` }} />
        </div>
      )}
    </div>
  );
}
